//! Historial, estado y configuración en SQLite.

use drivward_core::model::{Device, InstalledDriver};
use drivward_ipc::{BlockView, Origin, Settings};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

pub use rusqlite::Error;

const SCHEMA: &str = r#"
PRAGMA journal_mode = WAL;
CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS devices (
    instance_id TEXT PRIMARY KEY, name TEXT, class TEXT, hardware_ids TEXT NOT NULL,
    provider TEXT, version TEXT, date TEXT, inf_name TEXT, problem_code INTEGER NOT NULL, seen_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS changes (
    id INTEGER PRIMARY KEY, at TEXT NOT NULL, instance_id TEXT NOT NULL, device_name TEXT NOT NULL,
    class TEXT, old_version TEXT, new_version TEXT, old_inf TEXT, new_inf TEXT,
    origin TEXT NOT NULL, validated INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY, at TEXT NOT NULL, event_id INTEGER NOT NULL, provider TEXT NOT NULL, message TEXT NOT NULL,
    UNIQUE (at, event_id, provider)
);
CREATE TABLE IF NOT EXISTS known_good (
    instance_id TEXT NOT NULL, version TEXT NOT NULL, inf_name TEXT, backup_path TEXT, recorded_at TEXT NOT NULL,
    PRIMARY KEY (instance_id, version)
);
CREATE TABLE IF NOT EXISTS blocks (
    update_id TEXT PRIMARY KEY, title TEXT NOT NULL, hardware_id TEXT, created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS pending (instance_id TEXT PRIMARY KEY, created_at TEXT NOT NULL);
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeRow {
    pub id: i64,
    pub at: String,
    pub instance_id: String,
    pub device_name: String,
    pub class: Option<String>,
    pub old_version: Option<String>,
    pub new_version: Option<String>,
    pub old_inf: Option<String>,
    pub new_inf: Option<String>,
    pub origin: Origin,
    pub validated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventRow {
    pub at: String,
    pub event_id: u32,
    pub provider: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownGood {
    pub version: String,
    pub inf_name: Option<String>,
    pub backup_path: Option<String>,
}

pub struct Db {
    connection: Connection,
}

fn origin_text(origin: Origin) -> &'static str {
    match origin {
        Origin::Drivward => "drivward",
        Origin::External => "external",
        Origin::Rollback => "rollback",
    }
}

fn origin_from(text: &str) -> Origin {
    match text {
        "drivward" => Origin::Drivward,
        "rollback" => Origin::Rollback,
        _ => Origin::External,
    }
}

impl Db {
    pub fn open(path: &Path) -> Result<Self, Error> {
        Self::init(Connection::open(path)?)
    }

    pub fn in_memory() -> Result<Self, Error> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(connection: Connection) -> Result<Self, Error> {
        connection.execute_batch(SCHEMA)?;
        Ok(Self { connection })
    }

    pub fn settings(&self) -> Settings {
        self.get("settings")
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), Error> {
        let text = serde_json::to_string(settings).expect("la configuración siempre se serializa");
        self.set("settings", &text)
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.connection
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()
            .ok()
            .flatten()
    }

    pub fn set(&self, key: &str, value: &str) -> Result<(), Error> {
        self.connection.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn devices(&self) -> Result<Vec<Device>, Error> {
        let mut statement = self.connection.prepare(
            "SELECT instance_id, name, class, hardware_ids, provider, version, date, inf_name, problem_code FROM devices",
        )?;
        let rows = statement.query_map([], |row| {
            let hardware_ids: String = row.get(3)?;
            let version: Option<String> = row.get(5)?;
            let inf_name: Option<String> = row.get(7)?;
            let driver = (version.is_some() || inf_name.is_some()).then(|| InstalledDriver {
                provider: row.get(4).ok().flatten(),
                version: version.clone(),
                date: row.get(6).ok().flatten(),
                inf_name: inf_name.clone(),
                signed: true,
                signer: None,
            });
            Ok(Device {
                instance_id: row.get(0)?,
                name: row.get(1)?,
                class: row.get(2)?,
                hardware_ids: serde_json::from_str(&hardware_ids).unwrap_or_default(),
                problem_code: row.get(8)?,
                driver,
                ..Default::default()
            })
        })?;
        rows.collect()
    }

    pub fn save_devices(&mut self, devices: &[Device], now: &str) -> Result<(), Error> {
        let transaction = self.connection.transaction()?;
        transaction.execute("DELETE FROM devices", [])?;
        for device in devices {
            let driver = device.driver.as_ref();
            transaction.execute(
                "INSERT INTO devices VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    device.instance_id,
                    device.name,
                    device.class,
                    serde_json::to_string(&device.hardware_ids).unwrap_or_default(),
                    driver.and_then(|d| d.provider.clone()),
                    driver.and_then(|d| d.version.clone()),
                    driver.and_then(|d| d.date.clone()),
                    driver.and_then(|d| d.inf_name.clone()),
                    device.problem_code,
                    now,
                ],
            )?;
        }
        transaction.commit()
    }

    pub fn insert_change(&self, change: &ChangeRow) -> Result<i64, Error> {
        self.connection.execute(
            "INSERT INTO changes (at, instance_id, device_name, class, old_version, new_version, old_inf, new_inf, origin, validated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                change.at,
                change.instance_id,
                change.device_name,
                change.class,
                change.old_version,
                change.new_version,
                change.old_inf,
                change.new_inf,
                origin_text(change.origin),
                change.validated,
            ],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn changes(&self, limit: u32) -> Result<Vec<ChangeRow>, Error> {
        let mut statement = self.connection.prepare(
            "SELECT id, at, instance_id, device_name, class, old_version, new_version, old_inf, new_inf, origin, validated
             FROM changes ORDER BY at DESC, id DESC LIMIT ?1",
        )?;
        let rows = statement.query_map([limit], |row| {
            let origin: String = row.get(9)?;
            Ok(ChangeRow {
                id: row.get(0)?,
                at: row.get(1)?,
                instance_id: row.get(2)?,
                device_name: row.get(3)?,
                class: row.get(4)?,
                old_version: row.get(5)?,
                new_version: row.get(6)?,
                old_inf: row.get(7)?,
                new_inf: row.get(8)?,
                origin: origin_from(&origin),
                validated: row.get(10)?,
            })
        })?;
        rows.collect()
    }

    pub fn change(&self, id: i64) -> Result<Option<ChangeRow>, Error> {
        Ok(self.changes(u32::MAX)?.into_iter().find(|c| c.id == id))
    }

    pub fn mark_validated(&self, id: i64) -> Result<(), Error> {
        self.connection
            .execute("UPDATE changes SET validated = 1 WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn insert_events(&self, events: &[EventRow]) -> Result<usize, Error> {
        let mut inserted = 0;
        for event in events {
            inserted += self.connection.execute(
                "INSERT OR IGNORE INTO events (at, event_id, provider, message) VALUES (?1, ?2, ?3, ?4)",
                params![event.at, event.event_id, event.provider, event.message],
            )?;
        }
        Ok(inserted)
    }

    pub fn events_since(&self, since: &str) -> Result<Vec<EventRow>, Error> {
        let mut statement = self.connection.prepare(
            "SELECT at, event_id, provider, message FROM events WHERE at >= ?1 ORDER BY at",
        )?;
        let rows = statement.query_map([since], |row| {
            Ok(EventRow {
                at: row.get(0)?,
                event_id: row.get(1)?,
                provider: row.get(2)?,
                message: row.get(3)?,
            })
        })?;
        rows.collect()
    }

    pub fn last_event_time(&self) -> Option<String> {
        self.connection
            .query_row("SELECT MAX(at) FROM events", [], |row| row.get(0))
            .ok()
            .flatten()
    }

    pub fn save_known_good(
        &self,
        instance_id: &str,
        good: &KnownGood,
        now: &str,
    ) -> Result<(), Error> {
        self.connection.execute(
            "INSERT OR REPLACE INTO known_good VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                instance_id,
                good.version,
                good.inf_name,
                good.backup_path,
                now
            ],
        )?;
        Ok(())
    }

    pub fn known_good(&self, instance_id: &str, version: &str) -> Result<Option<KnownGood>, Error> {
        self.connection
            .query_row(
                "SELECT version, inf_name, backup_path FROM known_good WHERE instance_id = ?1 AND version = ?2",
                params![instance_id, version],
                |row| Ok(KnownGood { version: row.get(0)?, inf_name: row.get(1)?, backup_path: row.get(2)? }),
            )
            .optional()
    }

    pub fn add_block(&self, block: &BlockView) -> Result<(), Error> {
        self.connection.execute(
            "INSERT OR REPLACE INTO blocks VALUES (?1, ?2, ?3, ?4)",
            params![
                block.update_id,
                block.title,
                block.hardware_id,
                block.created_at
            ],
        )?;
        Ok(())
    }

    pub fn remove_block(&self, update_id: &str) -> Result<(), Error> {
        self.connection
            .execute("DELETE FROM blocks WHERE update_id = ?1", [update_id])?;
        Ok(())
    }

    pub fn blocks(&self) -> Result<Vec<BlockView>, Error> {
        let mut statement = self.connection.prepare(
            "SELECT update_id, title, hardware_id, created_at FROM blocks ORDER BY created_at DESC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(BlockView {
                update_id: row.get(0)?,
                title: row.get(1)?,
                hardware_id: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        rows.collect()
    }

    /// Dispositivos que Drivward está cambiando, para no confundirlos con cambios externos.
    pub fn add_pending(&self, instance_id: &str, now: &str) -> Result<(), Error> {
        self.connection.execute(
            "INSERT OR REPLACE INTO pending VALUES (?1, ?2)",
            params![instance_id, now],
        )?;
        Ok(())
    }

    pub fn take_pending(&self, instance_id: &str) -> Result<bool, Error> {
        Ok(self
            .connection
            .execute("DELETE FROM pending WHERE instance_id = ?1", [instance_id])?
            > 0)
    }
}

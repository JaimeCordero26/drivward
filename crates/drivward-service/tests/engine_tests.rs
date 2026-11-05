use drivward_core::model::{Category, Device, InstalledDriver, SystemInfo};
use drivward_core::select::Signature;
use drivward_ipc::{Confidence, Mode, Origin, Request, Response, Settings};
use drivward_platform::{
    CommandOutput, InstallResult, Platform, Result, SystemEvent, WindowsUpdateDriver,
};
use drivward_service::db::{Db, EventRow};
use drivward_service::engine::Engine;
use drivward_service::{monitor, policy, stability};
use std::path::Path;
use std::sync::{Arc, Mutex};
use time::macros::datetime;
use time::{Duration, OffsetDateTime};

#[derive(Default)]
struct State {
    devices: Vec<Device>,
    events: Vec<SystemEvent>,
    updates: Vec<WindowsUpdateDriver>,
    removed: Vec<String>,
    installed_updates: Vec<String>,
    hidden: Vec<String>,
}

#[derive(Clone, Default)]
struct Fake(Arc<Mutex<State>>);

fn gpu(version: &str, inf: &str) -> Device {
    Device {
        instance_id: "PCI\\VEN_10DE&DEV_2504\\4&1".into(),
        name: Some("Tarjeta gráfica".into()),
        class: Some("Display".into()),
        hardware_ids: vec!["PCI\\VEN_10DE&DEV_2504".into()],
        driver: Some(InstalledDriver {
            version: Some(version.into()),
            inf_name: Some(inf.into()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

impl Platform for Fake {
    fn system_info(&self) -> Result<SystemInfo> {
        Ok(SystemInfo::default())
    }
    fn devices(&self) -> Result<Vec<Device>> {
        Ok(self.0.lock().unwrap().devices.clone())
    }
    fn verify_signature(&self, _: &Path) -> Result<Signature> {
        Ok(Signature::Valid)
    }
    fn install_inf(&self, _: &Path) -> Result<InstallResult> {
        Ok(InstallResult {
            reboot_required: false,
            not_applied: false,
            output: CommandOutput {
                code: 0,
                output: String::new(),
            },
        })
    }
    fn backup_drivers(&self, _: &Path) -> Result<CommandOutput> {
        Ok(CommandOutput {
            code: 0,
            output: String::new(),
        })
    }
    fn backup_driver(&self, _: &str, _: &Path) -> Result<CommandOutput> {
        Ok(CommandOutput {
            code: 0,
            output: String::new(),
        })
    }
    fn low_battery(&self) -> Result<bool> {
        Ok(false)
    }
    fn create_restore_point(&self, _: &str) -> Result<bool> {
        Ok(true)
    }
    fn open_wifi_settings(&self) -> Result<()> {
        Ok(())
    }
    fn open_network_settings(&self) -> Result<()> {
        Ok(())
    }
    fn windows_update_drivers(&self, _: bool) -> Result<Vec<WindowsUpdateDriver>> {
        Ok(self.0.lock().unwrap().updates.clone())
    }
    fn install_windows_updates(&self, ids: &[String]) -> Result<InstallResult> {
        let mut state = self.0.lock().unwrap();
        state.installed_updates.extend(ids.iter().cloned());
        state.devices = vec![gpu("32.0.15.7000", "oem20.inf")];
        Ok(InstallResult {
            reboot_required: false,
            not_applied: false,
            output: CommandOutput {
                code: 2,
                output: String::new(),
            },
        })
    }
    fn set_update_hidden(&self, id: &str, hidden: bool) -> Result<()> {
        let mut state = self.0.lock().unwrap();
        if hidden {
            state.hidden.push(id.into());
        } else {
            state.hidden.retain(|h| h != id);
        }
        Ok(())
    }
    fn remove_driver(&self, inf: &str) -> Result<CommandOutput> {
        let mut state = self.0.lock().unwrap();
        state.removed.push(inf.into());
        // Windows vuelve al driver anterior que sigue en el Driver Store.
        state.devices = vec![gpu("31.0.15.5000", "oem12.inf")];
        Ok(CommandOutput {
            code: 0,
            output: String::new(),
        })
    }
    fn events_since(&self, _: &str) -> Result<Vec<SystemEvent>> {
        Ok(self.0.lock().unwrap().events.clone())
    }
    fn reboot_pending(&self) -> Result<bool> {
        Ok(false)
    }
    fn schedule_resume(&self, _: &Path, _: &str) -> Result<()> {
        Ok(())
    }
}

fn engine(fake: &Fake) -> (Engine, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::new(
        Box::new(fake.clone()),
        Db::in_memory().unwrap(),
        dir.path().to_path_buf(),
    );
    (engine, dir)
}

fn tdr(at: OffsetDateTime) -> SystemEvent {
    SystemEvent {
        time: at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap(),
        id: 4101,
        provider: "Display".into(),
        message: "TDR".into(),
    }
}

#[test]
fn records_external_change_and_rolls_back() {
    let fake = Fake::default();
    fake.0.lock().unwrap().devices = vec![gpu("31.0.15.5000", "oem12.inf")];
    let (mut engine, _dir) = engine(&fake);
    let start = datetime!(2026-10-01 10:00 UTC);
    engine.tick(start, start);

    // Windows Update cambia el driver de video y después aparecen fallos.
    fake.0.lock().unwrap().devices = vec![gpu("32.0.15.6000", "oem19.inf")];
    let notices = engine.tick(start + Duration::hours(1), start);
    assert_eq!(notices.len(), 1);
    assert!(notices[0].title.contains("Tarjeta gráfica"));

    let later = start + Duration::days(2);
    fake.0.lock().unwrap().events = (1..=4)
        .map(|h| tdr(start + Duration::hours(h * 5)))
        .collect();
    engine.tick(later, later);

    let Response::History(history) = engine.handle(Request::History { limit: 10 }, later) else {
        panic!()
    };
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].origin, Origin::External);
    assert!(history[0].can_rollback);

    let Response::Evidence(evidence) = engine.handle(Request::Evidence, later) else {
        panic!()
    };
    assert_eq!(evidence[0].confidence, Confidence::High);
    assert!(evidence[0].suggest_rollback);

    let Response::Status(status) = engine.handle(Request::Status, later) else {
        panic!()
    };
    assert_eq!(status.health, drivward_ipc::Health::Unstable);

    let response = engine.handle(
        Request::Rollback {
            change_id: history[0].id,
        },
        later,
    );
    assert!(matches!(response, Response::Done { .. }), "{response:?}");
    assert_eq!(fake.0.lock().unwrap().removed, ["oem19.inf"]);

    let Response::History(history) = engine.handle(Request::History { limit: 10 }, later) else {
        panic!()
    };
    assert_eq!(
        history[0].origin,
        Origin::Drivward,
        "el rollback no se confunde con un cambio externo"
    );
}

#[test]
fn observe_mode_never_installs_and_quarantine_is_respected() {
    let fake = Fake::default();
    fake.0.lock().unwrap().devices = vec![gpu("31.0.15.5000", "oem12.inf")];
    fake.0.lock().unwrap().updates = vec![WindowsUpdateDriver {
        update_id: "u1".into(),
        title: "NVIDIA - Display - 32.0.15.7000".into(),
        provider: Some("NVIDIA".into()),
        hardware_id: Some("pci\\ven_10de&dev_2504".into()),
        model: None,
        class: Some("Display".into()),
        date: Some("2026-10-28".into()),
        hidden: false,
    }];
    let (mut engine, _dir) = engine(&fake);
    let now = datetime!(2026-11-01 13:00 UTC);
    engine.handle(Request::ScanNow, now);

    let Response::Recommendations(list) = engine.handle(Request::Recommendations, now) else {
        panic!()
    };
    assert_eq!(list.len(), 1);
    assert!(list[0].requires_approval);
    assert_eq!(list[0].available_after.as_deref(), Some("2026-11-04"));

    let response = engine.handle(
        Request::Install {
            recommendation_id: "u1".into(),
        },
        now,
    );
    assert!(matches!(response, Response::Error { .. }));

    let settings = Settings {
        mode: Mode::Recommend,
        ..Default::default()
    };
    engine.handle(Request::SetSettings { settings }, now);
    assert!(matches!(
        engine.handle(
            Request::Install {
                recommendation_id: "u1".into()
            },
            now
        ),
        Response::Error { .. }
    ));

    let later = datetime!(2026-11-05 13:00 UTC);
    let response = engine.handle(
        Request::Install {
            recommendation_id: "u1".into(),
        },
        later,
    );
    assert!(matches!(response, Response::Done { .. }), "{response:?}");
    assert_eq!(fake.0.lock().unwrap().installed_updates, ["u1"]);
}

#[test]
fn automatic_mode_skips_critical_devices() {
    let fake = Fake::default();
    fake.0.lock().unwrap().devices = vec![gpu("31.0.15.5000", "oem12.inf")];
    fake.0.lock().unwrap().updates = vec![WindowsUpdateDriver {
        update_id: "u1".into(),
        title: "Display".into(),
        provider: None,
        hardware_id: Some("PCI\\VEN_10DE&DEV_2504".into()),
        model: None,
        class: None,
        date: Some("2026-01-01".into()),
        hidden: false,
    }];
    let (mut engine, _dir) = engine(&fake);
    let now = datetime!(2026-11-01 13:00 UTC);
    engine.handle(
        Request::SetSettings {
            settings: Settings {
                mode: Mode::Automatic,
                check_hour: 0,
                ..Default::default()
            },
        },
        now,
    );
    engine.tick(now, now);
    assert!(fake.0.lock().unwrap().installed_updates.is_empty());
}

#[test]
fn blocks_and_unblocks_updates() {
    let fake = Fake::default();
    let (mut engine, _dir) = engine(&fake);
    let now = datetime!(2026-11-01 13:00 UTC);
    assert!(matches!(
        engine.handle(
            Request::Block {
                update_id: "u9".into()
            },
            now
        ),
        Response::Done { .. }
    ));
    let Response::Blocks(blocks) = engine.handle(Request::Blocks, now) else {
        panic!()
    };
    assert_eq!(blocks.len(), 1);
    assert_eq!(fake.0.lock().unwrap().hidden, ["u9"]);
    engine.handle(
        Request::Unblock {
            update_id: "u9".into(),
        },
        now,
    );
    assert!(fake.0.lock().unwrap().hidden.is_empty());
}

#[test]
fn pause_stops_monitoring() {
    let fake = Fake::default();
    fake.0.lock().unwrap().devices = vec![gpu("1.0", "oem1.inf")];
    let (mut engine, _dir) = engine(&fake);
    let now = datetime!(2026-11-01 13:00 UTC);
    engine.handle(Request::Pause { hours: 24 }, now);
    engine.tick(now, now);
    let Response::Devices(devices) = engine.handle(Request::Devices, now) else {
        panic!()
    };
    assert!(devices.is_empty());
    assert!(policy::is_paused(&engine.settings(), now));
    assert!(!policy::is_paused(
        &engine.settings(),
        now + Duration::days(2)
    ));
}

#[test]
fn coincidence_alone_is_low_confidence() {
    let change = datetime!(2026-10-10 00:00 UTC);
    let events: Vec<EventRow> = [
        (change - Duration::days(20), 41),
        (change - Duration::days(10), 41),
        (change - Duration::days(5), 41),
        (change + Duration::days(1), 41),
    ]
    .iter()
    .map(|(at, id)| EventRow {
        at: at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap(),
        event_id: *id,
        provider: "Kernel-Power".into(),
        message: String::new(),
    })
    .collect();
    let evidence = stability::evaluate(
        "2026-10-10T00:00:00Z",
        Category::Network,
        &events,
        change + Duration::days(7),
        7,
    );
    assert!(evidence.confidence <= Confidence::Low);
    assert!(evidence.explanation.contains("no prueba"));
}

#[test]
fn diff_ignores_new_devices() {
    let before = vec![gpu("1.0", "oem1.inf")];
    let mut usb = gpu("1.0", "oem2.inf");
    usb.instance_id = "USB\\X".into();
    let after = vec![gpu("1.0", "oem1.inf"), usb];
    assert!(monitor::diff(&before, &after).is_empty());
    let changed = monitor::diff(&before, &[gpu("2.0", "oem3.inf")]);
    assert_eq!(changed[0].old_version.as_deref(), Some("1.0"));
}

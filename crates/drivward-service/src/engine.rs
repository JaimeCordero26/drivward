//! Lógica del servicio de escritorio. No depende de Windows para poder probarse.

use crate::db::{ChangeRow, Db, EventRow, KnownGood};
use crate::monitor;
use crate::policy::{self, Conditions};
use crate::stability::{self, BASELINE_DAYS};
use drivward_core::classify::{classify, DeviceState};
use drivward_core::journal::{self, ActionKind, ActionRecord, Journal, Outcome};
use drivward_core::model::{Category, Device};
use drivward_core::select::{normalize, Selection};
use drivward_ipc::{
    BlockView, ChangeView, Confidence, DeviceView, EvidenceView, Health, Mode, Origin,
    RecommendationView, Request, Response, Settings, Status,
};
use drivward_platform::{Platform, WindowsUpdateDriver};
use std::path::{Path, PathBuf};
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

const LAST_SCAN: &str = "last_scan";
const LAST_CHECK: &str = "last_update_check";

pub struct Engine {
    platform: Box<dyn Platform>,
    db: Db,
    data_dir: PathBuf,
    journal: Option<Journal>,
    updates: Vec<WindowsUpdateDriver>,
}

/// Algo que la app de bandeja debe notificar (DESK-19, DESK-29).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub title: String,
    pub body: String,
}

fn format(time: OffsetDateTime) -> String {
    time.format(&Rfc3339).unwrap_or_default()
}

fn parse(text: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(text, &Rfc3339).ok()
}

/// Nombre de carpeta seguro a partir de un Instance ID.
fn folder_name(instance_id: &str) -> String {
    instance_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .take(120)
        .collect()
}

fn find_inf(folder: &Path) -> Option<PathBuf> {
    let mut stack = vec![folder.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("inf"))
            {
                return Some(path);
            }
        }
    }
    None
}

impl Engine {
    pub fn new(platform: Box<dyn Platform>, db: Db, data_dir: PathBuf) -> Self {
        let journal = Journal::open(data_dir.join("logs").join("acciones.jsonl")).ok();
        Self {
            platform,
            db,
            data_dir,
            journal,
            updates: Vec::new(),
        }
    }

    pub fn settings(&self) -> Settings {
        self.db.settings()
    }

    fn log(&self, record: &ActionRecord) {
        if let Some(journal) = &self.journal {
            let _ = journal.append(record);
        }
    }

    /// Revisión periódica: registra cambios y fallos, valida cambios y aplica el modo elegido.
    pub fn tick(&mut self, now: OffsetDateTime, local_now: OffsetDateTime) -> Vec<Notice> {
        let settings = self.settings();
        if policy::is_paused(&settings, now) {
            return Vec::new();
        }
        let mut notices = self.scan(now);
        if policy::check_due(&settings, self.db.get(LAST_CHECK).as_deref(), local_now) {
            self.refresh_updates(now);
        }
        notices.extend(self.validate_changes(now, &settings));
        if settings.mode == Mode::Automatic {
            notices.extend(self.apply_automatic(now));
        }
        notices
    }

    fn scan(&mut self, now: OffsetDateTime) -> Vec<Notice> {
        let stamp = format(now);
        let Ok(current) = self.platform.devices() else {
            return Vec::new();
        };
        let previous = self.db.devices().unwrap_or_default();
        let first_run = previous.is_empty();
        let mut notices = Vec::new();
        for change in monitor::diff(&previous, &current) {
            let origin = match self.db.take_pending(&change.instance_id) {
                Ok(true) => Origin::Drivward,
                _ => Origin::External,
            };
            let row = ChangeRow {
                id: 0,
                at: stamp.clone(),
                instance_id: change.instance_id.clone(),
                device_name: change.device_name.clone(),
                class: change.class.clone(),
                old_version: change.old_version.clone(),
                new_version: change.new_version.clone(),
                old_inf: change.old_inf.clone(),
                new_inf: change.new_inf.clone(),
                origin,
                validated: false,
            };
            let _ = self.db.insert_change(&row);
            if origin == Origin::External {
                notices.push(Notice {
                    title: format!("Cambió el driver de {}", change.device_name),
                    body: format!(
                        "{} → {}. Drivward vigilará la estabilidad y podrás volver a la versión anterior desde el historial.",
                        change.old_version.as_deref().unwrap_or("sin versión"),
                        change.new_version.as_deref().unwrap_or("sin driver"),
                    ),
                });
            }
        }
        // Alerta si la red o el video quedaron con error (DESK-29).
        for device in &current {
            let category = device.category();
            let was_ok = previous
                .iter()
                .find(|d| d.instance_id == device.instance_id)
                .is_some_and(|d| d.problem_code == 0);
            if matches!(category, Category::Network | Category::Display)
                && device.problem_code != 0
                && was_ok
            {
                notices.push(Notice {
                    title: format!("{} dejó de funcionar", device.display_name()),
                    body: "Abre Drivward > Historial y usa \"Volver a la versión anterior\". Si no hay internet, el rollback usa la copia local.".into(),
                });
            }
        }
        if first_run {
            self.record_baseline(&current, &stamp);
        }
        let _ = self.db.save_devices(&current, &stamp);
        let _ = self.db.set(LAST_SCAN, &stamp);

        let since = self
            .db
            .last_event_time()
            .unwrap_or_else(|| format(now - Duration::days(BASELINE_DAYS)));
        if let Ok(events) = self.platform.events_since(&since) {
            let rows: Vec<EventRow> = events
                .into_iter()
                .map(|e| EventRow {
                    at: e.time,
                    event_id: e.id,
                    provider: e.provider,
                    message: e.message,
                })
                .collect();
            let _ = self.db.insert_events(&rows);
        }
        notices
    }

    /// En la primera revisión se guarda una copia de los drivers críticos actuales (DESK-20).
    fn record_baseline(&self, devices: &[Device], now: &str) {
        for device in devices
            .iter()
            .filter(|d| d.category().is_critical() && d.problem_code == 0)
        {
            self.remember_good(device, now);
        }
    }

    fn remember_good(&self, device: &Device, now: &str) {
        let Some(driver) = &device.driver else { return };
        let Some(version) = driver.version.clone().filter(|v| !v.is_empty()) else {
            return;
        };
        let inf = driver
            .inf_name
            .clone()
            .filter(|i| drivward_platform::is_published_inf(i));
        let backup_path = inf.as_ref().and_then(|inf| {
            let folder = self
                .data_dir
                .join("respaldos")
                .join(folder_name(&device.instance_id))
                .join(&version);
            self.platform
                .backup_driver(inf, &folder)
                .ok()
                .map(|_| folder.display().to_string())
        });
        let good = KnownGood {
            version,
            inf_name: inf,
            backup_path,
        };
        let _ = self.db.save_known_good(&device.instance_id, &good, now);
    }

    fn refresh_updates(&mut self, now: OffsetDateTime) {
        if let Ok(updates) = self.platform.windows_update_drivers(false) {
            self.updates = updates;
            let _ = self.db.set(LAST_CHECK, &format(now));
        }
    }

    fn validate_changes(&mut self, now: OffsetDateTime, settings: &Settings) -> Vec<Notice> {
        let mut notices = Vec::new();
        let events = self.events_for_evidence(now);
        let devices = self.db.devices().unwrap_or_default();
        for change in self
            .db
            .changes(200)
            .unwrap_or_default()
            .into_iter()
            .filter(|c| !c.validated)
        {
            let Some(at) = parse(&change.at) else {
                continue;
            };
            let category = Category::from_class(change.class.as_deref());
            let evidence = stability::evaluate(
                &change.at,
                category,
                &events,
                now,
                settings.observation_days,
            );
            if evidence.confidence >= Confidence::Medium {
                if settings.auto_rollback
                    && evidence.confidence == Confidence::High
                    && self.can_rollback(&change)
                {
                    let response = self.rollback(change.id, now);
                    if let Response::Done { message } | Response::Error { message } = response {
                        notices.push(Notice {
                            title: format!("Rollback automático de {}", change.device_name),
                            body: message,
                        });
                    }
                }
                continue;
            }
            if now - at >= Duration::days(settings.observation_days as i64) {
                let _ = self.db.mark_validated(change.id);
                if let Some(device) = devices.iter().find(|d| d.instance_id == change.instance_id) {
                    self.remember_good(device, &format(now));
                }
            }
        }
        notices
    }

    fn events_for_evidence(&self, now: OffsetDateTime) -> Vec<EventRow> {
        let since = format(now - Duration::days(BASELINE_DAYS * 2 + 30));
        self.db.events_since(&since).unwrap_or_default()
    }

    fn changes_in_observation(&self) -> u32 {
        self.db
            .changes(200)
            .unwrap_or_default()
            .iter()
            .filter(|c| {
                !c.validated
                    && c.origin == Origin::Drivward
                    && Category::from_class(c.class.as_deref()).is_critical()
            })
            .count() as u32
    }

    fn conditions(&self) -> Conditions {
        Conditions {
            low_battery: self.platform.low_battery().unwrap_or(false),
            reboot_pending: self.platform.reboot_pending().unwrap_or(false),
            changes_in_observation: self.changes_in_observation(),
        }
    }

    fn device_for(&self, update: &WindowsUpdateDriver, devices: &[Device]) -> Option<Device> {
        let id = normalize(update.hardware_id.as_deref()?);
        devices
            .iter()
            .find(|d| {
                d.hardware_ids
                    .iter()
                    .chain(&d.compatible_ids)
                    .any(|own| normalize(own) == id)
            })
            .cloned()
    }

    pub fn recommendations(&self, now: OffsetDateTime) -> Vec<RecommendationView> {
        let settings = self.settings();
        let devices = self.db.devices().unwrap_or_default();
        let blocked = policy::blocked_reason(&self.conditions());
        self.updates
            .iter()
            .filter_map(|update| {
                let device = self.device_for(update, &devices)?;
                let category = device.category();
                let ready = policy::quarantine_passed(
                    update.date.as_deref(),
                    settings.quarantine_days,
                    now,
                );
                let available_after = (!ready)
                    .then(|| {
                        policy::available_after(update.date.as_deref(), settings.quarantine_days)
                    })
                    .flatten()
                    .map(|d| d.to_string());
                let mut reasons = vec![format!("Windows Update ofrece \"{}\".", update.title)];
                if let Some(date) = &update.date {
                    reasons.push(format!("Publicado el {date}."));
                }
                if !ready {
                    reasons.push(format!(
                        "Se espera {} días desde la publicación antes de recomendarlo.",
                        settings.quarantine_days
                    ));
                }
                if category.is_critical() {
                    reasons.push("Es un driver crítico: siempre pide tu aprobación.".into());
                }
                Some(RecommendationView {
                    id: update.update_id.clone(),
                    device_id: device.instance_id.clone(),
                    device_name: device.display_name().to_owned(),
                    category,
                    current_version: device.driver.as_ref().and_then(|d| d.version.clone()),
                    new_version: update.date.clone(),
                    source: "Windows Update".into(),
                    reasons,
                    requires_approval: policy::requires_approval(settings.mode, category),
                    available_after,
                    blocked_reason: blocked.clone(),
                })
            })
            .collect()
    }

    fn apply_automatic(&mut self, now: OffsetDateTime) -> Vec<Notice> {
        let candidate = self.recommendations(now).into_iter().find(|r| {
            !r.requires_approval && r.available_after.is_none() && r.blocked_reason.is_none()
        });
        let Some(recommendation) = candidate else {
            return Vec::new();
        };
        let name = recommendation.device_name.clone();
        match self.install(&recommendation.id, now) {
            Response::Done { message } => vec![Notice {
                title: format!("Driver actualizado: {name}"),
                body: message,
            }],
            Response::Error { message } => vec![Notice {
                title: format!("No se actualizó {name}"),
                body: message,
            }],
            _ => Vec::new(),
        }
    }

    fn install(&mut self, update_id: &str, now: OffsetDateTime) -> Response {
        let Some(recommendation) = self
            .recommendations(now)
            .into_iter()
            .find(|r| r.id == update_id)
        else {
            return Response::Error {
                message: "La recomendación ya no está disponible. Busca de nuevo.".into(),
            };
        };
        if let Some(reason) = recommendation.blocked_reason {
            return Response::Error { message: reason };
        }
        if let Some(date) = recommendation.available_after {
            return Response::Error {
                message: format!("La versión sigue en espera hasta el {date}."),
            };
        }
        let stamp = format(now);
        let devices = self.db.devices().unwrap_or_default();
        if let Some(device) = devices
            .iter()
            .find(|d| d.instance_id == recommendation.device_id)
        {
            self.remember_good(device, &stamp);
        }
        let restore = self
            .platform
            .create_restore_point("Drivward: antes de actualizar un driver");
        self.log(&ActionRecord::new(
            ActionKind::RestorePoint,
            match restore {
                Ok(true) => Outcome::Success,
                Ok(false) => Outcome::Skipped("Windows no permitió crear el punto".into()),
                Err(e) => Outcome::Skipped(e.to_string()),
            },
        ));
        let _ = self.db.add_pending(&recommendation.device_id, &stamp);
        let mut record = ActionRecord::new(ActionKind::Install, Outcome::Success);
        record.device_id = Some(recommendation.device_id.clone());
        record.device_name = Some(recommendation.device_name.clone());
        record.previous_version = recommendation.current_version.clone();
        record.new_version = recommendation.new_version.clone();
        record.target = Some(update_id.to_owned());
        let response = match self
            .platform
            .install_windows_updates(&[update_id.to_owned()])
        {
            Ok(result) => {
                record.output = journal::trim_output(&result.output.output);
                record.outcome = if result.reboot_required {
                    Outcome::RebootRequired
                } else {
                    Outcome::Success
                };
                let mut message =
                    format!("Se instaló el driver de {}.", recommendation.device_name);
                if result.reboot_required {
                    message.push_str(" Reinicia para completar el cambio.");
                }
                message.push_str(
                    " Si algo falla, usa \"Volver a la versión anterior\" en el historial.",
                );
                Response::Done { message }
            }
            Err(error) => {
                let _ = self.db.take_pending(&recommendation.device_id);
                record.outcome = Outcome::Failed(error.to_string());
                Response::Error {
                    message: format!("No se pudo instalar: {error}"),
                }
            }
        };
        self.log(&record);
        self.scan(now);
        response
    }

    fn can_rollback(&self, change: &ChangeRow) -> bool {
        let Some(old) = &change.old_version else {
            return false;
        };
        let has_backup = self
            .db
            .known_good(&change.instance_id, old)
            .ok()
            .flatten()
            .is_some_and(|good| good.backup_path.is_some());
        let new_removable = change
            .new_inf
            .as_deref()
            .is_some_and(drivward_platform::is_published_inf);
        new_removable && (has_backup || change.old_inf.is_some())
    }

    /// Vuelve a la versión anterior: asegura que esté en el Driver Store, quita la nueva y comprueba (DESK-21, DESK-22).
    pub fn rollback(&mut self, change_id: i64, now: OffsetDateTime) -> Response {
        let Ok(Some(change)) = self.db.change(change_id) else {
            return Response::Error {
                message: "No se encontró el cambio.".into(),
            };
        };
        if !self.can_rollback(&change) {
            return Response::Error {
                message: "No hay una versión anterior disponible para este dispositivo.".into(),
            };
        }
        let current = self.platform.devices().unwrap_or_default();
        let Some(device) = current.iter().find(|d| d.instance_id == change.instance_id) else {
            return Response::Error {
                message: "El dispositivo ya no está conectado.".into(),
            };
        };
        let current_inf = device.driver.as_ref().and_then(|d| d.inf_name.clone());
        if current_inf != change.new_inf {
            return Response::Error { message: "El driver cambió otra vez después de este registro. Revisa el historial más reciente.".into() };
        }
        let old_version = change.old_version.clone().unwrap_or_default();
        let stamp = format(now);
        let _ = self
            .platform
            .create_restore_point("Drivward: antes de volver a la versión anterior");
        // Copia de la versión actual, por si hay que deshacer el rollback.
        self.remember_good(device, &stamp);

        let mut record = ActionRecord::new(ActionKind::Rollback, Outcome::Success);
        record.device_id = Some(change.instance_id.clone());
        record.device_name = Some(change.device_name.clone());
        record.previous_version = change.new_version.clone();
        record.new_version = change.old_version.clone();
        record.target = change.new_inf.clone();

        if let Ok(Some(good)) = self.db.known_good(&change.instance_id, &old_version) {
            if let Some(inf) = good
                .backup_path
                .as_deref()
                .and_then(|p| find_inf(Path::new(p)))
            {
                // Solo agrega el paquete al Driver Store; Windows lo elegirá al quitar el nuevo.
                if let Err(error) = self.platform.install_inf(&inf) {
                    record
                        .output
                        .push_str(&format!("No se pudo reinstalar la copia: {error}\n"));
                }
            }
        }
        let _ = self.db.add_pending(&change.instance_id, &stamp);
        let new_inf = change.new_inf.clone().unwrap_or_default();
        if let Err(error) = self.platform.remove_driver(&new_inf) {
            let _ = self.db.take_pending(&change.instance_id);
            record.outcome = Outcome::Failed(error.to_string());
            self.log(&record);
            return Response::Error {
                message: format!("No se pudo quitar el driver nuevo: {error}"),
            };
        }
        let after = self.platform.devices().unwrap_or_default();
        let version_now = after
            .iter()
            .find(|d| d.instance_id == change.instance_id)
            .and_then(|d| d.driver.as_ref().and_then(|x| x.version.clone()));
        let response = if version_now.as_deref() == Some(old_version.as_str()) {
            Response::Done {
                message: format!(
                    "{} volvió a la versión {old_version}. Puedes bloquear la versión nueva en Windows Update para que no vuelva a instalarse.",
                    change.device_name
                ),
            }
        } else {
            record.outcome = Outcome::Failed(format!(
                "la versión actual es {}",
                version_now.as_deref().unwrap_or("desconocida")
            ));
            Response::Error {
                message: format!(
                    "Se quitó el driver nuevo, pero el dispositivo quedó con {}. Reinicia la compu; si sigue igual, usa el punto de restauración.",
                    version_now.as_deref().unwrap_or("una versión desconocida")
                ),
            }
        };
        self.log(&record);
        self.scan(now);
        response
    }

    fn status(&self, now: OffsetDateTime) -> Status {
        let settings = self.settings();
        let devices = self.device_views();
        let attention = devices
            .iter()
            .filter(|d| matches!(d.state, DeviceState::MissingDriver | DeviceState::Error))
            .count() as u32;
        let since = format(now - Duration::days(7));
        let events = self.db.events_since(&since).map(|e| e.len()).unwrap_or(0) as u32;
        let evidence = self.evidence(now);
        let unstable = evidence.iter().any(|e| e.suggest_rollback);
        let recommendations = self.updates.len() as u32;
        let (health, message) = if policy::is_paused(&settings, now) {
            (Health::Paused, "Monitoreo en pausa.".to_owned())
        } else if unstable {
            (
                Health::Unstable,
                "Hay fallos después de un cambio de driver. Revisa la evidencia.".to_owned(),
            )
        } else if attention > 0 {
            (
                Health::Attention,
                format!("{attention} dispositivos tienen problemas de driver."),
            )
        } else {
            (Health::Stable, "El equipo está estable.".to_owned())
        };
        Status {
            health,
            message,
            mode: settings.mode,
            paused_until: settings.paused_until,
            last_scan: self.db.get(LAST_SCAN),
            devices_total: devices.len() as u32,
            devices_attention: attention,
            recommendations,
            events_last_7_days: events,
            changes_in_observation: self.changes_in_observation(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn device_views(&self) -> Vec<DeviceView> {
        let mut views: Vec<DeviceView> = self
            .db
            .devices()
            .unwrap_or_default()
            .into_iter()
            .filter(|d| !d.hardware_ids.is_empty())
            .map(|device| {
                let state = classify(&device, &Selection::default());
                let driver = device.driver.clone().unwrap_or_default();
                DeviceView {
                    instance_id: device.instance_id.clone(),
                    name: device.display_name().to_owned(),
                    category: device.category(),
                    state,
                    provider: driver.provider,
                    version: driver.version,
                    date: driver.date,
                }
            })
            .collect();
        views.sort_by_key(|d| (d.state.priority(), d.name.to_lowercase()));
        views
    }

    fn history(&self, limit: u32) -> Vec<ChangeView> {
        self.db
            .changes(limit)
            .unwrap_or_default()
            .into_iter()
            .map(|change| ChangeView {
                can_rollback: self.can_rollback(&change),
                id: change.id,
                at: change.at,
                device_id: change.instance_id,
                device_name: change.device_name,
                category: Category::from_class(change.class.as_deref()),
                old_version: change.old_version,
                new_version: change.new_version,
                origin: change.origin,
                validated: change.validated,
            })
            .collect()
    }

    fn evidence(&self, now: OffsetDateTime) -> Vec<EvidenceView> {
        let settings = self.settings();
        let events = self.events_for_evidence(now);
        let since = now - Duration::days(BASELINE_DAYS);
        self.db
            .changes(100)
            .unwrap_or_default()
            .into_iter()
            .filter(|c| parse(&c.at).is_some_and(|at| at >= since))
            .map(|change| {
                let category = Category::from_class(change.class.as_deref());
                let window = settings.observation_days.max(7);
                let e = stability::evaluate(&change.at, category, &events, now, window);
                EvidenceView {
                    suggest_rollback: e.confidence >= Confidence::Medium
                        && self.can_rollback(&change),
                    change_id: change.id,
                    device_name: change.device_name,
                    events_before: e.events_before,
                    events_after: e.events_after,
                    days_before: e.days_before,
                    days_after: e.days_after,
                    confidence: e.confidence,
                    explanation: e.explanation,
                }
            })
            .collect()
    }

    fn block(&mut self, update_id: &str, now: OffsetDateTime) -> Response {
        let update = self
            .updates
            .iter()
            .find(|u| u.update_id == update_id)
            .cloned();
        if let Err(error) = self.platform.set_update_hidden(update_id, true) {
            return Response::Error {
                message: format!("No se pudo bloquear: {error}"),
            };
        }
        let block = BlockView {
            update_id: update_id.to_owned(),
            title: update
                .as_ref()
                .map(|u| u.title.clone())
                .unwrap_or_else(|| update_id.to_owned()),
            hardware_id: update.and_then(|u| u.hardware_id),
            created_at: format(now),
        };
        let _ = self.db.add_block(&block);
        self.updates.retain(|u| u.update_id != update_id);
        let mut record = ActionRecord::new(ActionKind::HideUpdate, Outcome::Success);
        record.target = Some(update_id.to_owned());
        self.log(&record);
        Response::Done {
            message: format!("Windows Update ya no ofrecerá \"{}\".", block.title),
        }
    }

    fn unblock(&mut self, update_id: &str) -> Response {
        if let Err(error) = self.platform.set_update_hidden(update_id, false) {
            return Response::Error {
                message: format!("No se pudo quitar el bloqueo: {error}"),
            };
        }
        let _ = self.db.remove_block(update_id);
        let mut record = ActionRecord::new(ActionKind::UnhideUpdate, Outcome::Success);
        record.target = Some(update_id.to_owned());
        self.log(&record);
        Response::Done {
            message: "Se quitó el bloqueo. Windows Update podrá ofrecer esa versión otra vez."
                .into(),
        }
    }

    fn set_settings(&mut self, mut settings: Settings) -> Response {
        settings.quarantine_days = settings.quarantine_days.min(90);
        settings.observation_days = settings.observation_days.clamp(1, 60);
        settings.check_hour = settings.check_hour.min(23);
        match self.db.save_settings(&settings) {
            Ok(()) => Response::Done {
                message: "Configuración guardada.".into(),
            },
            Err(error) => Response::Error {
                message: error.to_string(),
            },
        }
    }

    pub fn handle(&mut self, request: Request, now: OffsetDateTime) -> Response {
        match request {
            Request::Status => Response::Status(self.status(now)),
            Request::Devices => Response::Devices(self.device_views()),
            Request::History { limit } => Response::History(self.history(limit.clamp(1, 500))),
            Request::Recommendations => Response::Recommendations(self.recommendations(now)),
            Request::Evidence => Response::Evidence(self.evidence(now)),
            Request::Blocks => Response::Blocks(self.db.blocks().unwrap_or_default()),
            Request::Settings => Response::Settings(self.settings()),
            Request::SetSettings { settings } => self.set_settings(settings),
            Request::Pause { hours } => {
                let mut settings = self.settings();
                settings.paused_until = Some(format(
                    now + Duration::hours(hours.clamp(1, 24 * 30) as i64),
                ));
                self.set_settings(settings)
            }
            Request::Resume => {
                let mut settings = self.settings();
                settings.paused_until = None;
                self.set_settings(settings)
            }
            Request::ScanNow => {
                self.scan(now);
                self.refresh_updates(now);
                Response::Done {
                    message: "Revisión terminada.".into(),
                }
            }
            Request::Install { recommendation_id } => {
                if self.settings().mode == Mode::Observe {
                    return Response::Error {
                        message:
                            "El modo Observar no instala drivers. Cambia a Recomendar en Ajustes."
                                .into(),
                    };
                }
                self.install(&recommendation_id, now)
            }
            Request::Rollback { change_id } => self.rollback(change_id, now),
            Request::MarkGood { change_id } => {
                let Ok(Some(change)) = self.db.change(change_id) else {
                    return Response::Error {
                        message: "No se encontró el cambio.".into(),
                    };
                };
                let _ = self.db.mark_validated(change_id);
                if let Some(device) = self
                    .db
                    .devices()
                    .unwrap_or_default()
                    .iter()
                    .find(|d| d.instance_id == change.instance_id)
                {
                    self.remember_good(device, &format(now));
                }
                Response::Done {
                    message: "Marcado como estable en este equipo.".into(),
                }
            }
            Request::Block { update_id } => self.block(&update_id, now),
            Request::Unblock { update_id } => self.unblock(&update_id),
        }
    }
}

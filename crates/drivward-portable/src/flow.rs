//! Flujo de la portable: recuperar la red, escanear y aplicar lo que el usuario apruebe.
//!
//! No depende de la ventana, así se puede probar con una plataforma falsa.

use drivward_core::classify::{classify, DeviceState};
use drivward_core::index::{self, Index, Verification};
use drivward_core::journal::{self, ActionKind, ActionRecord, Journal, Outcome};
use drivward_core::library;
use drivward_core::model::{Device, SystemInfo};
use drivward_core::report::{DeviceReport, Report};
use drivward_core::select::{self, Candidate, Context, Selection, Signature, Source, Stability};
use drivward_platform::{Connectivity, Platform, WindowsUpdateDriver};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const LIBRARY_DIR: &str = "drivers";
const LOG_FILE: &str = "logs/acciones.jsonl";
const PENDING_FILE: &str = "estado/pendiente.json";

/// Librería de drivers de la USB, ya comparada con su índice.
pub struct Library {
    pub root: PathBuf,
    pub index: Index,
    pub verification: Verification,
    candidates: Vec<Candidate>,
}

impl Library {
    pub fn open(root: &Path) -> Result<Self, String> {
        let index = index::load(root).map_err(|error| error.to_string())?;
        let verification = index::verify(root, &index).map_err(|error| error.to_string())?;
        let candidates = library::candidates(&index);
        Ok(Self {
            root: root.to_path_buf(),
            index,
            verification,
            candidates,
        })
    }

    /// Paquetes que no se deben usar porque sus archivos no coinciden con el índice.
    fn damaged_folders(&self) -> Vec<String> {
        let v = &self.verification;
        let mut folders: Vec<String> = v
            .missing
            .iter()
            .chain(&v.changed)
            .chain(&v.unexpected)
            .chain(&v.unsafe_paths)
            .map(|path| folder_of(path))
            .collect();
        folders.sort();
        folders.dedup();
        folders
    }
}

fn folder_of(relative: &str) -> String {
    relative
        .rsplit_once('/')
        .map(|(folder, _)| folder.to_owned())
        .unwrap_or_default()
}

/// Un dispositivo con su estado y el driver recomendado.
#[derive(Debug, Clone)]
pub struct Item {
    pub device: Device,
    pub state: DeviceState,
    pub selection: Selection,
}

impl Item {
    pub fn recommended(&self) -> Option<&Candidate> {
        self.selection.recommendation.as_ref().map(|r| &r.candidate)
    }

    /// Se puede instalar algo: falta driver, hay error o hay una versión mejor.
    pub fn actionable(&self) -> bool {
        self.recommended().is_some() && self.state != DeviceState::Ok
    }
}

pub struct Session<'a> {
    platform: &'a dyn Platform,
    root: PathBuf,
    pub library: Result<Library, String>,
    journal: Journal,
    pub system: SystemInfo,
    pub devices: Vec<Device>,
    windows_update: Vec<WindowsUpdateDriver>,
    signatures: HashMap<String, Signature>,
    safety_done: bool,
    pub actions: Vec<ActionRecord>,
}

impl<'a> Session<'a> {
    /// `root` es la carpeta de la portable en la USB.
    pub fn open(platform: &'a dyn Platform, root: &Path) -> Result<Self, String> {
        let journal = Journal::open(root.join(LOG_FILE))
            .map_err(|e| format!("no se pudo abrir el registro: {e}"))?;
        let library = Library::open(&root.join(LIBRARY_DIR));
        let mut session = Self {
            platform,
            root: root.to_path_buf(),
            library,
            journal,
            system: SystemInfo::default(),
            devices: Vec::new(),
            windows_update: Vec::new(),
            signatures: HashMap::new(),
            safety_done: false,
            actions: Vec::new(),
        };
        let record = match &session.library {
            Ok(lib) if lib.verification.is_valid() => {
                ActionRecord::new(ActionKind::Verify, Outcome::Success)
            }
            Ok(lib) => ActionRecord::new(
                ActionKind::Verify,
                Outcome::Failed(format!(
                    "{} paquetes no coinciden con el índice",
                    lib.damaged_folders().len()
                )),
            ),
            Err(error) => ActionRecord::new(ActionKind::Verify, Outcome::Failed(error.clone())),
        };
        session.log(record);
        Ok(session)
    }

    pub fn refresh(&mut self) -> Result<(), String> {
        self.system = self.platform.system_info().map_err(|e| e.to_string())?;
        self.devices = self.platform.devices().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn context(&self) -> Context {
        Context {
            os_build: self.system.os_build,
            architecture: if self.system.architecture.is_empty() {
                "x64".into()
            } else {
                self.system.architecture.clone()
            },
            system_manufacturer: self.system.manufacturer.clone(),
            allow_downgrade: false,
        }
    }

    pub fn connectivity(&self) -> Connectivity {
        self.platform.check_connectivity()
    }

    pub fn library_summary(&self) -> String {
        match &self.library {
            Ok(lib) if lib.verification.is_valid() => {
                format!("Librería verificada: {} drivers", lib.index.packages.len())
            }
            Ok(lib) => format!(
                "Librería con {} paquetes alterados o incompletos; no se usarán",
                lib.damaged_folders().len()
            ),
            Err(error) => format!("Sin librería de drivers: {error}"),
        }
    }

    fn library_candidates(&self) -> Vec<Candidate> {
        let Ok(lib) = &self.library else {
            return Vec::new();
        };
        let damaged = lib.damaged_folders();
        lib.candidates
            .iter()
            .filter(|c| !damaged.contains(&folder_of(&c.id)))
            .map(|c| {
                let mut c = c.clone();
                if let Some(signature) = self.signatures.get(&c.id) {
                    c.signature = *signature;
                }
                c
            })
            .collect()
    }

    fn windows_update_candidates(&self) -> Vec<Candidate> {
        let architecture = self.context().architecture;
        self.windows_update
            .iter()
            .filter_map(|driver| {
                let hardware_id = driver.hardware_id.as_deref().filter(|id| !id.is_empty())?;
                Some(Candidate {
                    id: driver.update_id.clone(),
                    source: Source::WindowsUpdate,
                    provider: driver.provider.clone(),
                    version: None,
                    date: driver.date.clone(),
                    architecture: architecture.clone(),
                    min_build: None,
                    hardware_ids: vec![select::normalize(hardware_id)],
                    // Microsoft firma lo que distribuye Windows Update.
                    signature: Signature::Valid,
                    approved_origin: true,
                    stability: Stability::Unknown,
                    oem: None,
                    category_hint: driver.class.clone(),
                })
            })
            .collect()
    }

    /// Adaptadores de red sin driver o con error, con el mejor driver de la librería (PORT-07, PORT-08).
    pub fn network_items(&self) -> Vec<Item> {
        let candidates = self.library_candidates();
        let context = self.context();
        let mut items: Vec<Item> = self
            .devices
            .iter()
            .filter(|device| device.looks_like_network())
            .map(|device| {
                let selection = select::select(device, &candidates, &context);
                Item {
                    state: classify(device, &selection),
                    device: device.clone(),
                    selection,
                }
            })
            .filter(|item| matches!(item.state, DeviceState::MissingDriver | DeviceState::Error))
            .collect();
        items.sort_by_key(|item| item.recommended().is_none());
        items
    }

    /// Escaneo completo con la librería y, si hay internet, Windows Update (PORT-12).
    pub fn scan(&mut self, use_windows_update: bool) -> Vec<Item> {
        if use_windows_update {
            // Si Windows Update falla, el escaneo sigue solo con la librería.
            self.windows_update = self
                .platform
                .windows_update_drivers(false)
                .unwrap_or_default();
        }
        let mut candidates = self.library_candidates();
        candidates.extend(self.windows_update_candidates());
        let context = self.context();
        let mut items: Vec<Item> = self
            .devices
            .iter()
            .filter(|device| !device.hardware_ids.is_empty())
            .map(|device| {
                let selection = select::select(device, &candidates, &context);
                Item {
                    state: classify(device, &selection),
                    device: device.clone(),
                    selection,
                }
            })
            .collect();
        items.sort_by_key(|item| {
            (
                item.state.priority(),
                item.device.display_name().to_lowercase(),
            )
        });
        items
    }

    fn log(&mut self, record: ActionRecord) {
        // Si la USB no permite escribir, la acción ya ocurrió: se conserva en memoria para el reporte.
        let _ = self.journal.append(&record);
        self.actions.push(record);
    }

    /// Respaldo de drivers y punto de restauración, una vez por sesión (CORE-15, CORE-16).
    fn ensure_safety(&mut self) -> Result<(), String> {
        if self.safety_done {
            return Ok(());
        }
        let stamp = journal::now()
            .replace([':', '-'], "")
            .chars()
            .take(15)
            .collect::<String>();
        let destination = self.root.join("respaldos").join(stamp);
        match self.platform.backup_drivers(&destination) {
            Ok(output) => {
                let mut record = ActionRecord::new(ActionKind::Backup, Outcome::Success);
                record.target = Some(destination.display().to_string());
                record.output = journal::trim_output(&output.output);
                self.log(record);
            }
            Err(error) => {
                let message = format!("no se pudo respaldar los drivers actuales: {error}");
                self.log(ActionRecord::new(
                    ActionKind::Backup,
                    Outcome::Failed(message.clone()),
                ));
                return Err(message);
            }
        }
        let outcome = match self
            .platform
            .create_restore_point("Drivward: antes de instalar drivers")
        {
            Ok(true) => Outcome::Success,
            Ok(false) => {
                Outcome::Skipped("Windows no permitió crear el punto de restauración".into())
            }
            Err(error) => Outcome::Skipped(error.to_string()),
        };
        self.log(ActionRecord::new(ActionKind::RestorePoint, outcome));
        self.safety_done = true;
        Ok(())
    }

    /// Instala el driver recomendado de un dispositivo que el usuario aprobó (PORT-13).
    pub fn install(&mut self, item: &Item) -> ActionRecord {
        let Some(candidate) = item.recommended().cloned() else {
            return ActionRecord::new(
                ActionKind::Install,
                Outcome::Skipped("no hay driver recomendado".into()),
            );
        };
        let mut record = ActionRecord::new(ActionKind::Install, Outcome::Success);
        record.device_id = Some(item.device.instance_id.clone());
        record.device_name = Some(item.device.display_name().to_owned());
        record.previous_version = item
            .device
            .driver
            .as_ref()
            .and_then(|d| d.version.clone())
            .filter(|v| !v.is_empty());
        record.new_version = candidate
            .version
            .map(|v| v.to_string())
            .or_else(|| candidate.date.clone());
        record.target = Some(candidate.id.clone());

        record.outcome = match self.ensure_safety() {
            Err(error) => Outcome::Failed(error),
            Ok(()) => match candidate.source {
                Source::Library => self.install_from_library(&candidate, &mut record.output),
                Source::WindowsUpdate => {
                    self.install_from_windows_update(&candidate, &mut record.output)
                }
                _ => Outcome::Skipped("este origen todavía no se instala desde la portable".into()),
            },
        };
        self.log(record.clone());
        record
    }

    fn install_from_library(&mut self, candidate: &Candidate, output: &mut String) -> Outcome {
        let Ok(lib) = &self.library else {
            return Outcome::Failed("no hay librería".into());
        };
        let folder = folder_of(&candidate.id);
        // Se vuelve a comprobar justo antes de instalar por si la USB cambió (PORT-17).
        match index::verify_folder(&lib.root, &lib.index, &folder) {
            Ok(v) if v.is_valid() => {}
            Ok(_) => {
                return Outcome::Failed("los archivos del driver no coinciden con el índice".into())
            }
            Err(error) => return Outcome::Failed(error.to_string()),
        }
        let inf_path = lib.root.join(&candidate.id);
        let package = lib.index.packages.iter().find(|p| p.inf == candidate.id);
        let catalog = package.and_then(|p| p.driver.catalog_file.as_deref());
        let Some(catalog) = catalog else {
            return Outcome::Failed("el driver no declara un catálogo firmado".into());
        };
        let catalog_path = inf_path.with_file_name(catalog);
        let signature = self
            .platform
            .verify_signature(&catalog_path)
            .unwrap_or(Signature::Invalid);
        self.signatures.insert(candidate.id.clone(), signature);
        if signature != Signature::Valid {
            return Outcome::Failed("la firma del driver no es válida (SEG-04)".into());
        }
        match self.platform.install_inf(&inf_path) {
            Ok(result) => {
                *output = journal::trim_output(&result.output.output);
                if result.not_applied {
                    Outcome::Skipped(
                        "Windows indicó que el driver no aplica o ya hay uno igual o mejor".into(),
                    )
                } else if result.reboot_required {
                    Outcome::RebootRequired
                } else {
                    Outcome::Success
                }
            }
            Err(error) => Outcome::Failed(error.to_string()),
        }
    }

    fn install_from_windows_update(
        &mut self,
        candidate: &Candidate,
        output: &mut String,
    ) -> Outcome {
        match self
            .platform
            .install_windows_updates(std::slice::from_ref(&candidate.id))
        {
            Ok(result) => {
                *output = journal::trim_output(&result.output.output);
                match (result.not_applied, result.reboot_required) {
                    (true, _) => {
                        Outcome::Skipped("Windows Update ya no ofrece esta actualización".into())
                    }
                    (_, true) => Outcome::RebootRequired,
                    _ => Outcome::Success,
                }
            }
            Err(error) => Outcome::Failed(error.to_string()),
        }
    }

    /// Guarda lo que falta instalar y pide a Windows abrir la portable al volver (CORE-14).
    pub fn save_pending(&self, instance_ids: &[String], executable: &Path) -> Result<(), String> {
        let path = self.root.join(PENDING_FILE);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(instance_ids).map_err(|e| e.to_string())?;
        fs::write(&path, text).map_err(|e| e.to_string())?;
        self.platform
            .schedule_resume(executable, "--reanudar")
            .map_err(|e| e.to_string())
    }

    /// Devuelve y borra la lista guardada antes del reinicio.
    pub fn take_pending(&self) -> Vec<String> {
        let path = self.root.join(PENDING_FILE);
        let pending = fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        let _ = fs::remove_file(path);
        pending
    }

    pub fn report(&self, items: &[Item]) -> Report {
        Report {
            generated_at: journal::now(),
            tool: format!("Drivward Portable {}", env!("CARGO_PKG_VERSION")),
            system: self.system.clone(),
            devices: items.iter().map(device_report).collect(),
            actions: self.actions.clone(),
        }
    }

    /// Escribe el reporte en HTML y JSON en la USB y devuelve la ruta del HTML (CORE-18).
    pub fn export_report(&self, items: &[Item]) -> Result<PathBuf, String> {
        let report = self.report(items);
        let folder = self.root.join("reportes");
        fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        let stamp = report
            .generated_at
            .replace([':', '-'], "")
            .chars()
            .take(15)
            .collect::<String>();
        let html = folder.join(format!("reporte-{stamp}.html"));
        fs::write(&html, report.to_html()).map_err(|e| e.to_string())?;
        fs::write(
            folder.join(format!("reporte-{stamp}.json")),
            report.to_json(),
        )
        .map_err(|e| e.to_string())?;
        Ok(html)
    }
}

pub fn device_report(item: &Item) -> DeviceReport {
    let driver = item.device.driver.as_ref();
    let recommendation = item.selection.recommendation.as_ref();
    DeviceReport {
        name: item.device.display_name().to_owned(),
        class: item.device.class.clone(),
        hardware_id: item.device.hardware_ids.first().cloned(),
        state: item.state,
        current_version: driver
            .and_then(|d| d.version.clone())
            .filter(|v| !v.is_empty()),
        current_provider: driver
            .and_then(|d| d.provider.clone())
            .filter(|v| !v.is_empty()),
        recommended_version: recommendation.map(|r| {
            r.candidate
                .version
                .map(|v| v.to_string())
                .or_else(|| r.candidate.date.clone())
                .unwrap_or_else(|| "-".into())
        }),
        recommended_source: recommendation.map(|r| r.candidate.source.label().to_owned()),
        reasons: recommendation
            .map(|r| r.reasons.clone())
            .unwrap_or_else(|| {
                item.selection
                    .rejected
                    .iter()
                    .map(|r| format!("Descartado {}: {}", r.candidate_id, r.reason))
                    .collect()
            }),
    }
}

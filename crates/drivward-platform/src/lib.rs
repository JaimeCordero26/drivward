//! Acceso al sistema operativo. El núcleo no conoce esta capa (RNF-10) y cada sistema
//! tiene su propia implementación (RNF-11). Hoy solo existe la de Windows.

use drivward_core::model::{Device, SystemInfo};
use drivward_core::select::Signature;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

pub mod connectivity;
#[cfg(windows)]
mod windows;

pub use connectivity::Connectivity;

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("esta función solo está disponible en Windows")]
    Unsupported,
    #[error("no se pudo ejecutar {program}: {source}")]
    Spawn {
        program: String,
        source: std::io::Error,
    },
    #[error("{program} terminó con código {code}: {output}")]
    Command {
        program: String,
        code: i32,
        output: String,
    },
    #[error("respuesta inesperada de {what}: {detail}")]
    Parse { what: String, detail: String },
}

pub type Result<T> = std::result::Result<T, PlatformError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandOutput {
    pub code: i32,
    pub output: String,
}

/// Resultado de instalar un driver o una actualización.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallResult {
    pub reboot_required: bool,
    /// Windows dijo que el driver no aplica o ya hay uno igual o mejor instalado.
    pub not_applied: bool,
    pub output: CommandOutput,
}

/// Driver ofrecido por Windows Update (CORE-19).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowsUpdateDriver {
    pub update_id: String,
    pub title: String,
    pub provider: Option<String>,
    pub hardware_id: Option<String>,
    pub model: Option<String>,
    pub class: Option<String>,
    pub date: Option<String>,
    #[serde(default)]
    pub hidden: bool,
}

/// Evento del sistema relevante para la estabilidad (DESK-07).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemEvent {
    /// RFC 3339.
    pub time: String,
    pub id: u32,
    pub provider: String,
    pub message: String,
}

pub trait Platform: Send + Sync {
    fn system_info(&self) -> Result<SystemInfo>;
    fn devices(&self) -> Result<Vec<Device>>;
    /// Comprueba la firma Authenticode de un catálogo `.cat` o ejecutable (SEG-03).
    fn verify_signature(&self, path: &Path) -> Result<Signature>;
    /// Agrega el INF al Driver Store e instala en los dispositivos que coincidan (CORE-13).
    fn install_inf(&self, inf: &Path) -> Result<InstallResult>;
    /// Exporta todos los drivers de terceros instalados (CORE-15).
    fn backup_drivers(&self, destination: &Path) -> Result<CommandOutput>;
    /// Exporta un solo INF publicado para poder reinstalarlo después (DESK-20).
    fn backup_driver(&self, published_inf: &str, destination: &Path) -> Result<CommandOutput>;
    /// Batería por debajo del 30 % y sin cargador (DESK-14). En equipos sin batería es `false`.
    fn low_battery(&self) -> Result<bool>;
    /// Devuelve `false` si Windows no permitió crear el punto (CORE-16).
    fn create_restore_point(&self, description: &str) -> Result<bool>;
    fn check_connectivity(&self) -> Connectivity {
        connectivity::check()
    }
    /// Abre la configuración de Wi-Fi (PORT-10).
    fn open_wifi_settings(&self) -> Result<()>;
    /// Abre la configuración para compartir internet desde el celular por USB (PORT-11).
    fn open_network_settings(&self) -> Result<()>;
    fn windows_update_drivers(&self, include_hidden: bool) -> Result<Vec<WindowsUpdateDriver>>;
    /// Descarga e instala las actualizaciones aprobadas (CORE-20).
    fn install_windows_updates(&self, update_ids: &[String]) -> Result<InstallResult>;
    /// Oculta o vuelve a mostrar una actualización para que Windows Update no la reinstale (DESK-23).
    fn set_update_hidden(&self, update_id: &str, hidden: bool) -> Result<()>;
    /// Quita un INF publicado (`oemNN.inf`) del Driver Store y de los dispositivos (DESK-22).
    fn remove_driver(&self, published_inf: &str) -> Result<CommandOutput>;
    fn events_since(&self, since_rfc3339: &str) -> Result<Vec<SystemEvent>>;
    /// Indica si hay un reinicio pendiente (DESK-14).
    fn reboot_pending(&self) -> Result<bool>;
    /// Ejecuta el programa una vez después del siguiente inicio de sesión (CORE-14).
    fn schedule_resume(&self, executable: &Path, arguments: &str) -> Result<()>;
}

/// Plataforma de este sistema, o una que responde "no soportado".
pub fn native() -> Box<dyn Platform> {
    #[cfg(windows)]
    {
        Box::new(windows::Windows)
    }
    #[cfg(not(windows))]
    {
        Box::new(Unsupported)
    }
}

/// Permite compilar y probar el resto del proyecto fuera de Windows.
pub struct Unsupported;

impl Platform for Unsupported {
    fn system_info(&self) -> Result<SystemInfo> {
        Err(PlatformError::Unsupported)
    }
    fn devices(&self) -> Result<Vec<Device>> {
        Err(PlatformError::Unsupported)
    }
    fn verify_signature(&self, _: &Path) -> Result<Signature> {
        Err(PlatformError::Unsupported)
    }
    fn install_inf(&self, _: &Path) -> Result<InstallResult> {
        Err(PlatformError::Unsupported)
    }
    fn backup_drivers(&self, _: &Path) -> Result<CommandOutput> {
        Err(PlatformError::Unsupported)
    }
    fn backup_driver(&self, _: &str, _: &Path) -> Result<CommandOutput> {
        Err(PlatformError::Unsupported)
    }
    fn low_battery(&self) -> Result<bool> {
        Err(PlatformError::Unsupported)
    }
    fn create_restore_point(&self, _: &str) -> Result<bool> {
        Err(PlatformError::Unsupported)
    }
    fn open_wifi_settings(&self) -> Result<()> {
        Err(PlatformError::Unsupported)
    }
    fn open_network_settings(&self) -> Result<()> {
        Err(PlatformError::Unsupported)
    }
    fn windows_update_drivers(&self, _: bool) -> Result<Vec<WindowsUpdateDriver>> {
        Err(PlatformError::Unsupported)
    }
    fn install_windows_updates(&self, _: &[String]) -> Result<InstallResult> {
        Err(PlatformError::Unsupported)
    }
    fn set_update_hidden(&self, _: &str, _: bool) -> Result<()> {
        Err(PlatformError::Unsupported)
    }
    fn remove_driver(&self, _: &str) -> Result<CommandOutput> {
        Err(PlatformError::Unsupported)
    }
    fn events_since(&self, _: &str) -> Result<Vec<SystemEvent>> {
        Err(PlatformError::Unsupported)
    }
    fn reboot_pending(&self) -> Result<bool> {
        Err(PlatformError::Unsupported)
    }
    fn schedule_resume(&self, _: &Path, _: &str) -> Result<()> {
        Err(PlatformError::Unsupported)
    }
}

/// Un INF publicado tiene la forma `oemNN.inf`. Se valida antes de pasarlo a `pnputil`.
pub fn is_published_inf(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower
        .strip_prefix("oem")
        .and_then(|rest| rest.strip_suffix(".inf"))
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

/// Interpreta el código de salida de `pnputil /add-driver /install`.
pub fn interpret_pnputil(output: CommandOutput) -> Result<InstallResult> {
    const REBOOT_REQUIRED: i32 = 3010;
    const NO_MORE_ITEMS: i32 = 259;
    match output.code {
        0 => Ok(InstallResult {
            reboot_required: false,
            not_applied: false,
            output,
        }),
        REBOOT_REQUIRED => Ok(InstallResult {
            reboot_required: true,
            not_applied: false,
            output,
        }),
        NO_MORE_ITEMS => Ok(InstallResult {
            reboot_required: false,
            not_applied: true,
            output,
        }),
        code => Err(PlatformError::Command {
            program: "pnputil".into(),
            code,
            output: output.output,
        }),
    }
}

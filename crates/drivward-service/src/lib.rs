//! Servicio de escritorio de Drivward: vigila drivers y estabilidad, y aplica cambios
//! solo con evidencia suficiente y con una ruta de vuelta.

pub mod db;
pub mod engine;
pub mod monitor;
pub mod policy;
#[cfg(windows)]
pub mod service;
pub mod stability;

use std::path::PathBuf;

pub const SERVICE_NAME: &str = "Drivward";
pub const DISPLAY_NAME: &str = "Drivward";
pub const DESCRIPTION: &str =
    "Vigila los drivers y la estabilidad del equipo. Nada se instala sin tu aprobación.";

/// Carpeta de datos del servicio: `%ProgramData%\Drivward`.
pub fn data_dir() -> PathBuf {
    std::env::var_os("ProgramData")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Drivward")
}

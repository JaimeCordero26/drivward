//! Datos del equipo y de sus dispositivos, tal como los entrega la capa de plataforma.

use serde::{Deserialize, Serialize};

/// Identificación del sistema (CORE-01 y CORE-02).
///
/// No incluye nombre del equipo, usuario ni número de serie: no hacen falta para elegir drivers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os_name: String,
    /// Versión del núcleo, por ejemplo `10.0`.
    pub os_version: String,
    pub os_build: u32,
    /// Versión comercial, por ejemplo `23H2`.
    pub os_display_version: Option<String>,
    pub edition: Option<String>,
    /// `x64`, `arm64` o `x86`.
    pub architecture: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    /// Número de producto o SKU del fabricante.
    pub product_number: Option<String>,
    pub baseboard: Option<String>,
    pub bios_version: Option<String>,
    pub cpu: Option<String>,
    #[serde(default)]
    pub gpus: Vec<String>,
}

/// Dispositivo PnP (CORE-03).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    pub instance_id: String,
    pub name: Option<String>,
    pub class: Option<String>,
    pub class_guid: Option<String>,
    /// De más específico a más general, como los entrega Windows.
    #[serde(default)]
    pub hardware_ids: Vec<String>,
    #[serde(default)]
    pub compatible_ids: Vec<String>,
    pub status: Option<String>,
    /// Código de problema del Administrador de dispositivos. 0 es correcto.
    #[serde(default)]
    pub problem_code: u32,
    pub driver: Option<InstalledDriver>,
}

/// Driver instalado en un dispositivo (CORE-04).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledDriver {
    pub provider: Option<String>,
    pub version: Option<String>,
    /// AAAA-MM-DD.
    pub date: Option<String>,
    /// INF publicado en el Driver Store, por ejemplo `oem12.inf`.
    pub inf_name: Option<String>,
    #[serde(default)]
    pub signed: bool,
    pub signer: Option<String>,
}

/// Código 28: el dispositivo no tiene driver instalado.
pub const PROBLEM_NO_DRIVER: u32 = 28;
/// Código 1: el dispositivo no está bien configurado, suele indicar que falta el driver.
pub const PROBLEM_NOT_CONFIGURED: u32 = 1;

/// Grupo de dispositivos que define el riesgo de cambiar su driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Network,
    Storage,
    Display,
    Chipset,
    Audio,
    Other,
}

impl Category {
    pub fn from_class(class: Option<&str>) -> Self {
        let class = class.unwrap_or_default().to_ascii_lowercase();
        match class.as_str() {
            "net" => Self::Network,
            "scsiadapter" | "hdc" | "diskdrive" | "storport" => Self::Storage,
            "display" => Self::Display,
            "system" => Self::Chipset,
            "media" | "audioendpoint" | "audioprocessingobject" => Self::Audio,
            _ => Self::Other,
        }
    }

    /// Cambiar estos drivers puede dejar el equipo sin internet, video o arranque (DESK-18).
    pub fn is_critical(self) -> bool {
        matches!(
            self,
            Self::Network | Self::Storage | Self::Display | Self::Chipset
        )
    }
}

impl Device {
    pub fn category(&self) -> Category {
        Category::from_class(self.class.as_deref())
    }

    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.instance_id)
    }

    pub fn lacks_driver(&self) -> bool {
        matches!(
            self.problem_code,
            PROBLEM_NO_DRIVER | PROBLEM_NOT_CONFIGURED
        ) || self.driver.as_ref().is_none_or(|driver| {
            blank(driver.inf_name.as_deref()) && blank(driver.version.as_deref())
        })
    }

    /// Un adaptador sin driver suele aparecer sin clase, así que también se miran sus IDs.
    pub fn looks_like_network(&self) -> bool {
        if self.category() == Category::Network {
            return true;
        }
        if self.class.is_some() && !self.lacks_driver() {
            return false;
        }
        // Clase PCI 02 = controlador de red.
        self.compatible_ids
            .iter()
            .any(|id| id.to_ascii_uppercase().starts_with("PCI\\CC_02"))
    }
}

fn blank(text: Option<&str>) -> bool {
    text.is_none_or(|value| value.trim().is_empty())
}

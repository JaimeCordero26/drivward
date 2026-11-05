//! Estado de cada dispositivo (CORE-05).

use crate::model::Device;
use crate::select::Selection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceState {
    MissingDriver,
    Error,
    Updatable,
    Ok,
    Unknown,
}

impl DeviceState {
    pub fn label(self) -> &'static str {
        match self {
            Self::MissingDriver => "Falta driver",
            Self::Error => "Con error",
            Self::Updatable => "Actualizable",
            Self::Ok => "Correcto",
            Self::Unknown => "Desconocido",
        }
    }

    /// Orden para mostrar primero lo que necesita atención.
    pub fn priority(self) -> u8 {
        match self {
            Self::MissingDriver => 0,
            Self::Error => 1,
            Self::Updatable => 2,
            Self::Unknown => 3,
            Self::Ok => 4,
        }
    }
}

pub fn classify(device: &Device, selection: &Selection) -> DeviceState {
    if device.lacks_driver() {
        return DeviceState::MissingDriver;
    }
    if device.problem_code != 0 {
        return DeviceState::Error;
    }
    match &selection.recommendation {
        Some(recommendation) if recommendation.is_update => DeviceState::Updatable,
        _ if device
            .driver
            .as_ref()
            .is_some_and(|driver| driver.version.is_some()) =>
        {
            DeviceState::Ok
        }
        _ => DeviceState::Unknown,
    }
}

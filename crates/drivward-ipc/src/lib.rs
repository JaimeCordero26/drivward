//! Protocolo entre el servicio de Windows y la app de bandeja.
//!
//! Cada conexión envía una línea JSON con un [`Request`] y recibe una línea con un [`Response`].
//! Hay dos tuberías: la de estado acepta a usuarios de la sesión y solo responde consultas;
//! la de control solo acepta administradores con permisos elevados (SEG-07).

use drivward_core::classify::DeviceState;
use drivward_core::model::Category;
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, BufReader, Read, Write};

pub const STATUS_PIPE: &str = r"\\.\pipe\drivward-estado";
pub const CONTROL_PIPE: &str = r"\\.\pipe\drivward-control";
/// Una petición más larga se rechaza para no agotar memoria.
pub const MAX_LINE: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Solo registra cambios y fallos. No instala nada.
    #[default]
    Observe,
    /// Propone cambios y espera aprobación (DESK-05).
    Recommend,
    /// Instala solo drivers de bajo riesgo; el resto pide aprobación (DESK-17, DESK-18).
    Automatic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub mode: Mode,
    /// Días de espera antes de recomendar una versión nueva (DESK-12).
    pub quarantine_days: u32,
    /// Días sin problemas para dar por validada una actualización (DESK-16).
    pub observation_days: u32,
    /// Hora local para buscar actualizaciones (DESK-11).
    pub check_hour: u8,
    /// RFC 3339. Mientras no haya pasado, el monitoreo y las actualizaciones están en pausa (DESK-10).
    pub paused_until: Option<String>,
    /// Rollback automático con evidencia alta (DESK-28). Apagado por defecto.
    pub auto_rollback: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: Mode::Observe,
            quarantine_days: 7,
            observation_days: 7,
            check_hour: 12,
            paused_until: None,
            auto_rollback: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    Status,
    Devices,
    History { limit: u32 },
    Recommendations,
    Evidence,
    Blocks,
    Settings,
    SetSettings { settings: Settings },
    Pause { hours: u32 },
    Resume,
    ScanNow,
    Install { recommendation_id: String },
    Rollback { change_id: i64 },
    MarkGood { change_id: i64 },
    Block { update_id: String },
    Unblock { update_id: String },
}

impl Request {
    /// Las peticiones que cambian el sistema o la configuración solo van por la tubería de control.
    pub fn is_control(&self) -> bool {
        !matches!(
            self,
            Self::Status
                | Self::Devices
                | Self::History { .. }
                | Self::Recommendations
                | Self::Evidence
                | Self::Blocks
                | Self::Settings
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Health {
    Stable,
    Attention,
    Unstable,
    Paused,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    pub health: Health,
    pub message: String,
    pub mode: Mode,
    pub paused_until: Option<String>,
    pub last_scan: Option<String>,
    pub devices_total: u32,
    pub devices_attention: u32,
    pub recommendations: u32,
    pub events_last_7_days: u32,
    pub changes_in_observation: u32,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceView {
    pub instance_id: String,
    pub name: String,
    pub category: Category,
    pub state: DeviceState,
    pub provider: Option<String>,
    pub version: Option<String>,
    pub date: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    Drivward,
    /// Windows Update, el usuario u otro programa: Windows no indica cuál.
    External,
    Rollback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeView {
    pub id: i64,
    pub at: String,
    pub device_id: String,
    pub device_name: String,
    pub category: Category,
    pub old_version: Option<String>,
    pub new_version: Option<String>,
    pub origin: Origin,
    pub validated: bool,
    /// La versión anterior sigue en el Driver Store o en un respaldo.
    pub can_rollback: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    None,
    Low,
    Medium,
    High,
}

/// Relación entre un cambio de driver y fallos posteriores (DESK-09).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceView {
    pub change_id: i64,
    pub device_name: String,
    pub events_before: u32,
    pub events_after: u32,
    pub days_before: f64,
    pub days_after: f64,
    pub confidence: Confidence,
    pub explanation: String,
    pub suggest_rollback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecommendationView {
    pub id: String,
    pub device_id: String,
    pub device_name: String,
    pub category: Category,
    pub current_version: Option<String>,
    pub new_version: Option<String>,
    pub source: String,
    pub reasons: Vec<String>,
    pub requires_approval: bool,
    /// Si la versión aún está en espera, fecha desde la que se podrá instalar.
    pub available_after: Option<String>,
    /// Por qué no se puede instalar ahora (batería, reinicio pendiente, otro cambio en observación).
    pub blocked_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockView {
    pub update_id: String,
    pub title: String,
    pub hardware_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum Response {
    Status(Status),
    Devices(Vec<DeviceView>),
    History(Vec<ChangeView>),
    Recommendations(Vec<RecommendationView>),
    Evidence(Vec<EvidenceView>),
    Blocks(Vec<BlockView>),
    Settings(Settings),
    Done { message: String },
    Error { message: String },
}

/// Lee una línea con límite de tamaño.
pub fn read_line(reader: &mut impl BufRead) -> io::Result<String> {
    let mut line = Vec::new();
    reader
        .by_ref()
        .take(MAX_LINE as u64 + 1)
        .read_until(b'\n', &mut line)?;
    if line.len() > MAX_LINE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "mensaje demasiado largo",
        ));
    }
    String::from_utf8(line)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "mensaje no es UTF-8"))
}

/// Envía una petición al servicio por la tubería indicada.
pub fn request(pipe: &str, request: &Request) -> io::Result<Response> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(pipe)?;
    let mut writer = &file;
    let mut line = serde_json::to_string(request).map_err(io::Error::other)?;
    line.push('\n');
    writer.write_all(line.as_bytes())?;
    writer.flush()?;
    let mut reader = BufReader::new(&file);
    let reply = read_line(&mut reader)?;
    serde_json::from_str(&reply).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

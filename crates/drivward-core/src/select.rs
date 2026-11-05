//! Elección del driver recomendado para cada dispositivo (CORE-09 a CORE-12, CORE-23, CORE-25).

use crate::model::Device;
use crate::version::DriverVersion;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// De dónde viene un candidato. El orden importa: más abajo es más confiable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// Librería local de la USB.
    Library,
    WindowsUpdate,
    /// Fabricante del componente: Intel, AMD, NVIDIA, Realtek...
    Vendor,
    /// Fabricante del equipo: Dell, Lenovo, HP...
    Oem,
}

impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Self::Library => "librería local",
            Self::WindowsUpdate => "Windows Update",
            Self::Vendor => "fabricante del componente",
            Self::Oem => "fabricante del equipo",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Signature {
    Valid,
    Invalid,
    /// Aún no se comprobó. La plataforma la comprueba antes de instalar (SEG-03).
    Unchecked,
}

/// Estado de una versión según el catálogo (API-03).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stability {
    Blocked,
    New,
    Unknown,
    Observation,
    Recommended,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    /// Identificador estable: ruta del INF, ID de Windows Update o entrada del catálogo.
    pub id: String,
    pub source: Source,
    pub provider: Option<String>,
    pub version: Option<DriverVersion>,
    pub date: Option<String>,
    pub architecture: String,
    pub min_build: Option<u32>,
    /// IDs declarados por el driver, normalizados en mayúsculas.
    pub hardware_ids: Vec<String>,
    pub signature: Signature,
    /// La fuente está en la lista de fuentes oficiales o aprobadas (RC-4).
    pub approved_origin: bool,
    pub stability: Stability,
    /// Fabricante del equipo para el que se publicó, si es un driver OEM.
    pub oem: Option<String>,
    pub category_hint: Option<String>,
}

/// Qué tan bien coincide un candidato. Un número menor es más específico.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "rank", rename_all = "snake_case")]
pub enum Match {
    HardwareId(usize),
    CompatibleId(usize),
}

impl Match {
    fn key(self) -> (u8, usize) {
        match self {
            Self::HardwareId(rank) => (0, rank),
            Self::CompatibleId(rank) => (1, rank),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Context {
    pub os_build: u32,
    pub architecture: String,
    pub system_manufacturer: Option<String>,
    /// Permite instalar una versión menor que la actual (CORE-25).
    pub allow_downgrade: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recommendation {
    pub candidate: Candidate,
    pub matched: Match,
    pub reasons: Vec<String>,
    /// La versión es mayor que la instalada.
    pub is_update: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rejection {
    pub candidate_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    pub recommendation: Option<Recommendation>,
    pub rejected: Vec<Rejection>,
}

pub fn normalize(id: &str) -> String {
    id.trim().to_ascii_uppercase()
}

/// Busca el ID más específico del dispositivo que declara el candidato (CORE-10).
pub fn match_device(device: &Device, candidate: &Candidate) -> Option<Match> {
    let declared = |id: &String| {
        candidate
            .hardware_ids
            .iter()
            .any(|own| *own == normalize(id))
    };
    if let Some(rank) = device.hardware_ids.iter().position(declared) {
        return Some(Match::HardwareId(rank));
    }
    device
        .compatible_ids
        .iter()
        .position(declared)
        .map(Match::CompatibleId)
}

pub fn select(device: &Device, candidates: &[Candidate], context: &Context) -> Selection {
    let installed = device
        .driver
        .as_ref()
        .and_then(|driver| driver.version.as_deref())
        .and_then(|version| version.parse::<DriverVersion>().ok());
    let mut selection = Selection::default();
    let mut eligible = Vec::new();
    for candidate in candidates {
        let Some(matched) = match_device(device, candidate) else {
            continue;
        };
        match reject_reason(candidate, context, installed) {
            Some(reason) => selection.rejected.push(Rejection {
                candidate_id: candidate.id.clone(),
                reason,
            }),
            None => eligible.push((candidate, matched)),
        }
    }
    eligible.sort_by(|a, b| compare(a, b, context));
    if let Some((candidate, matched)) = eligible.first() {
        let is_update = match (candidate.version, installed) {
            (Some(new), Some(old)) => new > old,
            (_, None) => true,
            (None, Some(_)) => false,
        };
        selection.recommendation = Some(Recommendation {
            candidate: (*candidate).clone(),
            matched: *matched,
            reasons: explain(candidate, *matched, context, installed, eligible.len()),
            is_update,
        });
    }
    selection
}

fn reject_reason(
    candidate: &Candidate,
    context: &Context,
    installed: Option<DriverVersion>,
) -> Option<String> {
    if !candidate
        .architecture
        .eq_ignore_ascii_case(&context.architecture)
    {
        return Some(format!(
            "es para {}, el equipo es {}",
            candidate.architecture, context.architecture
        ));
    }
    if let Some(min_build) = candidate.min_build {
        if context.os_build != 0 && min_build > context.os_build {
            return Some(format!("requiere Windows build {min_build} o superior"));
        }
    }
    if candidate.signature == Signature::Invalid {
        return Some("no tiene una firma válida".into());
    }
    if !candidate.approved_origin {
        return Some("no viene de una fuente oficial o aprobada".into());
    }
    if candidate.stability == Stability::Blocked {
        return Some("esta versión está marcada como problemática".into());
    }
    if let (Some(new), Some(old)) = (candidate.version, installed) {
        if new < old && !context.allow_downgrade {
            return Some(format!("{new} es menor que la versión instalada {old}"));
        }
    }
    None
}

fn is_own_oem(candidate: &Candidate, context: &Context) -> bool {
    match (&candidate.oem, &context.system_manufacturer) {
        (Some(oem), Some(system)) => same_manufacturer(oem, system),
        _ => false,
    }
}

/// "Dell Inc." y "Dell" son el mismo fabricante.
fn same_manufacturer(a: &str, b: &str) -> bool {
    let first = |text: &str| {
        text.split(|c: char| !c.is_alphanumeric())
            .find(|word| !word.is_empty())
            .unwrap_or_default()
            .to_ascii_lowercase()
    };
    let (a, b) = (first(a), first(b));
    !a.is_empty() && a == b
}

/// Orden de preferencia: coincidencia, estabilidad, driver del fabricante del equipo (CORE-11),
/// firma comprobada, origen, versión y fecha.
fn compare(a: &(&Candidate, Match), b: &(&Candidate, Match), context: &Context) -> Ordering {
    let (ca, ma) = a;
    let (cb, mb) = b;
    ma.key()
        .cmp(&mb.key())
        .then_with(|| cb.stability.cmp(&ca.stability))
        .then_with(|| is_own_oem(cb, context).cmp(&is_own_oem(ca, context)))
        .then_with(|| (cb.signature == Signature::Valid).cmp(&(ca.signature == Signature::Valid)))
        .then_with(|| cb.source.cmp(&ca.source))
        .then_with(|| cb.version.cmp(&ca.version))
        .then_with(|| cb.date.cmp(&ca.date))
        .then_with(|| ca.id.cmp(&cb.id))
}

fn explain(
    candidate: &Candidate,
    matched: Match,
    context: &Context,
    installed: Option<DriverVersion>,
    eligible: usize,
) -> Vec<String> {
    let mut reasons = vec![match matched {
        Match::HardwareId(0) => {
            "Coincide exactamente con el Hardware ID del dispositivo.".to_owned()
        }
        Match::HardwareId(_) => "Coincide con un Hardware ID del dispositivo.".to_owned(),
        Match::CompatibleId(_) => {
            "Coincide solo con un Compatible ID: es un driver genérico para este dispositivo."
                .to_owned()
        }
    }];
    reasons.push(match candidate.stability {
        Stability::Recommended => {
            "La versión está marcada como recomendada en el catálogo.".to_owned()
        }
        Stability::Observation => {
            "La versión está en observación: todavía no hay suficiente evidencia de estabilidad."
                .to_owned()
        }
        Stability::New => "La versión es nueva y todavía no pasó el periodo de espera.".to_owned(),
        _ => "No hay datos de estabilidad para esta versión.".to_owned(),
    });
    if is_own_oem(candidate, context) {
        reasons.push("Es el driver publicado por el fabricante del equipo.".to_owned());
    }
    reasons.push(format!("Origen: {}.", candidate.source.label()));
    if candidate.signature == Signature::Valid {
        reasons.push("La firma digital es válida.".to_owned());
    }
    match (candidate.version, installed) {
        (Some(new), Some(old)) if new > old => reasons.push(format!("Actualiza de {old} a {new}.")),
        (Some(new), Some(old)) if new == old => {
            reasons.push(format!("Es la misma versión instalada ({new})."))
        }
        (Some(new), Some(old)) => {
            reasons.push(format!("Vuelve de {old} a {new} (downgrade autorizado)."))
        }
        (Some(new), None) => reasons.push(format!("Instala la versión {new}.")),
        _ => {}
    }
    if eligible > 1 {
        reasons.push(format!("Se compararon {eligible} candidatos compatibles."));
    }
    reasons
}

//! Relación entre cambios de driver y fallos (DESK-08, DESK-09, DESK-25).
//!
//! Una coincidencia de tiempo no prueba que un driver causó un fallo. Por eso se compara la
//! frecuencia de fallos antes y después del cambio, se mira si el tipo de fallo encaja con el
//! dispositivo y se informa un nivel de confianza junto con la evidencia.

use crate::db::EventRow;
use drivward_core::model::Category;
use drivward_ipc::Confidence;
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

/// Días de historia antes del cambio que forman la línea base.
pub const BASELINE_DAYS: i64 = 30;

/// Eventos que se vigilan y qué indican.
pub fn describe_event(id: u32) -> &'static str {
    match id {
        41 => "reinicio inesperado",
        1001 => "pantallazo azul",
        6008 => "apagado inesperado",
        4101 => "el driver de video dejó de responder",
        219 => "un driver no pudo cargarse",
        _ => "evento del sistema",
    }
}

fn related(event_id: u32, category: Category) -> bool {
    match event_id {
        4101 => category == Category::Display,
        41 | 1001 | 6008 | 219 => true,
        _ => false,
    }
}

/// El fallo apunta directamente al tipo de dispositivo que cambió.
fn specific(event_id: u32, category: Category) -> bool {
    event_id == 4101 && category == Category::Display
}

#[derive(Debug, Clone, PartialEq)]
pub struct Evidence {
    pub events_before: u32,
    pub events_after: u32,
    pub days_before: f64,
    pub days_after: f64,
    pub confidence: Confidence,
    pub explanation: String,
}

fn parse(text: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(text, &Rfc3339).ok()
}

pub fn evaluate(
    change_at: &str,
    category: Category,
    events: &[EventRow],
    now: OffsetDateTime,
    window_days: u32,
) -> Evidence {
    let Some(change) = parse(change_at) else {
        return Evidence {
            events_before: 0,
            events_after: 0,
            days_before: 0.0,
            days_after: 0.0,
            confidence: Confidence::None,
            explanation: "La fecha del cambio no es válida.".into(),
        };
    };
    let baseline_start = change - Duration::days(BASELINE_DAYS);
    let window_end = (change + Duration::days(window_days.max(1) as i64)).min(now);
    let mut before = 0u32;
    let mut after = 0u32;
    let mut after_specific = 0u32;
    let mut kinds = Vec::new();
    for event in events {
        let Some(at) = parse(&event.at) else { continue };
        if !related(event.event_id, category) {
            continue;
        }
        if at >= baseline_start && at < change {
            before += 1;
        } else if at >= change && at <= window_end {
            after += 1;
            if specific(event.event_id, category) {
                after_specific += 1;
            }
            let kind = describe_event(event.event_id);
            if !kinds.contains(&kind) {
                kinds.push(kind);
            }
        }
    }
    let days_before = BASELINE_DAYS as f64;
    let days_after = ((window_end - change).as_seconds_f64() / 86_400.0).max(1.0);
    let rate_before = before as f64 / days_before;
    let rate_after = after as f64 / days_after;
    // Con una línea base sin fallos se usa un piso para no dividir por cero.
    let ratio = rate_after / rate_before.max(1.0 / days_before);

    let confidence = if after == 0 || ratio < 1.5 {
        if after == 0 {
            Confidence::None
        } else {
            Confidence::Low
        }
    } else if after >= 3 && ratio >= 3.0 && after_specific > 0 {
        Confidence::High
    } else if after >= 2 && ratio >= 2.0 {
        Confidence::Medium
    } else {
        Confidence::Low
    };

    let explanation = match confidence {
        Confidence::None => format!(
            "Sin fallos relacionados desde el cambio. En los {BASELINE_DAYS} días anteriores hubo {before}."
        ),
        _ => format!(
            "Desde el cambio hubo {after} fallos ({}) en {:.0} días; en los {BASELINE_DAYS} días anteriores hubo {before}. \
             Que coincidan en el tiempo no prueba que el driver sea la causa.",
            kinds.join(", "),
            days_after
        ),
    };
    Evidence {
        events_before: before,
        events_after: after,
        days_before,
        days_after,
        confidence,
        explanation,
    }
}

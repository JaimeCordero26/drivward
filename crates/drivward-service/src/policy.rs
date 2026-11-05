//! Reglas para decidir cuándo recomendar o aplicar un cambio (DESK-12 a DESK-18).

use drivward_core::model::Category;
use drivward_ipc::{Mode, Settings};
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use time::{Date, Duration, OffsetDateTime};

pub fn is_paused(settings: &Settings, now: OffsetDateTime) -> bool {
    settings
        .paused_until
        .as_deref()
        .and_then(|text| OffsetDateTime::parse(text, &Rfc3339).ok())
        .is_some_and(|until| until > now)
}

/// Gráfica, chipset, almacenamiento y red siempre piden aprobación (DESK-18).
pub fn requires_approval(mode: Mode, category: Category) -> bool {
    match mode {
        Mode::Automatic => category.is_critical(),
        _ => true,
    }
}

/// Fecha desde la que una versión publicada en `date` (AAAA-MM-DD) puede recomendarse (DESK-12).
pub fn available_after(date: Option<&str>, quarantine_days: u32) -> Option<Date> {
    let format = format_description!("[year]-[month]-[day]");
    let published = Date::parse(date?, &format).ok()?;
    published.checked_add(Duration::days(quarantine_days as i64))
}

pub fn quarantine_passed(date: Option<&str>, quarantine_days: u32, now: OffsetDateTime) -> bool {
    match available_after(date, quarantine_days) {
        Some(available) => now.date() >= available,
        // Sin fecha no se puede saber si es nueva: se trata como recién publicada.
        None => false,
    }
}

pub struct Conditions {
    pub low_battery: bool,
    pub reboot_pending: bool,
    pub changes_in_observation: u32,
}

/// Motivo para no aplicar un cambio ahora, o `None` si se puede (DESK-14, DESK-15).
pub fn blocked_reason(conditions: &Conditions) -> Option<String> {
    if conditions.reboot_pending {
        return Some("Hay un reinicio pendiente. Reinicia antes de cambiar drivers.".into());
    }
    if conditions.low_battery {
        return Some("La batería está baja. Conecta el cargador.".into());
    }
    if conditions.changes_in_observation > 0 {
        return Some(
            "Otro cambio importante sigue en observación. Se aplica un cambio a la vez.".into(),
        );
    }
    None
}

/// La hora configurada para buscar actualizaciones ya llegó hoy y no se buscó (DESK-11).
pub fn check_due(settings: &Settings, last_check: Option<&str>, now_local: OffsetDateTime) -> bool {
    if now_local.hour() < settings.check_hour {
        return false;
    }
    match last_check.and_then(|text| OffsetDateTime::parse(text, &Rfc3339).ok()) {
        Some(last) => last.to_offset(now_local.offset()).date() < now_local.date(),
        None => true,
    }
}

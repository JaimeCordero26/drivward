//! Reporte del análisis y de las acciones en JSON y HTML (CORE-18, PORT-15).
//!
//! No incluye nombre del equipo, usuario ni número de serie (SEG-09).

use crate::classify::DeviceState;
use crate::journal::{ActionRecord, Outcome};
use crate::model::SystemInfo;
use serde::{Deserialize, Serialize};
use std::fmt::Write;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    pub generated_at: String,
    pub tool: String,
    pub system: SystemInfo,
    pub devices: Vec<DeviceReport>,
    pub actions: Vec<ActionRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceReport {
    pub name: String,
    pub class: Option<String>,
    pub hardware_id: Option<String>,
    pub state: DeviceState,
    pub current_version: Option<String>,
    pub current_provider: Option<String>,
    pub recommended_version: Option<String>,
    pub recommended_source: Option<String>,
    pub reasons: Vec<String>,
}

impl Report {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into())
    }

    pub fn to_html(&self) -> String {
        let mut html = String::new();
        let system = &self.system;
        let model = [system.manufacturer.as_deref(), system.model.as_deref()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ");
        let _ = write!(
            html,
            r#"<!doctype html><html lang="es"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Reporte de drivers · Drivward</title><style>
body{{font-family:system-ui,Segoe UI,sans-serif;margin:2rem auto;max-width:1100px;padding:0 1rem;color:#111}}
h1{{margin-bottom:.2rem}}.muted{{color:#666}}table{{border-collapse:collapse;width:100%;margin:1rem 0 2rem;font-size:.9rem}}
th,td{{border-bottom:1px solid #ddd;padding:.5rem;text-align:left;vertical-align:top}}th{{background:#f4f4f5}}
.s-missing_driver,.s-error{{color:#b91c1c;font-weight:600}}.s-updatable{{color:#1d4ed8;font-weight:600}}.s-ok{{color:#15803d}}
ul{{margin:0;padding-left:1rem}}code{{font-size:.8rem}}
</style></head><body><h1>Reporte de drivers</h1><p class="muted">Generado {} con {}</p>
<h2>Equipo</h2><table><tr><th>Modelo</th><td>{}</td></tr><tr><th>Windows</th><td>{} · build {} · {}</td></tr>
<tr><th>Procesador</th><td>{}</td></tr><tr><th>Gráfica</th><td>{}</td></tr><tr><th>BIOS</th><td>{}</td></tr></table>"#,
            escape(&self.generated_at),
            escape(&self.tool),
            escape(&model),
            escape(&system.os_name),
            system.os_build,
            escape(&system.architecture),
            escape(system.cpu.as_deref().unwrap_or("-")),
            escape(&system.gpus.join(", ")),
            escape(system.bios_version.as_deref().unwrap_or("-")),
        );
        html.push_str("<h2>Dispositivos</h2><table><tr><th>Dispositivo</th><th>Estado</th><th>Actual</th><th>Recomendado</th><th>Por qué</th></tr>");
        for device in &self.devices {
            let state = serde_json::to_value(device.state)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default();
            let _ = write!(
                html,
                r#"<tr><td>{}<br><code class="muted">{}</code></td><td class="s-{}">{}</td><td>{}</td><td>{}</td><td><ul>{}</ul></td></tr>"#,
                escape(&device.name),
                escape(device.hardware_id.as_deref().unwrap_or("")),
                state,
                device.state.label(),
                escape(device.current_version.as_deref().unwrap_or("-")),
                escape(
                    &match (&device.recommended_version, &device.recommended_source) {
                        (Some(v), Some(s)) => format!("{v} ({s})"),
                        (Some(v), None) => v.clone(),
                        _ => "-".into(),
                    }
                ),
                device
                    .reasons
                    .iter()
                    .map(|r| format!("<li>{}</li>", escape(r)))
                    .collect::<String>(),
            );
        }
        html.push_str("</table><h2>Acciones</h2>");
        if self.actions.is_empty() {
            html.push_str("<p class=\"muted\">No se hicieron cambios.</p>");
        } else {
            html.push_str("<table><tr><th>Fecha</th><th>Acción</th><th>Dispositivo</th><th>Versión anterior</th><th>Versión nueva</th><th>Resultado</th></tr>");
            for action in &self.actions {
                let outcome = match &action.outcome {
                    Outcome::Success => "Correcto".to_owned(),
                    Outcome::RebootRequired => "Correcto, requiere reinicio".to_owned(),
                    Outcome::Skipped(why) => format!("Omitido: {why}"),
                    Outcome::Failed(why) => format!("Falló: {why}"),
                };
                let kind = serde_json::to_value(action.action)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_default();
                let _ = write!(
                    html,
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    escape(&action.timestamp),
                    escape(&kind),
                    escape(action.device_name.as_deref().unwrap_or("-")),
                    escape(action.previous_version.as_deref().unwrap_or("-")),
                    escape(action.new_version.as_deref().unwrap_or("-")),
                    escape(&outcome),
                );
            }
            html.push_str("</table>");
        }
        html.push_str(
            "<p class=\"muted\">Drivward es software libre bajo GPL-3.0.</p></body></html>\n",
        );
        html
    }
}

pub fn escape(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            _ => output.push(character),
        }
    }
    output
}

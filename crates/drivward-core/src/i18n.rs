//! Textos de la interfaz. El español es el idioma base; la estructura admite inglés (CORE-27).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    Spanish,
    English,
}

impl Language {
    /// Elige el idioma a partir de un código como `es-CR` o `en-US`.
    pub fn from_code(code: &str) -> Self {
        match code.to_ascii_lowercase().split(['-', '_']).next() {
            Some("en") => Self::English,
            _ => Self::Spanish,
        }
    }
}

/// Cada entrada es `(clave, español, inglés)`.
const MESSAGES: &[(&str, &str, &str)] = &[
    ("app.title", "Drivward", "Drivward"),
    ("state.missing_driver", "Falta driver", "Missing driver"),
    ("state.error", "Con error", "Error"),
    ("state.updatable", "Actualizable", "Update available"),
    ("state.ok", "Correcto", "OK"),
    ("state.unknown", "Desconocido", "Unknown"),
    (
        "net.online",
        "Conectado a internet",
        "Connected to the internet",
    ),
    ("net.offline", "Sin internet", "No internet"),
    (
        "net.portal",
        "La red pide iniciar sesión en el navegador",
        "The network requires signing in from a browser",
    ),
    ("report.title", "Reporte de drivers", "Driver report"),
];

pub fn text(language: Language, key: &str) -> &str {
    MESSAGES
        .iter()
        .find(|(id, _, _)| *id == key)
        .map(|(_, es, en)| match language {
            Language::Spanish => *es,
            Language::English => *en,
        })
        .unwrap_or(key)
}

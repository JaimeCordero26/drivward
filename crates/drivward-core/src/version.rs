//! Versiones de driver con el formato de Windows `a.b.c.d`.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

/// Hasta cuatro partes numéricas. Las partes que faltan valen 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DriverVersion([u32; 4]);

impl DriverVersion {
    pub fn new(a: u32, b: u32, c: u32, d: u32) -> Self {
        Self([a, b, c, d])
    }

    pub fn parts(&self) -> [u32; 4] {
        self.0
    }
}

impl FromStr for DriverVersion {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        let mut parts = [0u32; 4];
        for (count, piece) in text.split('.').enumerate() {
            if count == 4 {
                return Err(format!("versión con más de cuatro partes: {text}"));
            }
            parts[count] = piece
                .trim()
                .parse()
                .map_err(|_| format!("versión inválida: {text}"))?;
        }
        Ok(Self(parts))
    }
}

impl TryFrom<String> for DriverVersion {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<DriverVersion> for String {
    fn from(version: DriverVersion) -> Self {
        version.to_string()
    }
}

impl fmt::Display for DriverVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [a, b, c, d] = self.0;
        write!(f, "{a}.{b}.{c}.{d}")
    }
}

impl Ord for DriverVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for DriverVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Lee una versión que puede faltar o estar mal escrita.
pub fn parse_optional(text: Option<&str>) -> Option<DriverVersion> {
    text.and_then(|value| value.parse().ok())
}

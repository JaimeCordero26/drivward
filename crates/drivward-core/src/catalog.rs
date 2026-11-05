//! Catálogo remoto firmado (API-02 a API-05, SEG-02, SEG-05).
//!
//! El catálogo publica datos de drivers, no archivos: cada entrada apunta a la descarga oficial
//! y trae el hash para verificarla. El cliente elige localmente, sin enviar su inventario.

use crate::select::{normalize, Candidate, Signature, Source, Stability};
use crate::version::DriverVersion;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signature as EdSignature, VerifyingKey};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("catálogo inválido: {0}")]
    Json(#[from] serde_json::Error),
    #[error("la firma no está en base64")]
    Encoding,
    #[error("la firma o la clave pública tienen un tamaño incorrecto")]
    Length,
    #[error("la firma del catálogo no es válida")]
    BadSignature,
    #[error("versión de esquema no soportada: {0}")]
    UnsupportedSchema(u32),
    #[error("la entrada {0} no usa HTTPS")]
    InsecureUrl(String),
}

/// Archivo publicado: el JSON del catálogo tal cual se firmó y su firma Ed25519.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedCatalog {
    pub payload: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Catalog {
    pub schema_version: u32,
    /// Proveedor del fragmento, por ejemplo `intel` o `dell`.
    pub shard: String,
    /// AAAA-MM-DD.
    pub generated_at: String,
    pub entries: Vec<CatalogEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    pub provider: String,
    pub source: Source,
    pub version: DriverVersion,
    pub date: String,
    pub architecture: String,
    pub min_build: Option<u32>,
    pub hardware_ids: Vec<String>,
    /// Modelos de equipo para los que el fabricante publicó el driver.
    #[serde(default)]
    pub models: Vec<String>,
    pub oem: Option<String>,
    pub url: String,
    pub sha256: String,
    pub license: String,
    pub status: Stability,
}

/// Comprueba la firma y devuelve el catálogo. Nada del contenido se usa antes de verificarla.
pub fn verify(signed: &SignedCatalog, public_key: &[u8; 32]) -> Result<Catalog, CatalogError> {
    let key = VerifyingKey::from_bytes(public_key).map_err(|_| CatalogError::Length)?;
    let bytes = STANDARD
        .decode(signed.signature.trim())
        .map_err(|_| CatalogError::Encoding)?;
    let signature = EdSignature::from_slice(&bytes).map_err(|_| CatalogError::Length)?;
    key.verify_strict(signed.payload.as_bytes(), &signature)
        .map_err(|_| CatalogError::BadSignature)?;
    let catalog: Catalog = serde_json::from_str(&signed.payload)?;
    if catalog.schema_version != SCHEMA_VERSION {
        return Err(CatalogError::UnsupportedSchema(catalog.schema_version));
    }
    if let Some(entry) = catalog
        .entries
        .iter()
        .find(|e| !e.url.starts_with("https://"))
    {
        return Err(CatalogError::InsecureUrl(entry.id.clone()));
    }
    Ok(catalog)
}

impl Catalog {
    pub fn candidates(&self) -> Vec<Candidate> {
        self.entries
            .iter()
            .map(|entry| Candidate {
                id: entry.id.clone(),
                source: entry.source,
                provider: Some(entry.provider.clone()),
                version: Some(entry.version),
                date: Some(entry.date.clone()),
                architecture: entry.architecture.clone(),
                min_build: entry.min_build,
                hardware_ids: entry.hardware_ids.iter().map(|id| normalize(id)).collect(),
                // La firma del paquete se comprueba al descargarlo, antes de instalar.
                signature: Signature::Unchecked,
                approved_origin: true,
                stability: entry.status,
                oem: entry.oem.clone(),
                category_hint: None,
            })
            .collect()
    }

    /// Versiones que no deben elegirse ni reinstalarse (CORE-23, DESK-13).
    pub fn blocked(&self) -> impl Iterator<Item = &CatalogEntry> {
        self.entries
            .iter()
            .filter(|e| e.status == Stability::Blocked)
    }
}

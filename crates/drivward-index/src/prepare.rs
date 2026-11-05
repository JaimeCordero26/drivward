//! Preparador de la USB (DIST-03, DIST-04): descarga los paquetes oficiales en la compu del
//! usuario, verifica su hash, los extrae en `drivers/` y registra origen y licencia.
//!
//! Los drivers nunca se guardan en el repositorio (DIST-02).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Component, Path};

pub const SOURCES_FILE: &str = "fuentes.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub packages: Vec<Source>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    /// Carpeta dentro de `drivers/`, por ejemplo `red/wifi`.
    pub category: String,
    pub vendor: String,
    pub description: String,
    pub url: String,
    pub sha256: String,
    pub license: String,
    /// La licencia permite incluir el paquete en una USB publicada (DIST-05, RC-5).
    pub redistributable: bool,
}

#[derive(Debug)]
pub struct Prepared {
    pub id: String,
    pub result: Result<(), String>,
}

fn safe_relative(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}

pub fn validate(manifest: &Manifest) -> Result<(), String> {
    for source in &manifest.packages {
        if !source.url.starts_with("https://") {
            return Err(format!(
                "{}: la descarga debe usar HTTPS (SEG-05)",
                source.id
            ));
        }
        if source.sha256.len() != 64 || !source.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("{}: falta el SHA-256 de 64 caracteres", source.id));
        }
        if !safe_relative(&source.category) || !safe_relative(&source.id) {
            return Err(format!(
                "{}: la carpeta o el id no son rutas seguras",
                source.id
            ));
        }
        if source.license.trim().is_empty() {
            return Err(format!("{}: falta la licencia (DIST-04)", source.id));
        }
    }
    Ok(())
}

fn hash_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

fn download(url: &str, destination: &Path) -> Result<(), String> {
    let response = ureq::get(url)
        .call()
        .map_err(|e| format!("no se pudo descargar: {e}"))?;
    let mut reader = response.into_body().into_reader();
    let mut file = File::create(destination).map_err(|e| e.to_string())?;
    io::copy(&mut reader, &mut file).map_err(|e| format!("descarga interrumpida: {e}"))?;
    file.flush().map_err(|e| e.to_string())
}

/// Coloca un paquete ya descargado: comprueba el hash y extrae el ZIP.
pub fn place(source: &Source, archive: &Path, library: &Path) -> Result<(), String> {
    let actual = hash_file(archive).map_err(|e| e.to_string())?;
    if !actual.eq_ignore_ascii_case(&source.sha256) {
        return Err(format!(
            "el SHA-256 no coincide: se esperaba {}, llegó {actual}",
            source.sha256
        ));
    }
    let target = library.join(&source.category).join(&source.id);
    if target.exists() {
        fs::remove_dir_all(&target).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    let file = File::open(archive).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("no es un ZIP válido: {e}"))?;
    // `extract` rechaza rutas que salgan de la carpeta destino.
    zip.extract(&target)
        .map_err(|e| format!("no se pudo extraer: {e}"))
}

/// Descarga y coloca cada paquete, y guarda `fuentes.json` con su origen y licencia.
pub fn prepare(
    manifest: &Manifest,
    library: &Path,
    only_redistributable: bool,
) -> Result<Vec<Prepared>, String> {
    validate(manifest)?;
    fs::create_dir_all(library).map_err(|e| e.to_string())?;
    let downloads = library.join(".descargas");
    fs::create_dir_all(&downloads).map_err(|e| e.to_string())?;
    let mut results = Vec::new();
    let mut placed = Vec::new();
    for source in &manifest.packages {
        if only_redistributable && !source.redistributable {
            results.push(Prepared {
                id: source.id.clone(),
                result: Err("la licencia no permite redistribuirlo".into()),
            });
            continue;
        }
        let archive = downloads.join(format!("{}.zip", source.id));
        let result =
            download(&source.url, &archive).and_then(|()| place(source, &archive, library));
        let _ = fs::remove_file(&archive);
        if result.is_ok() {
            placed.push(source.clone());
        }
        results.push(Prepared {
            id: source.id.clone(),
            result,
        });
    }
    let _ = fs::remove_dir_all(&downloads);
    let record = Manifest {
        schema_version: manifest.schema_version,
        packages: placed,
    };
    let text = serde_json::to_string_pretty(&record).map_err(|e| e.to_string())?;
    // Se guarda junto a la carpeta de drivers para que no forme parte del índice.
    let sources = library.parent().unwrap_or(library).join(SOURCES_FILE);
    fs::write(sources, text).map_err(|e| e.to_string())?;
    Ok(results)
}

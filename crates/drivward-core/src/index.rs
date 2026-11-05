//! Generación y validación de `index.json`.

use crate::inf::{self, DriverInf};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use thiserror::Error;
use walkdir::WalkDir;

pub const SCHEMA_VERSION: u32 = 1;
pub const INDEX_FILE: &str = "index.json";
const MAX_INF_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum IndexError {
    #[error("{path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("{path}: {source}")]
    Walk {
        path: PathBuf,
        source: walkdir::Error,
    },
    #[error("index.json inválido: {0}")]
    Json(#[from] serde_json::Error),
    #[error("versión de esquema no soportada: {0}")]
    UnsupportedSchema(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Index {
    pub schema_version: u32,
    pub packages: Vec<Package>,
    /// Archivos INF que no se pudieron incluir y por qué.
    pub rejected: Vec<Rejected>,
    pub files: BTreeMap<String, FileEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Package {
    pub inf: String,
    #[serde(flatten)]
    pub driver: DriverInf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rejected {
    pub inf: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Verification {
    pub missing: Vec<String>,
    pub changed: Vec<String>,
    pub unexpected: Vec<String>,
    pub unsafe_paths: Vec<String>,
}

impl Verification {
    pub fn is_valid(&self) -> bool {
        self.missing.is_empty()
            && self.changed.is_empty()
            && self.unexpected.is_empty()
            && self.unsafe_paths.is_empty()
    }
}

pub fn build(root: &Path) -> Result<Index, IndexError> {
    let mut index = Index {
        schema_version: SCHEMA_VERSION,
        packages: Vec::new(),
        rejected: Vec::new(),
        files: BTreeMap::new(),
    };
    for (relative, path) in list_files(root)? {
        index.files.insert(relative.clone(), describe(&path)?);
        if !relative.to_lowercase().ends_with(".inf") {
            continue;
        }
        match read_inf(&path)? {
            Ok(driver) => index.packages.push(Package {
                inf: relative,
                driver,
            }),
            Err(reason) => index.rejected.push(Rejected {
                inf: relative,
                reason,
            }),
        }
    }
    Ok(index)
}

pub fn write(root: &Path, index: &Index) -> Result<(), IndexError> {
    let path = root.join(INDEX_FILE);
    let mut text = serde_json::to_string_pretty(index)?;
    text.push('\n');
    fs::write(&path, text).map_err(|source| IndexError::Io { path, source })
}

pub fn load(root: &Path) -> Result<Index, IndexError> {
    let path = root.join(INDEX_FILE);
    let text = fs::read_to_string(&path).map_err(|source| IndexError::Io { path, source })?;
    let index: Index = serde_json::from_str(&text)?;
    if index.schema_version != SCHEMA_VERSION {
        return Err(IndexError::UnsupportedSchema(index.schema_version));
    }
    Ok(index)
}

/// Compara el índice con la carpeta. Nunca abre rutas que salgan de `root`.
pub fn verify(root: &Path, index: &Index) -> Result<Verification, IndexError> {
    let mut result = Verification::default();
    let actual = list_files(root)?;
    for (relative, expected) in &index.files {
        if !is_safe_relative(relative) {
            result.unsafe_paths.push(relative.clone());
            continue;
        }
        match actual.get(relative) {
            None => result.missing.push(relative.clone()),
            Some(path) if describe(path)? != *expected => result.changed.push(relative.clone()),
            Some(_) => {}
        }
    }
    result.unexpected = actual
        .keys()
        .filter(|path| !index.files.contains_key(*path))
        .cloned()
        .collect();
    Ok(result)
}

/// Comprueba solo los archivos de una carpeta del índice, justo antes de instalar (PORT-17).
pub fn verify_folder(root: &Path, index: &Index, folder: &str) -> Result<Verification, IndexError> {
    let prefix = format!("{}/", folder.trim_end_matches('/'));
    let mut result = Verification::default();
    for (relative, expected) in index.files.range(prefix.clone()..) {
        if !relative.starts_with(&prefix) {
            break;
        }
        if !is_safe_relative(relative) {
            result.unsafe_paths.push(relative.clone());
            continue;
        }
        let path = root.join(relative);
        if !path.is_file() {
            result.missing.push(relative.clone());
        } else if describe(&path)? != *expected {
            result.changed.push(relative.clone());
        }
    }
    let actual = list_files(&root.join(folder))?;
    for name in actual.keys() {
        let relative = format!("{prefix}{name}");
        if !index.files.contains_key(&relative) {
            result.unexpected.push(relative);
        }
    }
    Ok(result)
}

fn list_files(root: &Path) -> Result<BTreeMap<String, PathBuf>, IndexError> {
    let mut files = BTreeMap::new();
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(|source| IndexError::Walk {
            path: root.to_path_buf(),
            source,
        })?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(root)
            .expect("WalkDir devuelve rutas dentro de root");
        let relative = relative
            .components()
            .map(|part| part.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        if relative != INDEX_FILE {
            files.insert(relative, entry.into_path());
        }
    }
    Ok(files)
}

fn read_inf(path: &Path) -> Result<Result<DriverInf, String>, IndexError> {
    let io_error = |source| IndexError::Io {
        path: path.to_path_buf(),
        source,
    };
    let size = fs::metadata(path).map_err(io_error)?.len();
    if size > MAX_INF_BYTES {
        return Ok(Err(format!("supera el límite de {MAX_INF_BYTES} bytes")));
    }
    let bytes = fs::read(path).map_err(io_error)?;
    Ok(inf::parse_bytes(&bytes).map_err(|error| error.to_string()))
}

fn describe(path: &Path) -> Result<FileEntry, IndexError> {
    let io_error = |source| IndexError::Io {
        path: path.to_path_buf(),
        source,
    };
    let mut file = File::open(path).map_err(io_error)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    let mut size = 0;
    loop {
        let read = file.read(&mut buffer).map_err(io_error)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        size += read as u64;
    }
    let sha256 = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(FileEntry { size, sha256 })
}

fn is_safe_relative(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && !path.contains(':')
        && Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

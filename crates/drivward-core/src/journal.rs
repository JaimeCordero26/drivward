//! Registro de cada acción sobre drivers (CORE-17), en JSON Lines con rotación (RNF-09).

use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

const MAX_BYTES: u64 = 5 * 1024 * 1024;
const KEEP: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Install,
    Backup,
    RestorePoint,
    Rollback,
    RemoveDriver,
    HideUpdate,
    UnhideUpdate,
    Verify,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", content = "detail", rename_all = "snake_case")]
pub enum Outcome {
    Success,
    RebootRequired,
    Skipped(String),
    Failed(String),
}

impl Outcome {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success | Self::RebootRequired)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionRecord {
    pub timestamp: String,
    pub action: ActionKind,
    pub device_id: Option<String>,
    pub device_name: Option<String>,
    pub previous_version: Option<String>,
    pub new_version: Option<String>,
    /// INF, ID de Windows Update o carpeta, según la acción.
    pub target: Option<String>,
    pub outcome: Outcome,
    /// Salida del comando, recortada.
    #[serde(default)]
    pub output: String,
}

impl ActionRecord {
    pub fn new(action: ActionKind, outcome: Outcome) -> Self {
        Self {
            timestamp: now(),
            action,
            device_id: None,
            device_name: None,
            previous_version: None,
            new_version: None,
            target: None,
            outcome,
            output: String::new(),
        }
    }
}

pub fn now() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_default()
}

/// Recorta la salida de un comando para que el registro no crezca sin control.
pub fn trim_output(text: &str) -> String {
    const LIMIT: usize = 4000;
    let text = text.trim();
    if text.len() <= LIMIT {
        return text.to_owned();
    }
    let mut end = LIMIT;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

pub struct Journal {
    path: PathBuf,
}

impl Journal {
    pub fn open(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn append(&self, record: &ActionRecord) -> io::Result<()> {
        self.rotate_if_needed()?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let line = serde_json::to_string(record).map_err(io::Error::other)?;
        writeln!(file, "{line}")?;
        file.sync_data()
    }

    /// Lee las entradas del archivo actual. Las líneas dañadas se ignoran.
    pub fn read(&self) -> io::Result<Vec<ActionRecord>> {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error),
        };
        Ok(BufReader::new(file)
            .lines()
            .map_while(Result::ok)
            .filter_map(|line| serde_json::from_str(&line).ok())
            .collect())
    }

    fn rotated(&self, number: usize) -> PathBuf {
        let mut name = self.path.as_os_str().to_owned();
        name.push(format!(".{number}"));
        PathBuf::from(name)
    }

    fn rotate_if_needed(&self) -> io::Result<()> {
        match fs::metadata(&self.path) {
            Ok(meta) if meta.len() >= MAX_BYTES => {}
            _ => return Ok(()),
        }
        let _ = fs::remove_file(self.rotated(KEEP));
        for number in (1..KEEP).rev() {
            let from = self.rotated(number);
            if from.exists() {
                fs::rename(&from, self.rotated(number + 1))?;
            }
        }
        fs::rename(&self.path, self.rotated(1))
    }
}

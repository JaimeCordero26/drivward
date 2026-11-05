//! Analizador de archivos INF de drivers de Windows.

use encoding_rs::{UTF_16BE, UTF_16LE, WINDOWS_1252};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum InfError {
    #[error("falta la sección [Version]")]
    MissingVersion,
    #[error("falta la sección [Manufacturer]")]
    MissingManufacturer,
    #[error("DriverVer inválido: {0}")]
    InvalidDriverVer(String),
    #[error("el INF no declara dispositivos")]
    NoDevices,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverInf {
    pub provider: Option<String>,
    pub class: Option<String>,
    pub class_guid: Option<String>,
    pub catalog_file: Option<String>,
    /// Fecha en formato AAAA-MM-DD.
    pub date: Option<String>,
    pub version: Option<String>,
    pub targets: Vec<Target>,
}

/// Dispositivos de una sección Models para una plataforma concreta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    pub manufacturer: String,
    pub architecture: String,
    pub min_os: Option<String>,
    pub min_build: Option<u32>,
    pub product_type: Option<String>,
    pub devices: Vec<Device>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    pub description: String,
    pub hardware_id: String,
    pub compatible_ids: Vec<String>,
}

#[derive(Debug, Default)]
struct Section {
    entries: Vec<(Option<String>, Vec<String>)>,
}

#[derive(Debug, Default)]
struct RawInf {
    sections: HashMap<String, Section>,
    strings: HashMap<String, String>,
}

/// Convierte los bytes de un INF a texto. Soporta UTF-16, UTF-8 y ANSI.
pub fn decode(bytes: &[u8]) -> String {
    let (encoding, body) = match bytes {
        [0xFF, 0xFE, rest @ ..] => (Some(UTF_16LE), rest),
        [0xFE, 0xFF, rest @ ..] => (Some(UTF_16BE), rest),
        [0xEF, 0xBB, 0xBF, rest @ ..] => (None, rest),
        [a, 0, ..] if *a != 0 => (Some(UTF_16LE), bytes),
        [0, b, ..] if *b != 0 => (Some(UTF_16BE), bytes),
        _ => (None, bytes),
    };
    if let Some(encoding) = encoding {
        return encoding.decode_without_bom_handling(body).0.into_owned();
    }
    match std::str::from_utf8(body) {
        Ok(text) => text.to_owned(),
        Err(_) => WINDOWS_1252
            .decode_without_bom_handling(body)
            .0
            .into_owned(),
    }
}

pub fn parse_bytes(bytes: &[u8]) -> Result<DriverInf, InfError> {
    parse(&decode(bytes))
}

pub fn parse(text: &str) -> Result<DriverInf, InfError> {
    let raw = read_sections(text);
    let version = raw
        .sections
        .get("version")
        .ok_or(InfError::MissingVersion)?;
    let manufacturer = raw
        .sections
        .get("manufacturer")
        .ok_or(InfError::MissingManufacturer)?;

    let (date, driver_version) = match version_value(version, "driverver") {
        Some(fields) => parse_driver_ver(&raw, fields)?,
        None => (None, None),
    };

    let mut targets = Vec::new();
    for (key, values) in &manufacturer.entries {
        let manufacturer_name = key
            .as_deref()
            .map(|name| raw.resolve(name))
            .unwrap_or_default();
        let Some(models) = values.first() else {
            continue;
        };
        let models = raw.resolve(models);
        for decoration in decorations(&values[1..]) {
            let section_name = match decoration.is_empty() {
                true => models.to_lowercase(),
                false => format!("{}.{}", models.to_lowercase(), decoration.to_lowercase()),
            };
            let Some(section) = raw.sections.get(&section_name) else {
                continue;
            };
            let devices = read_devices(&raw, section);
            if !devices.is_empty() {
                targets.push(build_target(&manufacturer_name, &decoration, devices));
            }
        }
    }
    if targets.is_empty() {
        return Err(InfError::NoDevices);
    }

    let value = |key| {
        version_value(version, key)
            .and_then(|fields| fields.first())
            .map(|v| raw.resolve(v))
    };
    Ok(DriverInf {
        provider: value("provider"),
        class: value("class"),
        class_guid: value("classguid"),
        catalog_file: value("catalogfile.ntamd64").or_else(|| value("catalogfile")),
        date,
        version: driver_version,
        targets,
    })
}

fn read_sections(text: &str) -> RawInf {
    let mut raw = RawInf::default();
    let mut current: Option<String> = None;
    for line in logical_lines(text) {
        let line = line.trim();
        if let Some(name) = section_header(line) {
            let name = name.to_lowercase();
            raw.sections.entry(name.clone()).or_default();
            current = Some(name);
        } else if let Some(name) = &current {
            if let Some(section) = raw.sections.get_mut(name) {
                section.entries.push(split_entry(line));
            }
        }
    }
    raw.strings = read_strings(&raw.sections);
    raw
}

/// Une líneas terminadas en `\` y elimina comentarios y líneas vacías.
fn logical_lines(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut pending = String::new();
    let mut quoted = false;
    for line in text.lines() {
        let stripped = strip_comment(line, &mut quoted);
        let trimmed = stripped.trim_end();
        match trimmed.strip_suffix('\\') {
            Some(start) => {
                pending.push_str(start);
                pending.push(' ');
            }
            None => {
                pending.push_str(trimmed);
                if !pending.trim().is_empty() {
                    lines.push(std::mem::take(&mut pending));
                }
                pending.clear();
                quoted = false;
            }
        }
    }
    if !pending.trim().is_empty() {
        lines.push(pending);
    }
    lines
}

/// `quoted` se conserva entre líneas unidas con `\\`.
fn strip_comment<'a>(line: &'a str, quoted: &mut bool) -> &'a str {
    for (index, character) in line.char_indices() {
        match character {
            '"' => *quoted = !*quoted,
            ';' if !*quoted => return &line[..index],
            _ => {}
        }
    }
    line
}

fn section_header(line: &str) -> Option<&str> {
    line.strip_prefix('[')?.split(']').next().map(str::trim)
}

fn split_entry(line: &str) -> (Option<String>, Vec<String>) {
    match find_unquoted(line, '=') {
        Some(index) => (
            Some(clean(&line[..index])),
            split_fields(&line[index + 1..]),
        ),
        None => (None, split_fields(line)),
    }
}

fn find_unquoted(text: &str, target: char) -> Option<usize> {
    let mut quoted = false;
    for (index, character) in text.char_indices() {
        if character == '"' {
            quoted = !quoted;
        } else if character == target && !quoted {
            return Some(index);
        }
    }
    None
}

fn split_fields(text: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut rest = text;
    while let Some(index) = find_unquoted(rest, ',') {
        fields.push(clean(&rest[..index]));
        rest = &rest[index + 1..];
    }
    fields.push(clean(rest));
    fields
}

fn clean(value: &str) -> String {
    let value = value.trim();
    match value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
        Some(inner) => inner.replace("\"\"", "\""),
        None => value.to_owned(),
    }
}

/// `[Strings]` es la base; `[Strings.0409]` aporta el inglés si falta algo.
fn read_strings(sections: &HashMap<String, Section>) -> HashMap<String, String> {
    let mut strings = HashMap::new();
    let mut names: Vec<_> = sections
        .keys()
        .filter(|name| name.as_str() == "strings" || name.starts_with("strings."))
        .collect();
    names.sort_by_key(|name| match name.as_str() {
        "strings" => 0,
        "strings.0409" => 1,
        _ => 2,
    });
    for name in names {
        for (key, values) in &sections[name.as_str()].entries {
            if let (Some(key), Some(value)) = (key, values.first()) {
                strings
                    .entry(key.to_lowercase())
                    .or_insert_with(|| value.clone());
            }
        }
    }
    strings
}

impl RawInf {
    /// Sustituye tokens `%nombre%`. Un token desconocido se conserva.
    fn resolve(&self, value: &str) -> String {
        let mut output = String::new();
        let mut rest = value;
        while let Some(start) = rest.find('%') {
            output.push_str(&rest[..start]);
            let after = &rest[start + 1..];
            let Some(end) = after.find('%') else {
                output.push_str(&rest[start..]);
                return output;
            };
            let key = &after[..end];
            if key.is_empty() {
                output.push('%');
            } else {
                match self.strings.get(&key.to_lowercase()) {
                    Some(text) => output.push_str(text),
                    None => {
                        output.push('%');
                        output.push_str(key);
                        output.push('%');
                    }
                }
            }
            rest = &after[end + 1..];
        }
        output.push_str(rest);
        output
    }
}

fn version_value<'a>(section: &'a Section, key: &str) -> Option<&'a Vec<String>> {
    section
        .entries
        .iter()
        .find(|(name, _)| name.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(key)))
        .map(|(_, values)| values)
}

fn parse_driver_ver(
    raw: &RawInf,
    fields: &[String],
) -> Result<(Option<String>, Option<String>), InfError> {
    if fields.iter().all(|field| field.is_empty()) {
        return Ok((None, None));
    }
    let text = fields.join(",");
    let invalid = || InfError::InvalidDriverVer(text.clone());
    let date = raw.resolve(fields.first().ok_or_else(invalid)?);
    let parts: Vec<_> = date.split('/').collect();
    let [month, day, year] = parts.as_slice() else {
        return Err(invalid());
    };
    let number = |part: &&str| part.parse::<u32>().map_err(|_| invalid());
    let (month, day, year) = (number(month)?, number(day)?, number(year)?);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || year < 1980 {
        return Err(invalid());
    }
    let version = fields
        .get(1)
        .map(|v| raw.resolve(v))
        .filter(|v| !v.is_empty());
    Ok((Some(format!("{year:04}-{month:02}-{day:02}")), version))
}

fn decorations(values: &[String]) -> Vec<String> {
    match values.is_empty() {
        true => vec![String::new()],
        false => values.iter().filter(|v| !v.is_empty()).cloned().collect(),
    }
}

fn read_devices(raw: &RawInf, section: &Section) -> Vec<Device> {
    let mut devices = Vec::new();
    let mut seen = BTreeSet::new();
    for (key, values) in &section.entries {
        let Some(key) = key else { continue };
        let Some(hardware_id) = values.get(1).filter(|id| !id.is_empty()) else {
            continue;
        };
        let hardware_id = normalize_id(&raw.resolve(hardware_id));
        let compatible_ids: Vec<_> = values[2..]
            .iter()
            .filter(|id| !id.is_empty())
            .map(|id| normalize_id(&raw.resolve(id)))
            .collect();
        if seen.insert((hardware_id.clone(), compatible_ids.clone())) {
            devices.push(Device {
                description: raw.resolve(key),
                hardware_id,
                compatible_ids,
            });
        }
    }
    devices
}

pub fn normalize_id(id: &str) -> String {
    id.trim().to_uppercase()
}

/// Sin decoración de plataforma, o con `NT`, una sección Models aplica a x86.
fn build_target(manufacturer: &str, decoration: &str, devices: Vec<Device>) -> Target {
    let mut parts = decoration.split('.');
    let platform = parts.next().unwrap_or_default().to_lowercase();
    let architecture = match platform.as_str() {
        "ntamd64" => "x64",
        "ntarm64" => "arm64",
        "ntarm" => "arm",
        "ntia64" => "ia64",
        "" | "nt" | "ntx86" => "x86",
        _ => "unknown",
    };
    let major = parts.next().filter(|v| !v.is_empty());
    let minor = parts.next().filter(|v| !v.is_empty());
    let product_type = parts.next().filter(|v| !v.is_empty());
    let _suite = parts.next();
    let min_build = parts.next().and_then(|v| v.parse().ok());
    Target {
        manufacturer: manufacturer.to_owned(),
        architecture: architecture.to_owned(),
        min_os: major.map(|major| format!("{major}.{}", minor.unwrap_or("0"))),
        min_build,
        product_type: product_type.map(str::to_owned),
        devices,
    }
}

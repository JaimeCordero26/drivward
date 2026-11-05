//! Convierte el índice de la librería local en candidatos para la selección.

use crate::index::Index;
use crate::select::{normalize, Candidate, Signature, Source, Stability};
use crate::version::parse_optional;

/// Un candidato por INF y arquitectura. La librería solo contiene fuentes aprobadas (DIST-04).
pub fn candidates(index: &Index) -> Vec<Candidate> {
    let mut output = Vec::new();
    for package in &index.packages {
        let driver = &package.driver;
        let mut architectures: Vec<&str> = driver
            .targets
            .iter()
            .map(|t| t.architecture.as_str())
            .collect();
        architectures.sort_unstable();
        architectures.dedup();
        for architecture in architectures {
            let targets: Vec<_> = driver
                .targets
                .iter()
                .filter(|t| t.architecture == architecture)
                .collect();
            // Si alguna sección no pide build mínima, el driver sirve desde cualquiera.
            let min_build = match targets.iter().any(|t| t.min_build.is_none()) {
                true => None,
                false => targets.iter().filter_map(|t| t.min_build).min(),
            };
            let mut hardware_ids = Vec::new();
            for target in &targets {
                for device in &target.devices {
                    hardware_ids.push(normalize(&device.hardware_id));
                    hardware_ids.extend(device.compatible_ids.iter().map(|id| normalize(id)));
                }
            }
            hardware_ids.sort();
            hardware_ids.dedup();
            output.push(Candidate {
                id: package.inf.clone(),
                source: Source::Library,
                provider: driver.provider.clone(),
                version: parse_optional(driver.version.as_deref()),
                date: driver.date.clone(),
                architecture: architecture.to_owned(),
                min_build,
                hardware_ids,
                signature: if driver.catalog_file.is_some() {
                    Signature::Unchecked
                } else {
                    Signature::Invalid
                },
                approved_origin: true,
                stability: Stability::Unknown,
                oem: None,
                category_hint: driver.class.clone(),
            });
        }
    }
    output
}

//! Detecta cambios de driver comparando dos inventarios (DESK-06).

use drivward_core::model::Device;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub instance_id: String,
    pub device_name: String,
    pub class: Option<String>,
    pub old_version: Option<String>,
    pub new_version: Option<String>,
    pub old_inf: Option<String>,
    pub new_inf: Option<String>,
}

fn version(device: &Device) -> Option<String> {
    device
        .driver
        .as_ref()
        .and_then(|d| d.version.clone())
        .filter(|v| !v.is_empty())
}

fn inf(device: &Device) -> Option<String> {
    device
        .driver
        .as_ref()
        .and_then(|d| d.inf_name.clone())
        .filter(|v| !v.is_empty())
}

/// Dispositivos nuevos o retirados no son cambios de driver: solo se comparan los que estaban antes.
pub fn diff(previous: &[Device], current: &[Device]) -> Vec<Change> {
    let before: HashMap<&str, &Device> = previous
        .iter()
        .map(|d| (d.instance_id.as_str(), d))
        .collect();
    current
        .iter()
        .filter_map(|device| {
            let old = before.get(device.instance_id.as_str())?;
            let changed = version(old) != version(device) || inf(old) != inf(device);
            changed.then(|| Change {
                instance_id: device.instance_id.clone(),
                device_name: device.display_name().to_owned(),
                class: device.class.clone().or_else(|| old.class.clone()),
                old_version: version(old),
                new_version: version(device),
                old_inf: inf(old),
                new_inf: inf(device),
            })
        })
        .collect()
}

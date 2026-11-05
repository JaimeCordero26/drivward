use drivward_core::classify::{classify, DeviceState};
use drivward_core::model::{Device, InstalledDriver};
use drivward_core::select::{select, Candidate, Context, Match, Signature, Source, Stability};
use drivward_core::version::DriverVersion;

fn device(version: Option<&str>) -> Device {
    Device {
        instance_id: "PCI\\VEN_10EC&DEV_8168\\1".into(),
        name: Some("Ethernet".into()),
        class: Some("Net".into()),
        hardware_ids: vec![
            "PCI\\VEN_10EC&DEV_8168&SUBSYS_00011462&REV_15".into(),
            "PCI\\VEN_10EC&DEV_8168&SUBSYS_00011462".into(),
            "PCI\\VEN_10EC&DEV_8168".into(),
        ],
        compatible_ids: vec!["PCI\\VEN_10EC&CC_020000".into(), "PCI\\CC_0200".into()],
        driver: version.map(|v| InstalledDriver {
            version: Some(v.into()),
            inf_name: Some("oem3.inf".into()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn candidate(id: &str, ids: &[&str], version: &str) -> Candidate {
    Candidate {
        id: id.into(),
        source: Source::Library,
        provider: Some("Realtek".into()),
        version: Some(version.parse().unwrap()),
        date: Some("2025-01-01".into()),
        architecture: "x64".into(),
        min_build: None,
        hardware_ids: ids.iter().map(|s| s.to_string()).collect(),
        signature: Signature::Unchecked,
        approved_origin: true,
        stability: Stability::Unknown,
        oem: None,
        category_hint: None,
    }
}

fn context() -> Context {
    Context {
        os_build: 22631,
        architecture: "x64".into(),
        system_manufacturer: Some("Dell Inc.".into()),
        allow_downgrade: false,
    }
}

#[test]
fn parses_and_orders_versions() {
    let a: DriverVersion = "10.68.1.2026".parse().unwrap();
    let b: DriverVersion = "10.7".parse().unwrap();
    assert!(a > b);
    assert_eq!(b.to_string(), "10.7.0.0");
    assert!("1.2.3.4.5".parse::<DriverVersion>().is_err());
    assert!("1.x".parse::<DriverVersion>().is_err());
}

#[test]
fn prefers_hardware_id_over_newer_compatible_id() {
    let generic = candidate("generic.inf", &["PCI\\CC_0200"], "99.0");
    let exact = candidate("exact.inf", &["PCI\\VEN_10EC&DEV_8168"], "10.0");
    let result = select(&device(None), &[generic, exact], &context());
    let rec = result.recommendation.unwrap();
    assert_eq!(rec.candidate.id, "exact.inf");
    assert_eq!(rec.matched, Match::HardwareId(2));
    assert!(rec.reasons.iter().any(|r| r.contains("Hardware ID")));
}

#[test]
fn prefers_more_specific_hardware_id() {
    let broad = candidate("broad.inf", &["PCI\\VEN_10EC&DEV_8168"], "11.0");
    let specific = candidate(
        "specific.inf",
        &["PCI\\VEN_10EC&DEV_8168&SUBSYS_00011462"],
        "10.0",
    );
    let rec = select(&device(None), &[broad, specific], &context())
        .recommendation
        .unwrap();
    assert_eq!(rec.candidate.id, "specific.inf");
}

#[test]
fn prefers_stable_over_newest() {
    let mut new = candidate("new.inf", &["PCI\\VEN_10EC&DEV_8168"], "12.0");
    new.stability = Stability::New;
    let mut stable = candidate("stable.inf", &["PCI\\VEN_10EC&DEV_8168"], "11.0");
    stable.stability = Stability::Recommended;
    let rec = select(&device(None), &[new, stable], &context())
        .recommendation
        .unwrap();
    assert_eq!(rec.candidate.id, "stable.inf");
}

#[test]
fn prefers_system_manufacturer_driver() {
    let vendor = candidate("vendor", &["PCI\\VEN_10EC&DEV_8168"], "11.0");
    let mut oem = candidate("oem", &["PCI\\VEN_10EC&DEV_8168"], "10.5");
    oem.source = Source::Oem;
    oem.oem = Some("Dell".into());
    let rec = select(&device(None), &[vendor, oem], &context())
        .recommendation
        .unwrap();
    assert_eq!(rec.candidate.id, "oem");
    assert!(rec
        .reasons
        .iter()
        .any(|r| r.contains("fabricante del equipo")));
}

#[test]
fn rejects_unsafe_candidates() {
    let ids = &["PCI\\VEN_10EC&DEV_8168"];
    let mut arm = candidate("arm", ids, "1.0");
    arm.architecture = "arm64".into();
    let mut unsigned = candidate("unsigned", ids, "1.0");
    unsigned.signature = Signature::Invalid;
    let mut unknown_origin = candidate("origin", ids, "1.0");
    unknown_origin.approved_origin = false;
    let mut blocked = candidate("blocked", ids, "1.0");
    blocked.stability = Stability::Blocked;
    let mut future = candidate("future", ids, "1.0");
    future.min_build = Some(30000);
    let result = select(
        &device(None),
        &[arm, unsigned, unknown_origin, blocked, future],
        &context(),
    );
    assert!(result.recommendation.is_none());
    assert_eq!(result.rejected.len(), 5);
}

#[test]
fn never_downgrades_without_permission() {
    let old = candidate("old.inf", &["PCI\\VEN_10EC&DEV_8168"], "9.0");
    let result = select(
        &device(Some("10.0.0.0")),
        std::slice::from_ref(&old),
        &context(),
    );
    assert!(result.recommendation.is_none());
    assert!(result.rejected[0].reason.contains("menor"));
    let mut ctx = context();
    ctx.allow_downgrade = true;
    let rec = select(&device(Some("10.0.0.0")), &[old], &ctx)
        .recommendation
        .unwrap();
    assert!(!rec.is_update);
}

#[test]
fn classifies_devices() {
    let none = Default::default();
    assert_eq!(classify(&device(None), &none), DeviceState::MissingDriver);
    let mut broken = device(Some("1.0"));
    broken.problem_code = 10;
    assert_eq!(classify(&broken, &none), DeviceState::Error);
    let newer = candidate("n.inf", &["PCI\\VEN_10EC&DEV_8168"], "2.0");
    let sel = select(&device(Some("1.0")), &[newer], &context());
    assert_eq!(classify(&device(Some("1.0")), &sel), DeviceState::Updatable);
    assert_eq!(classify(&device(Some("1.0")), &none), DeviceState::Ok);
}

#[test]
fn detects_network_adapter_without_driver() {
    let mut adapter = device(None);
    adapter.class = None;
    assert!(adapter.looks_like_network());
}

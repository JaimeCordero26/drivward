use drivward_core::inf::{self, InfError};

const FIXTURE: &str = include_str!("fixtures/realtek_like.inf");

#[test]
fn reads_metadata() {
    let driver = inf::parse(FIXTURE).unwrap();
    assert_eq!(driver.provider.as_deref(), Some("Vendor; Inc."));
    assert_eq!(driver.class.as_deref(), Some("Net"));
    assert_eq!(driver.catalog_file.as_deref(), Some("net.cat"));
    assert_eq!(driver.date.as_deref(), Some("2026-03-14"));
    assert_eq!(driver.version.as_deref(), Some("10.68.1.2026"));
}

#[test]
fn reads_each_target() {
    let driver = inf::parse(FIXTURE).unwrap();
    assert_eq!(driver.targets.len(), 3);
    let first = &driver.targets[0];
    assert_eq!(first.architecture, "x64");
    assert_eq!(first.min_os.as_deref(), Some("10.0"));
    assert_eq!(first.min_build, Some(19041));
    assert_eq!(first.devices[0].description, "Gigabit Ethernet");
    assert_eq!(
        first.devices[0].hardware_id,
        "PCI\\VEN_10EC&DEV_8168&SUBSYS_00011462"
    );
    assert_eq!(first.devices[0].compatible_ids, ["PCI\\VEN_10EC&DEV_8168"]);
    assert_eq!(driver.targets[2].architecture, "arm64");
}

#[test]
fn joins_continued_lines_and_normalizes_ids() {
    let driver = inf::parse(FIXTURE).unwrap();
    let device = &driver.targets[1].devices[0];
    assert_eq!(device.description, "Adaptador   largo");
    assert_eq!(device.hardware_id, "PCI\\VEN_10EC&DEV_8125");
}

#[test]
fn decodes_utf16_with_and_without_bom() {
    let mut with_bom = vec![0xFF, 0xFE];
    let mut without_bom = Vec::new();
    for unit in FIXTURE.encode_utf16() {
        with_bom.extend(unit.to_le_bytes());
        without_bom.extend(unit.to_le_bytes());
    }
    let expected = inf::parse(FIXTURE).unwrap();
    assert_eq!(inf::parse_bytes(&with_bom).unwrap(), expected);
    assert_eq!(inf::parse_bytes(&without_bom).unwrap(), expected);
}

#[test]
fn decodes_ansi() {
    let bytes = b"[Version]\nProvider=\"Compa\xF1\xEDa\"\n[Manufacturer]\nM=S\n[S]\nD=I,PCI\\X\n";
    let driver = inf::parse_bytes(bytes).unwrap();
    assert_eq!(driver.provider.as_deref(), Some("Compañía"));
    assert_eq!(driver.targets[0].architecture, "x86");
}

#[test]
fn marks_unresolved_platforms_as_unknown() {
    let text = "[Version]\n[Manufacturer]\nM=S,NT$ARCH$\n[S.NT$ARCH$]\nD=I,PCI\\X\n";
    assert_eq!(inf::parse(text).unwrap().targets[0].architecture, "unknown");
}

#[test]
fn rejects_damaged_files() {
    assert_eq!(
        inf::parse("[Strings]\nA=B\n"),
        Err(InfError::MissingVersion)
    );
    assert_eq!(
        inf::parse("[Version]\n"),
        Err(InfError::MissingManufacturer)
    );
    assert_eq!(
        inf::parse("[Version]\n[Manufacturer]\nM=S\n"),
        Err(InfError::NoDevices)
    );
    assert!(matches!(
        inf::parse("[Version]\nDriverVer=13/99/2026,1.0\n[Manufacturer]\nM=S\n[S]\nD=I,X\n"),
        Err(InfError::InvalidDriverVer(_))
    ));
}

#[test]
fn survives_arbitrary_bytes() {
    let mut state = 7u32;
    for size in 0..2000 {
        let bytes: Vec<u8> = (0..size)
            .map(|_| {
                state = state.wrapping_mul(1103515245).wrapping_add(12345);
                (state >> 16) as u8
            })
            .collect();
        let _ = inf::parse_bytes(&bytes);
    }
    let _ = inf::parse("[Version]\nDriverVer=\n[Manufacturer]\n=\n,\n%\n[");
}

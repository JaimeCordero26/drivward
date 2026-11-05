use drivward_core::classify::DeviceState;
use drivward_core::journal::{ActionKind, ActionRecord, Journal, Outcome};
use drivward_core::report::{DeviceReport, Report};

#[test]
fn appends_and_reads_records() {
    let dir = tempfile::tempdir().unwrap();
    let journal = Journal::open(dir.path().join("logs/acciones.jsonl")).unwrap();
    let mut record = ActionRecord::new(ActionKind::Install, Outcome::RebootRequired);
    record.device_name = Some("Wi-Fi".into());
    journal.append(&record).unwrap();
    journal
        .append(&ActionRecord::new(ActionKind::Backup, Outcome::Success))
        .unwrap();
    let read = journal.read().unwrap();
    assert_eq!(read.len(), 2);
    assert_eq!(read[0], record);
}

#[test]
fn html_report_escapes_content() {
    let report = Report {
        generated_at: "2026-11-05".into(),
        tool: "Drivward".into(),
        system: Default::default(),
        devices: vec![DeviceReport {
            name: "<script>x</script>".into(),
            class: None,
            hardware_id: None,
            state: DeviceState::MissingDriver,
            current_version: None,
            current_provider: None,
            recommended_version: Some("1.0.0.0".into()),
            recommended_source: Some("librería local".into()),
            reasons: vec!["a & b".into()],
        }],
        actions: vec![],
    };
    let html = report.to_html();
    assert!(!html.contains("<script>x"));
    assert!(html.contains("&lt;script&gt;"));
    assert!(html.contains("a &amp; b"));
    assert!(report.to_json().contains("missing_driver"));
}

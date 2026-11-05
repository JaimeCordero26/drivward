use crate::flow::{Session, LIBRARY_DIR};
use drivward_core::classify::DeviceState;
use drivward_core::index;
use drivward_core::journal::{ActionKind, Outcome};
use drivward_core::model::{Device, SystemInfo};
use drivward_core::select::Signature;
use drivward_platform::{
    CommandOutput, Connectivity, InstallResult, Platform, Result, SystemEvent, WindowsUpdateDriver,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const INF: &str = r#"[Version]
Signature = "$Windows NT$"
Class = Net
Provider = %V%
CatalogFile = net.cat
DriverVer = 03/14/2026,10.68.1.2026
[Manufacturer]
%V% = Models, NTamd64.10.0...19041
[Models.NTamd64.10.0...19041]
%D% = Install, PCI\VEN_10EC&DEV_8168
[Strings]
V = "Realtek"
D = "Realtek PCIe GbE"
"#;

#[derive(Default)]
struct Fake {
    installed: Mutex<Vec<PathBuf>>,
    backups: Mutex<u32>,
    signature: Option<Signature>,
    reboot: bool,
}

fn adapter() -> Device {
    Device {
        instance_id: "PCI\\VEN_10EC&DEV_8168\\4&1".into(),
        name: Some("Controladora Ethernet".into()),
        class: None,
        hardware_ids: vec![
            "PCI\\VEN_10EC&DEV_8168&SUBSYS_1".into(),
            "PCI\\VEN_10EC&DEV_8168".into(),
        ],
        compatible_ids: vec!["PCI\\CC_020000".into()],
        problem_code: 28,
        ..Default::default()
    }
}

impl Platform for Fake {
    fn system_info(&self) -> Result<SystemInfo> {
        Ok(SystemInfo {
            os_build: 22631,
            architecture: "x64".into(),
            ..Default::default()
        })
    }
    fn devices(&self) -> Result<Vec<Device>> {
        Ok(vec![adapter()])
    }
    fn verify_signature(&self, _: &Path) -> Result<Signature> {
        Ok(self.signature.unwrap_or(Signature::Valid))
    }
    fn install_inf(&self, inf: &Path) -> Result<InstallResult> {
        self.installed.lock().unwrap().push(inf.to_path_buf());
        Ok(InstallResult {
            reboot_required: self.reboot,
            not_applied: false,
            output: CommandOutput {
                code: 0,
                output: "ok".into(),
            },
        })
    }
    fn backup_drivers(&self, _: &Path) -> Result<CommandOutput> {
        *self.backups.lock().unwrap() += 1;
        Ok(CommandOutput {
            code: 0,
            output: String::new(),
        })
    }
    fn backup_driver(&self, _: &str, _: &Path) -> Result<CommandOutput> {
        unreachable!()
    }
    fn low_battery(&self) -> Result<bool> {
        Ok(false)
    }
    fn create_restore_point(&self, _: &str) -> Result<bool> {
        Ok(false)
    }
    fn check_connectivity(&self) -> Connectivity {
        Connectivity::NoNetwork
    }
    fn open_wifi_settings(&self) -> Result<()> {
        Ok(())
    }
    fn open_network_settings(&self) -> Result<()> {
        Ok(())
    }
    fn windows_update_drivers(&self, _: bool) -> Result<Vec<WindowsUpdateDriver>> {
        Ok(vec![])
    }
    fn install_windows_updates(&self, _: &[String]) -> Result<InstallResult> {
        unreachable!()
    }
    fn set_update_hidden(&self, _: &str, _: bool) -> Result<()> {
        Ok(())
    }
    fn remove_driver(&self, _: &str) -> Result<CommandOutput> {
        unreachable!()
    }
    fn events_since(&self, _: &str) -> Result<Vec<SystemEvent>> {
        Ok(vec![])
    }
    fn reboot_pending(&self) -> Result<bool> {
        Ok(false)
    }
    fn schedule_resume(&self, _: &Path, _: &str) -> Result<()> {
        Ok(())
    }
}

fn usb() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let library = dir.path().join(LIBRARY_DIR);
    let package = library.join("red/realtek");
    fs::create_dir_all(&package).unwrap();
    fs::write(package.join("net.inf"), INF).unwrap();
    fs::write(package.join("net.cat"), b"cat").unwrap();
    fs::write(package.join("net.sys"), b"sys").unwrap();
    let built = index::build(&library).unwrap();
    index::write(&library, &built).unwrap();
    dir
}

#[test]
fn recovers_network_from_library() {
    let dir = usb();
    let fake = Fake::default();
    let mut session = Session::open(&fake, dir.path()).unwrap();
    session.refresh().unwrap();
    let network = session.network_items();
    assert_eq!(network.len(), 1);
    assert_eq!(network[0].state, DeviceState::MissingDriver);
    assert_eq!(network[0].recommended().unwrap().id, "red/realtek/net.inf");

    let record = session.install(&network[0]);
    assert_eq!(record.outcome, Outcome::Success);
    assert_eq!(fake.installed.lock().unwrap().len(), 1);
    // Respaldo y punto de restauración antes del cambio; el punto se omite si Windows no lo permite.
    let kinds: Vec<_> = session.actions.iter().map(|a| a.action).collect();
    assert_eq!(
        kinds,
        [
            ActionKind::Verify,
            ActionKind::Backup,
            ActionKind::RestorePoint,
            ActionKind::Install
        ]
    );
    assert!(matches!(session.actions[2].outcome, Outcome::Skipped(_)));

    session.install(&network[0]);
    assert_eq!(
        *fake.backups.lock().unwrap(),
        1,
        "el respaldo se hace una vez por sesión"
    );
    assert!(dir.path().join("logs/acciones.jsonl").exists());
}

#[test]
fn refuses_tampered_files() {
    let dir = usb();
    let fake = Fake::default();
    let mut session = Session::open(&fake, dir.path()).unwrap();
    session.refresh().unwrap();
    let item = session.network_items().remove(0);
    fs::write(
        dir.path().join(LIBRARY_DIR).join("red/realtek/net.sys"),
        b"malicioso",
    )
    .unwrap();
    let record = session.install(&item);
    assert!(matches!(record.outcome, Outcome::Failed(_)));
    assert!(fake.installed.lock().unwrap().is_empty());
}

#[test]
fn damaged_packages_are_not_offered() {
    let dir = usb();
    fs::write(
        dir.path().join(LIBRARY_DIR).join("red/realtek/net.sys"),
        b"malicioso",
    )
    .unwrap();
    let fake = Fake::default();
    let mut session = Session::open(&fake, dir.path()).unwrap();
    session.refresh().unwrap();
    assert!(session.network_items()[0].recommended().is_none());
    assert!(session.library_summary().contains("alterados"));
}

#[test]
fn refuses_invalid_signature() {
    let dir = usb();
    let fake = Fake {
        signature: Some(Signature::Invalid),
        ..Default::default()
    };
    let mut session = Session::open(&fake, dir.path()).unwrap();
    session.refresh().unwrap();
    let item = session.network_items().remove(0);
    assert!(matches!(session.install(&item).outcome, Outcome::Failed(_)));
    assert!(fake.installed.lock().unwrap().is_empty());
}

#[test]
fn keeps_pending_work_across_reboot_and_exports_report() {
    let dir = usb();
    let fake = Fake {
        reboot: true,
        ..Default::default()
    };
    let mut session = Session::open(&fake, dir.path()).unwrap();
    session.refresh().unwrap();
    let items = session.scan(false);
    assert_eq!(session.install(&items[0]).outcome, Outcome::RebootRequired);
    session
        .save_pending(&["X".into()], Path::new("drivward.exe"))
        .unwrap();
    assert_eq!(session.take_pending(), ["X"]);
    assert!(session.take_pending().is_empty());
    let html = session.export_report(&items).unwrap();
    assert!(fs::read_to_string(html)
        .unwrap()
        .contains("Controladora Ethernet"));
}

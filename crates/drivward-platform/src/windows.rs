//! Implementación para Windows 10 y 11.
//!
//! El inventario y Windows Update se consultan con PowerShell y WMI/CIM, que vienen con
//! Windows; las instalaciones usan `pnputil`. Así la portable no necesita .NET extra ni
//! WebView2 (RC-1). Cada script devuelve JSON que se valida con serde.

use crate::{
    interpret_pnputil, is_published_inf, CommandOutput, InstallResult, Platform, PlatformError,
    Result, SystemEvent, WindowsUpdateDriver,
};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use drivward_core::model::{Device, SystemInfo};
use drivward_core::select::Signature;
use serde::de::DeserializeOwned;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const PRELUDE: &str = "$ErrorActionPreference='Stop';$ProgressPreference='SilentlyContinue';[Console]::OutputEncoding=[Text.Encoding]::UTF8;";

pub struct Windows;

fn system32(program: &str) -> String {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    format!(r"{root}\System32\{program}")
}

fn run(program: &str, args: &[&str]) -> Result<CommandOutput> {
    let output = Command::new(program)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|source| PlatformError::Spawn {
            program: program.into(),
            source,
        })?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    let errors = String::from_utf8_lossy(&output.stderr);
    if !errors.trim().is_empty() {
        text.push('\n');
        text.push_str(errors.trim());
    }
    Ok(CommandOutput {
        code: output.status.code().unwrap_or(-1),
        output: text,
    })
}

fn powershell(script: &str) -> Result<String> {
    let full = format!("{PRELUDE}{script}");
    let utf16: Vec<u8> = full.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let encoded = STANDARD.encode(utf16);
    let program = system32(r"WindowsPowerShell\v1.0\powershell.exe");
    let output = run(
        &program,
        &[
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-EncodedCommand",
            &encoded,
        ],
    )?;
    if output.code != 0 {
        return Err(PlatformError::Command {
            program: "powershell".into(),
            code: output.code,
            output: output.output,
        });
    }
    Ok(output.output)
}

fn powershell_json<T: DeserializeOwned>(what: &str, script: &str) -> Result<T> {
    let text = powershell(script)?;
    let start = text.find(['[', '{']).unwrap_or(0);
    serde_json::from_str(text[start..].trim()).map_err(|error| PlatformError::Parse {
        what: what.into(),
        detail: error.to_string(),
    })
}

/// Escapa un texto para usarlo entre comillas simples de PowerShell.
fn quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

const SYSTEM_INFO: &str = r#"
$os = Get-CimInstance Win32_OperatingSystem
$cs = Get-CimInstance Win32_ComputerSystem
$bb = Get-CimInstance Win32_BaseBoard | Select-Object -First 1
$bios = Get-CimInstance Win32_BIOS | Select-Object -First 1
$cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
$cv = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion'
$arch = switch ($env:PROCESSOR_ARCHITECTURE) { 'AMD64' {'x64'} 'ARM64' {'arm64'} default {'x86'} }
if ($env:PROCESSOR_ARCHITEW6432 -eq 'AMD64') { $arch = 'x64' }
[pscustomobject]@{
  os_name = [string]$os.Caption
  os_version = (($os.Version -split '\.')[0..1] -join '.')
  os_build = [int]$os.BuildNumber
  os_display_version = [string]$cv.DisplayVersion
  edition = [string]$cv.EditionID
  architecture = $arch
  manufacturer = [string]$cs.Manufacturer
  model = [string]$cs.Model
  product_number = [string]$cs.SystemSKUNumber
  baseboard = [string]$bb.Product
  bios_version = [string]$bios.SMBIOSBIOSVersion
  cpu = ([string]$cpu.Name).Trim()
  gpus = @(Get-CimInstance Win32_VideoController | ForEach-Object { [string]$_.Name } | Where-Object { $_ })
} | ConvertTo-Json -Compress -Depth 3
"#;

const DEVICES: &str = r#"
$drivers = @{}
Get-CimInstance Win32_PnPSignedDriver | ForEach-Object { if ($_.DeviceID) { $drivers[$_.DeviceID] = $_ } }
$list = @(Get-CimInstance Win32_PnPEntity | Where-Object { $_.PNPDeviceID } | ForEach-Object {
  $d = $drivers[$_.PNPDeviceID]
  $driver = $null
  if ($d -and ($d.DriverVersion -or $d.InfName)) {
    $date = $null
    if ($d.DriverDate) { $date = ([datetime]$d.DriverDate).ToString('yyyy-MM-dd') }
    $driver = [pscustomobject]@{
      provider = [string]$d.DriverProviderName
      version = [string]$d.DriverVersion
      date = $date
      inf_name = [string]$d.InfName
      signed = [bool]$d.IsSigned
      signer = [string]$d.Signer
    }
  }
  [pscustomobject]@{
    instance_id = [string]$_.PNPDeviceID
    name = [string]$_.Name
    class = $(if ($_.PNPClass) { [string]$_.PNPClass } else { $null })
    class_guid = $(if ($_.ClassGuid) { [string]$_.ClassGuid } else { $null })
    hardware_ids = @($_.HardwareID | Where-Object { $_ } | ForEach-Object { [string]$_ })
    compatible_ids = @($_.CompatibleID | Where-Object { $_ } | ForEach-Object { [string]$_ })
    status = [string]$_.Status
    problem_code = [int]$_.ConfigManagerErrorCode
    driver = $driver
  }
})
ConvertTo-Json -InputObject $list -Compress -Depth 4
"#;

const WU_SEARCH: &str = r#"
$session = New-Object -ComObject Microsoft.Update.Session
$searcher = $session.CreateUpdateSearcher()
$result = $searcher.Search($criteria)
$list = @(foreach ($u in $result.Updates) {
  $date = $null
  if ($u.DriverVerDate) { $date = ([datetime]$u.DriverVerDate).ToString('yyyy-MM-dd') }
  [pscustomobject]@{
    update_id = [string]$u.Identity.UpdateID
    title = [string]$u.Title
    provider = [string]$u.DriverProvider
    hardware_id = [string]$u.DriverHardwareID
    model = [string]$u.DriverModel
    class = [string]$u.DriverClass
    date = $date
    hidden = [bool]$u.IsHidden
  }
})
ConvertTo-Json -InputObject $list -Compress -Depth 3
"#;

const WU_INSTALL: &str = r#"
$session = New-Object -ComObject Microsoft.Update.Session
$searcher = $session.CreateUpdateSearcher()
$result = $searcher.Search("IsInstalled=0 and Type='Driver'")
$selected = New-Object -ComObject Microsoft.Update.UpdateColl
foreach ($u in $result.Updates) { if ($ids -contains $u.Identity.UpdateID) { $null = $selected.Add($u) } }
if ($selected.Count -eq 0) { Write-Output '{"installed":0,"reboot":false,"code":0}'; exit 0 }
$downloader = $session.CreateUpdateDownloader(); $downloader.Updates = $selected; $null = $downloader.Download()
$installer = $session.CreateUpdateInstaller(); $installer.Updates = $selected
$r = $installer.Install()
[pscustomobject]@{ installed = $selected.Count; reboot = [bool]$r.RebootRequired; code = [int]$r.ResultCode } | ConvertTo-Json -Compress
"#;

const WU_HIDE: &str = r#"
$session = New-Object -ComObject Microsoft.Update.Session
$searcher = $session.CreateUpdateSearcher()
$result = $searcher.Search("IsInstalled=0 and Type='Driver'")
$found = $false
foreach ($u in $result.Updates) { if ($u.Identity.UpdateID -eq $id) { $u.IsHidden = $hidden; $found = $true } }
if (-not $found) { throw "No se encontró la actualización $id" }
"#;

const EVENTS: &str = r#"
$filter = @{ LogName = 'System'; Id = 41, 1001, 6008, 4101, 219; StartTime = [datetime]::Parse($since).ToLocalTime() }
$list = @(Get-WinEvent -FilterHashtable $filter -ErrorAction SilentlyContinue | ForEach-Object {
  $message = [string]$_.Message
  if ($message.Length -gt 500) { $message = $message.Substring(0, 500) }
  [pscustomobject]@{ time = $_.TimeCreated.ToUniversalTime().ToString('o'); id = [int]$_.Id; provider = [string]$_.ProviderName; message = $message }
})
ConvertTo-Json -InputObject $list -Compress -Depth 3
"#;

const REBOOT_PENDING: &str = r#"
$pending = (Test-Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Component Based Servicing\RebootPending') -or
  (Test-Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\WindowsUpdate\Auto Update\RebootRequired') -or
  ($null -ne (Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager' -Name PendingFileRenameOperations -ErrorAction SilentlyContinue))
if ($pending) { 'true' } else { 'false' }
"#;

const LOW_BATTERY: &str = r#"
$b = Get-CimInstance Win32_Battery -ErrorAction SilentlyContinue | Select-Object -First 1
# BatteryStatus 2 = conectado a la corriente.
if ($b -and $b.BatteryStatus -ne 2 -and $b.EstimatedChargeRemaining -lt 30) { 'true' } else { 'false' }
"#;

const RESTORE_POINT: &str = r#"
$before = @(Get-ComputerRestorePoint -ErrorAction SilentlyContinue).Count
try { Checkpoint-Computer -Description $description -RestorePointType MODIFY_SETTINGS -WarningAction SilentlyContinue } catch { 'false'; exit 0 }
$after = @(Get-ComputerRestorePoint -ErrorAction SilentlyContinue).Count
if ($after -gt $before) { 'true' } else { 'false' }
"#;

impl Platform for Windows {
    fn system_info(&self) -> Result<SystemInfo> {
        powershell_json("el inventario del sistema", SYSTEM_INFO)
    }

    fn devices(&self) -> Result<Vec<Device>> {
        powershell_json("la lista de dispositivos", DEVICES)
    }

    fn verify_signature(&self, path: &Path) -> Result<Signature> {
        let script = format!(
            "(Get-AuthenticodeSignature -LiteralPath {}).Status.ToString()",
            quote(&path.to_string_lossy())
        );
        Ok(match powershell(&script)?.trim() {
            "Valid" => Signature::Valid,
            _ => Signature::Invalid,
        })
    }

    fn install_inf(&self, inf: &Path) -> Result<InstallResult> {
        let inf = inf.to_string_lossy();
        interpret_pnputil(run(
            &system32("pnputil.exe"),
            &["/add-driver", &inf, "/install"],
        )?)
    }

    fn backup_drivers(&self, destination: &Path) -> Result<CommandOutput> {
        std::fs::create_dir_all(destination).map_err(|source| PlatformError::Spawn {
            program: "mkdir".into(),
            source,
        })?;
        let destination = destination.to_string_lossy();
        let output = run(
            &system32("pnputil.exe"),
            &["/export-driver", "*", &destination],
        )?;
        if output.code != 0 {
            return Err(PlatformError::Command {
                program: "pnputil".into(),
                code: output.code,
                output: output.output,
            });
        }
        Ok(output)
    }

    fn backup_driver(&self, published_inf: &str, destination: &Path) -> Result<CommandOutput> {
        if !is_published_inf(published_inf) {
            return Err(PlatformError::Parse {
                what: "INF publicado".into(),
                detail: published_inf.into(),
            });
        }
        std::fs::create_dir_all(destination).map_err(|source| PlatformError::Spawn {
            program: "mkdir".into(),
            source,
        })?;
        let destination = destination.to_string_lossy();
        let output = run(
            &system32("pnputil.exe"),
            &["/export-driver", published_inf, &destination],
        )?;
        match output.code {
            0 => Ok(output),
            code => Err(PlatformError::Command {
                program: "pnputil".into(),
                code,
                output: output.output,
            }),
        }
    }

    fn low_battery(&self) -> Result<bool> {
        Ok(powershell(LOW_BATTERY)?.trim() == "true")
    }

    fn create_restore_point(&self, description: &str) -> Result<bool> {
        let script = format!("$description = {};{RESTORE_POINT}", quote(description));
        Ok(powershell(&script)?.trim() == "true")
    }

    fn open_wifi_settings(&self) -> Result<()> {
        run(
            &system32("cmd.exe"),
            &["/c", "start", "", "ms-settings:network-wifi"],
        )
        .map(|_| ())
    }

    fn open_network_settings(&self) -> Result<()> {
        run(
            &system32("cmd.exe"),
            &["/c", "start", "", "ms-settings:network-status"],
        )
        .map(|_| ())
    }

    fn windows_update_drivers(&self, include_hidden: bool) -> Result<Vec<WindowsUpdateDriver>> {
        let criteria = match include_hidden {
            true => "IsInstalled=0 and Type='Driver'",
            false => "IsInstalled=0 and Type='Driver' and IsHidden=0",
        };
        powershell_json(
            "Windows Update",
            &format!("$criteria = {};{WU_SEARCH}", quote(criteria)),
        )
    }

    fn install_windows_updates(&self, update_ids: &[String]) -> Result<InstallResult> {
        #[derive(serde::Deserialize)]
        struct Reply {
            installed: u32,
            reboot: bool,
            code: i32,
        }
        let ids = update_ids
            .iter()
            .map(|id| quote(id))
            .collect::<Vec<_>>()
            .join(",");
        let reply: Reply =
            powershell_json("Windows Update", &format!("$ids = @({ids});{WU_INSTALL}"))?;
        // ResultCode 2 = correcto, 3 = correcto con errores.
        if reply.installed > 0 && !matches!(reply.code, 2 | 3) {
            return Err(PlatformError::Command {
                program: "Windows Update".into(),
                code: reply.code,
                output: format!("{} actualizaciones no se instalaron", reply.installed),
            });
        }
        Ok(InstallResult {
            reboot_required: reply.reboot,
            not_applied: reply.installed == 0,
            output: CommandOutput {
                code: reply.code,
                output: format!("{} actualizaciones instaladas", reply.installed),
            },
        })
    }

    fn set_update_hidden(&self, update_id: &str, hidden: bool) -> Result<()> {
        let flag = if hidden { "$true" } else { "$false" };
        powershell(&format!(
            "$id = {}; $hidden = {flag};{WU_HIDE}",
            quote(update_id)
        ))
        .map(|_| ())
    }

    fn remove_driver(&self, published_inf: &str) -> Result<CommandOutput> {
        if !is_published_inf(published_inf) {
            return Err(PlatformError::Parse {
                what: "INF publicado".into(),
                detail: published_inf.into(),
            });
        }
        let output = run(
            &system32("pnputil.exe"),
            &["/delete-driver", published_inf, "/uninstall"],
        )?;
        match output.code {
            0 | 3010 => Ok(output),
            code => Err(PlatformError::Command {
                program: "pnputil".into(),
                code,
                output: output.output,
            }),
        }
    }

    fn events_since(&self, since_rfc3339: &str) -> Result<Vec<SystemEvent>> {
        powershell_json(
            "el registro de eventos",
            &format!("$since = {};{EVENTS}", quote(since_rfc3339)),
        )
    }

    fn reboot_pending(&self) -> Result<bool> {
        Ok(powershell(REBOOT_PENDING)?.trim() == "true")
    }

    fn schedule_resume(&self, executable: &Path, arguments: &str) -> Result<()> {
        let command = format!("\"{}\" {arguments}", executable.display());
        let output = run(
            &system32("reg.exe"),
            &[
                "add",
                r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\RunOnce",
                "/v",
                "Drivward",
                "/t",
                "REG_SZ",
                "/d",
                &command,
                "/f",
            ],
        )?;
        match output.code {
            0 => Ok(()),
            code => Err(PlatformError::Command {
                program: "reg".into(),
                code,
                output: output.output,
            }),
        }
    }
}

//! Integración con Windows: servicio con inicio automático (DESK-01) y tuberías con nombre
//! protegidas para la app de bandeja (SEG-07).

use crate::db::Db;
use crate::engine::Engine;
use crate::{data_dir, DESCRIPTION, DISPLAY_NAME, SERVICE_NAME};
use drivward_ipc::{read_line, Request, Response, CONTROL_PIPE, STATUS_PIPE};
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufReader, Write};
use std::os::windows::io::FromRawHandle;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use time::{OffsetDateTime, UtcOffset};
use windows_service::service::{
    ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl, ServiceExitCode,
    ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};
use windows_service::{define_windows_service, service_dispatcher};
use windows_sys::Win32::Foundation::{
    GetLastError, LocalFree, ERROR_PIPE_CONNECTED, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
use windows_sys::Win32::Storage::FileSystem::{
    FlushFileBuffers, FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_REJECT_REMOTE_CLIENTS,
    PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};

type Error = Box<dyn std::error::Error>;

/// Sistema y administradores con control total; usuarios de la sesión pueden consultar.
const STATUS_SDDL: &str = "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;IU)";
/// Solo sistema y administradores con el token elevado.
const CONTROL_SDDL: &str = "D:P(A;;GA;;;SY)(A;;GA;;;BA)";
/// Cada cuánto se revisan drivers y eventos.
const TICK: Duration = Duration::from_secs(15 * 60);

define_windows_service!(ffi_service_main, service_main);

pub fn install() -> Result<(), Error> {
    let manager = ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE,
    )?;
    let info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(DISPLAY_NAME),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: std::env::current_exe()?,
        launch_arguments: vec![OsString::from("run")],
        dependencies: vec![],
        // LocalSystem es necesario para instalar drivers con pnputil.
        account_name: None,
        account_password: None,
    };
    let service =
        manager.create_service(&info, ServiceAccess::CHANGE_CONFIG | ServiceAccess::START)?;
    service.set_description(DESCRIPTION)?;
    service.start::<&str>(&[])?;
    println!("Servicio instalado e iniciado.");
    Ok(())
}

pub fn uninstall() -> Result<(), Error> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
    let service = manager.open_service(
        SERVICE_NAME,
        ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE,
    )?;
    if service.query_status()?.current_state != ServiceState::Stopped {
        let _ = service.stop();
        for _ in 0..20 {
            if service.query_status()?.current_state == ServiceState::Stopped {
                break;
            }
            thread::sleep(Duration::from_millis(500));
        }
    }
    service.delete()?;
    println!("Servicio eliminado.");
    Ok(())
}

pub fn run() -> Result<(), Error> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
    Ok(())
}

/// Corre el mismo bucle en primer plano, útil para depurar como administrador.
pub fn console() -> Result<(), Error> {
    let (_stop_sender, stop) = mpsc::channel();
    main_loop(stop)
}

fn service_main(_arguments: Vec<OsString>) {
    let (stop_sender, stop) = mpsc::channel();
    let handler = move |control| match control {
        ServiceControl::Stop | ServiceControl::Shutdown => {
            let _ = stop_sender.send(());
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    };
    let Ok(status) = service_control_handler::register(SERVICE_NAME, handler) else {
        return;
    };
    let set = |state, accept| {
        let _ = status.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: state,
            controls_accepted: accept,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::from_secs(10),
            process_id: None,
        });
    };
    set(
        ServiceState::Running,
        ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
    );
    let _ = main_loop(stop);
    set(ServiceState::Stopped, ServiceControlAccept::empty());
}

fn local_now(now: OffsetDateTime) -> OffsetDateTime {
    UtcOffset::current_local_offset()
        .map(|offset| now.to_offset(offset))
        .unwrap_or(now)
}

fn main_loop(stop: mpsc::Receiver<()>) -> Result<(), Error> {
    let dir = data_dir();
    std::fs::create_dir_all(&dir)?;
    let db = Db::open(&dir.join("drivward.db"))?;
    let engine = Arc::new(Mutex::new(Engine::new(
        drivward_platform::native(),
        db,
        dir,
    )));
    let notices = Arc::new(Mutex::new(Vec::<String>::new()));
    let running = Arc::new(AtomicBool::new(true));

    for (pipe, sddl, control) in [
        (STATUS_PIPE, STATUS_SDDL, false),
        (CONTROL_PIPE, CONTROL_SDDL, true),
    ] {
        let engine = engine.clone();
        let running = running.clone();
        thread::spawn(move || serve(pipe, sddl, control, engine, running));
    }

    loop {
        let now = OffsetDateTime::now_utc();
        if let Ok(mut engine) = engine.lock() {
            for notice in engine.tick(now, local_now(now)) {
                if let Ok(mut list) = notices.lock() {
                    list.push(format!("{}: {}", notice.title, notice.body));
                }
            }
        }
        match stop.recv_timeout(TICK) {
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            _ => break,
        }
    }
    running.store(false, Ordering::SeqCst);
    Ok(())
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Crea una instancia de la tubería con el descriptor de seguridad indicado.
fn create_pipe(name: &str, sddl: &str) -> std::io::Result<File> {
    let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
    let sddl = wide(sddl);
    let ok = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor,
        bInheritHandle: 0,
    };
    let name = wide(name);
    // FILE_FLAG_FIRST_PIPE_INSTANCE evita que otro proceso cree la tubería antes que el servicio.
    let handle = unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
            PIPE_TYPE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            PIPE_UNLIMITED_INSTANCES,
            64 * 1024,
            64 * 1024,
            0,
            &attributes,
        )
    };
    unsafe { LocalFree(descriptor as _) };
    if handle == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_handle(handle as _) })
}

fn serve(
    name: &'static str,
    sddl: &'static str,
    control: bool,
    engine: Arc<Mutex<Engine>>,
    running: Arc<AtomicBool>,
) {
    use std::os::windows::io::AsRawHandle;
    while running.load(Ordering::SeqCst) {
        let pipe = match create_pipe(name, sddl) {
            Ok(pipe) => pipe,
            Err(_) => {
                thread::sleep(Duration::from_secs(5));
                continue;
            }
        };
        let handle = pipe.as_raw_handle() as _;
        let connected = unsafe { ConnectNamedPipe(handle, std::ptr::null_mut()) } != 0
            || unsafe { GetLastError() } == ERROR_PIPE_CONNECTED;
        if connected {
            let response = answer(&pipe, control, &engine);
            let mut writer = &pipe;
            if let Ok(mut line) = serde_json::to_string(&response) {
                line.push('\n');
                let _ = writer.write_all(line.as_bytes());
                unsafe { FlushFileBuffers(handle) };
            }
            unsafe { DisconnectNamedPipe(handle) };
        }
    }
}

fn answer(pipe: &File, control: bool, engine: &Mutex<Engine>) -> Response {
    let mut reader = BufReader::new(pipe);
    let request: Request = match read_line(&mut reader)
        .ok()
        .and_then(|line| serde_json::from_str(&line).ok())
    {
        Some(request) => request,
        None => {
            return Response::Error {
                message: "Petición inválida.".into(),
            }
        }
    };
    if request.is_control() && !control {
        return Response::Error {
            message: "Esta acción requiere abrir Drivward como administrador.".into(),
        };
    }
    match engine.lock() {
        Ok(mut engine) => engine.handle(request, OffsetDateTime::now_utc()),
        Err(_) => Response::Error {
            message: "El servicio está ocupado. Intenta de nuevo.".into(),
        },
    }
}

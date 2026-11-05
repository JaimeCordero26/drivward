//! App de bandeja de Drivward (DESK-02). Solo habla con el servicio; nunca cambia drivers por su cuenta.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use drivward_ipc::{Request, Response, CONTROL_PIPE, STATUS_PIPE};
use std::time::Duration;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WindowEvent};
use tauri_plugin_notification::NotificationExt;

/// Envía una petición al servicio. Las que cambian algo van por la tubería de control.
fn send(request: &Request) -> Result<Response, String> {
    let pipe = if request.is_control() {
        CONTROL_PIPE
    } else {
        STATUS_PIPE
    };
    drivward_ipc::request(pipe, request).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => "El servicio de Drivward no está corriendo.".to_owned(),
        std::io::ErrorKind::PermissionDenied => {
            "Esta acción requiere abrir Drivward como administrador.".to_owned()
        }
        _ => format!("No se pudo hablar con el servicio: {error}"),
    })
}

#[tauri::command]
async fn service_request(request: Request) -> Result<Response, String> {
    tauri::async_runtime::spawn_blocking(move || send(&request))
        .await
        .map_err(|error| error.to_string())?
}

/// Indica si la app puede usar la tubería de control (administrador con permisos elevados).
#[tauri::command]
async fn can_control() -> bool {
    tauri::async_runtime::spawn_blocking(|| {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(CONTROL_PIPE)
            .is_ok()
    })
    .await
    .unwrap_or(false)
}

/// Vuelve a abrir la app pidiendo permisos de administrador.
#[tauri::command]
fn relaunch_elevated(app: AppHandle) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let script = format!(
        "Start-Process -FilePath '{}' -Verb RunAs",
        exe.display().to_string().replace('\'', "''")
    );
    std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
        .spawn()
        .map_err(|e| e.to_string())?;
    app.exit(0);
    Ok(())
}

fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Avisa de cambios nuevos y de alertas de estabilidad (DESK-19, DESK-29).
fn watch(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last_change: Option<i64> = None;
        let mut last_health = None;
        loop {
            if let Ok(Response::History(history)) = send(&Request::History { limit: 1 }) {
                if let Some(change) = history.first() {
                    if last_change.is_some_and(|id| id != change.id) {
                        let _ = app
                            .notification()
                            .builder()
                            .title(format!("Cambió el driver de {}", change.device_name))
                            .body(format!(
                                "{} → {}. Si algo falla, puedes volver a la versión anterior desde Historial.",
                                change.old_version.as_deref().unwrap_or("sin versión"),
                                change.new_version.as_deref().unwrap_or("sin driver")
                            ))
                            .show();
                    }
                    last_change = Some(change.id);
                }
            }
            if let Ok(Response::Status(status)) = send(&Request::Status) {
                if status.health == drivward_ipc::Health::Unstable
                    && last_health != Some(status.health)
                {
                    let _ = app
                        .notification()
                        .builder()
                        .title("Drivward detectó fallos")
                        .body(&status.message)
                        .show();
                }
                last_health = Some(status.health);
            }
            std::thread::sleep(Duration::from_secs(60));
        }
    });
}

fn main() {
    let start_hidden = std::env::args().any(|arg| arg == "--bandeja");
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            service_request,
            can_control,
            relaunch_elevated
        ])
        .setup(move |app| {
            let open = MenuItem::with_id(app, "open", "Abrir Drivward", true, None::<&str>)?;
            let pause = MenuItem::with_id(app, "pause", "Pausar 24 horas", true, None::<&str>)?;
            let resume = MenuItem::with_id(app, "resume", "Reanudar", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &pause, &resume, &quit])?;
            TrayIconBuilder::with_id("drivward")
                .icon(
                    app.default_window_icon()
                        .cloned()
                        .expect("la app incluye un ícono"),
                )
                .tooltip("Drivward")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_window(app),
                    "pause" => {
                        let message = match send(&Request::Pause { hours: 24 }) {
                            Ok(_) => "Monitoreo en pausa por 24 horas.".to_owned(),
                            Err(error) => error,
                        };
                        let _ = app
                            .notification()
                            .builder()
                            .title("Drivward")
                            .body(message)
                            .show();
                    }
                    "resume" => {
                        let _ = send(&Request::Resume);
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_window(tray.app_handle());
                    }
                })
                .build(app)?;
            if !start_hidden {
                show_window(app.handle());
            }
            watch(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            // Cerrar la ventana la oculta: la app sigue en la bandeja.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("no se pudo iniciar Drivward");
}

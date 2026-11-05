# Trazabilidad de requerimientos

Estado de cada requerimiento en el código. **Hecho** significa implementado con pruebas automáticas donde aplica; todo debe validarse todavía en equipos reales (TEST-03, TEST-04). **Parcial** indica que falta una parte, descrita en la nota. **Pendiente** aún no tiene código.

## Núcleo

| ID | Estado | Dónde | Nota |
| --- | --- | --- | --- |
| CORE-01 | Hecho | `drivward-platform/src/windows.rs` (`SYSTEM_INFO`) | |
| CORE-02 | Hecho | `windows.rs` (`SYSTEM_INFO`) | No guarda número de serie |
| CORE-03 | Hecho | `windows.rs` (`DEVICES`), `core/src/model.rs` | |
| CORE-04 | Hecho | `windows.rs` (`DEVICES`) | |
| CORE-05 | Hecho | `core/src/classify.rs` | |
| CORE-06 | Hecho | `core/src/inf.rs` | |
| CORE-07 | Hecho | `core/src/index.rs` | |
| CORE-08 | Hecho | `index::verify`, `index::verify_folder` | |
| CORE-09 | Hecho | `core/src/select.rs` | |
| CORE-10 | Hecho | `select::match_device` | |
| CORE-11 | Hecho | `select::compare` | Requiere catálogos OEM (CORE-21) para tener efecto |
| CORE-12 | Hecho | `select::explain` | |
| CORE-13 | Hecho | `Platform::install_inf` | |
| CORE-14 | Hecho | `flow::save_pending`, `Platform::schedule_resume` | RunOnce en HKLM |
| CORE-15 | Hecho | `Platform::backup_drivers` | |
| CORE-16 | Hecho | `Platform::create_restore_point` | |
| CORE-17 | Hecho | `core/src/journal.rs` | |
| CORE-18 | Hecho | `core/src/report.rs` | |
| CORE-19 | Hecho | `Platform::windows_update_drivers` | |
| CORE-20 | Hecho | `Platform::install_windows_updates` | |
| CORE-21 | Pendiente | | Fase 2: catálogos Dell, Lenovo y HP |
| CORE-22 | Pendiente | | Fase 2: NVIDIA, AMD e Intel |
| CORE-23 | Hecho | `Stability::Blocked` en `select.rs`, `catalog.rs` | |
| CORE-24 | Pendiente | | Fase 2 |
| CORE-25 | Hecho | `select::reject_reason` | |
| CORE-26 | Pendiente | | Fase 2 |
| CORE-27 | Parcial | `core/src/i18n.rs` | Estructura lista; la interfaz aún usa textos en español directamente |

## Portable

| ID | Estado | Dónde | Nota |
| --- | --- | --- | --- |
| PORT-01 | Hecho | `drivward-portable` | Un `.exe` autónomo |
| PORT-02 | Hecho | `drivward-portable/build.rs` | `requireAdministrator` |
| PORT-03 | Hecho | `flow.rs` | La librería y la instalación no usan red |
| PORT-04/05 | Parcial | `library/fuentes.ejemplo.json` | Falta definir URL, hash y licencia de cada paquete |
| PORT-06 | Pendiente | | Fase 2 |
| PORT-07 | Hecho | `flow::network_items`, `Device::looks_like_network` | |
| PORT-08 | Hecho | `flow::network_items` | |
| PORT-09 | Hecho | `drivward-platform/src/connectivity.rs` | |
| PORT-10 | Hecho | `Platform::open_wifi_settings` | |
| PORT-11 | Hecho | `gui.rs` (`MOBILE_HELP`) | |
| PORT-12 | Hecho | `gui.rs` (`install_network`) | |
| PORT-13 | Hecho | `gui.rs` (`confirm`), modo consola | |
| PORT-14 | Pendiente | | Fase 2 |
| PORT-15 | Hecho | `flow::export_report` | |
| PORT-16 | Hecho | `drivward-index prepare` | |
| PORT-17 | Hecho | `flow::install_from_library` | |
| PORT-18 | Hecho | `logs/` en la USB, sin nombre de equipo ni usuario | |
| PORT-19 | Pendiente | | Fase 5 |

## Escritorio

| ID | Estado | Dónde | Nota |
| --- | --- | --- | --- |
| DESK-01 | Hecho | `service.rs::install`, `windows/hooks.nsh` | |
| DESK-02 | Hecho | `apps/desktop` | |
| DESK-03 | Hecho | `apps/desktop/src/pages` | |
| DESK-04 | Hecho | `drivward-ipc::Mode` | |
| DESK-05 | Parcial | | El modo inicial es Observar; Recomendar se elige en Ajustes |
| DESK-06 | Hecho | `monitor.rs` | Windows no distingue Windows Update de otros orígenes externos |
| DESK-07 | Hecho | `windows.rs` (`EVENTS`) | |
| DESK-08 | Hecho | `stability.rs` | |
| DESK-09 | Hecho | `stability.rs`, `EvidenceView` | |
| DESK-10 | Hecho | `Request::Pause` | |
| DESK-11 | Hecho | `policy::check_due` | |
| DESK-12 | Hecho | `policy::quarantine_passed` | |
| DESK-13 | Parcial | `catalog.rs` | Falta la API que publique la lista |
| DESK-14 | Hecho | `policy::blocked_reason` | Las tareas críticas no se detectan todavía |
| DESK-15 | Hecho | `policy::blocked_reason` | |
| DESK-16 | Hecho | `engine::validate_changes` | |
| DESK-17 | Hecho | `engine::apply_automatic` | |
| DESK-18 | Hecho | `policy::requires_approval` | |
| DESK-19 | Hecho | `engine::Notice`, notificaciones de la bandeja | |
| DESK-20 | Hecho | `engine::remember_good` | |
| DESK-21 | Hecho | `engine::rollback` | |
| DESK-22 | Hecho | `engine::rollback` | |
| DESK-23 | Hecho | `Platform::set_update_hidden` | |
| DESK-24 | Hecho | `BlocksPage.tsx` | |
| DESK-25 | Hecho | `EvidenceView::suggest_rollback` | |
| DESK-26 | Pendiente | | Fase 4 |
| DESK-27 | Hecho | `Request::MarkGood` | |
| DESK-28 | Hecho | `Settings::auto_rollback` | Apagado por defecto |
| DESK-29 | Hecho | `engine::scan` | |

## Seguridad y calidad

| ID | Estado | Nota |
| --- | --- | --- |
| SEG-01 | Pendiente | Falta el certificado de firma de código |
| SEG-02 | Hecho | `catalog.rs`, `drivward-index sign` |
| SEG-03 | Hecho | Hash por carpeta y firma Authenticode del catálogo `.cat` |
| SEG-04 | Hecho | `select::reject_reason`, `flow::install_from_library` |
| SEG-05 | Hecho | Catálogo y preparador exigen HTTPS |
| SEG-06 | Parcial | El servicio corre como LocalSystem porque `pnputil` lo requiere |
| SEG-07 | Hecho | `service.rs` (SDDL de las tuberías) |
| SEG-08 | Hecho | No hay telemetría |
| SEG-09 | Hecho | Reportes sin nombre de equipo, usuario ni serie |
| SEG-10 | Hecho | Bloquear requiere acción explícita en la app elevada |
| RNF-09 | Hecho | Rotación del registro |
| RNF-10 | Hecho | |
| RNF-11 | Hecho | Trait `Platform` |

## Catálogo, API y distribución

| ID | Estado | Nota |
| --- | --- | --- |
| API-01 a API-10 | Parcial | Formato y verificación del catálogo listos; faltan los recolectores y el servidor |
| DIST-01 | Hecho | GPL-3.0 |
| DIST-02 | Hecho | El repositorio no incluye drivers |
| DIST-03 | Hecho | `scripts/preparar-usb.ps1`, `drivward-index prepare` |
| DIST-04 | Hecho | `fuentes.json` |
| DIST-05 | Parcial | `--redistributable`; falta revisar licencias |

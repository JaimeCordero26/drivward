//! Ventana nativa Win32 de la portable. No usa WebView2 ni .NET (RC-1) y se maneja
//! por completo con teclado: Tab entre controles, flechas y Mayús/Ctrl en la lista (RNF-08).

use crate::flow::{Item, Session};
use drivward_core::journal::Outcome;
use drivward_platform::{Connectivity, Platform};
use native_windows_gui as nwg;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

type Shared = Arc<Mutex<Session<'static>>>;

/// Envía avances a la ventana desde un hilo de trabajo.
struct Reporter {
    sender: Sender<Message>,
    notice: nwg::NoticeSender,
}

impl Reporter {
    fn send(&self, message: Message) {
        let _ = self.sender.send(message);
        self.notice.notice();
    }

    fn status(&self, text: impl Into<String>) {
        self.send(Message::Status(text.into()));
    }
}

/// Resultado de una tarea en segundo plano, que vuelve al hilo de la ventana.
enum Message {
    Status(String),
    Network(Connectivity, Vec<Item>),
    Scan(Vec<Item>),
    Done(String),
    Error(String),
}

const MOBILE_HELP: &str =
    "Si no hay driver para la tarjeta de red, puedes usar el celular:\r\n\r\n\
1. Conecta el celular a la compu con un cable USB.\r\n\
2. En el celular, abre Ajustes > Red o Conexiones > Zona Wi-Fi / Anclaje.\r\n\
3. Activa \"Anclaje a red por USB\" (USB tethering).\r\n\
4. Windows instala solo el adaptador del celular. Después pulsa \"Probar conexión\".\r\n\r\n\
Ese adaptador usa un driver genérico que viene con Windows.";

#[derive(Default)]
struct Ui {
    window: nwg::Window,
    title_font: nwg::Font,
    title: nwg::Label,
    library: nwg::Label,
    network: nwg::Label,
    status: nwg::Label,
    install_network: nwg::Button,
    test_connection: nwg::Button,
    wifi: nwg::Button,
    mobile: nwg::Button,
    scan: nwg::Button,
    install_selected: nwg::Button,
    export: nwg::Button,
    list: nwg::ListView,
    details: nwg::TextBox,
    notice: nwg::Notice,
}

struct App {
    ui: Ui,
    platform: &'static dyn Platform,
    session: Shared,
    sender: Sender<Message>,
    receiver: Receiver<Message>,
    /// Filas de la lista; con la red mostrada, solo adaptadores de red.
    items: RefCell<Vec<Item>>,
    network_items: RefCell<Vec<Item>>,
    resume: bool,
}

pub fn run(resume: bool) {
    nwg::init().expect("no se pudo iniciar la interfaz");
    let _ = nwg::Font::set_global_family("Segoe UI");

    let platform: &'static dyn Platform = Box::leak(drivward_platform::native());
    let session = match Session::open(platform, &crate::portable_root()) {
        Ok(session) => session,
        Err(error) => {
            nwg::error_message("Drivward", &error);
            return;
        }
    };
    let (sender, receiver) = channel();
    let mut ui = Ui::default();
    build(&mut ui).expect("no se pudo crear la ventana");
    let app = Rc::new(App {
        ui,
        platform,
        session: Arc::new(Mutex::new(session)),
        sender,
        receiver,
        items: RefCell::new(Vec::new()),
        network_items: RefCell::new(Vec::new()),
        resume,
    });

    let events = app.clone();
    let handler =
        nwg::full_bind_event_handler(&app.ui.window.handle, move |event, _data, handle| {
            use nwg::Event as E;
            let ui = &events.ui;
            match event {
                E::OnWindowClose if handle == ui.window.handle => nwg::stop_thread_dispatch(),
                E::OnNotice if handle == ui.notice.handle => events.receive(),
                E::OnButtonClick if handle == ui.install_network.handle => events.install_network(),
                E::OnButtonClick if handle == ui.test_connection.handle => events.check_network(),
                E::OnButtonClick if handle == ui.wifi.handle => events.open_wifi(),
                E::OnButtonClick if handle == ui.mobile.handle => events.mobile_help(),
                E::OnButtonClick if handle == ui.scan.handle => events.scan(),
                E::OnButtonClick if handle == ui.install_selected.handle => {
                    events.install_selected()
                }
                E::OnButtonClick if handle == ui.export.handle => events.export(),
                E::OnListViewItemChanged | E::OnListViewClick if handle == ui.list.handle => {
                    events.show_details()
                }
                _ => {}
            }
        });

    app.start();
    nwg::dispatch_thread_events();
    nwg::unbind_event_handler(&handler);
}

fn build(ui: &mut Ui) -> Result<(), nwg::NwgError> {
    nwg::Window::builder()
        .size((960, 680))
        .center(true)
        .title("Drivward Portable")
        .build(&mut ui.window)?;
    nwg::Font::builder()
        .family("Segoe UI")
        .size(30)
        .weight(700)
        .build(&mut ui.title_font)?;
    nwg::Label::builder()
        .text("Drivward Portable")
        .font(Some(&ui.title_font))
        .position((20, 14))
        .size((500, 40))
        .parent(&ui.window)
        .build(&mut ui.title)?;
    for (label, y, text) in [
        (&mut ui.library, 58, "Revisando la librería…"),
        (&mut ui.network, 80, "Revisando la red…"),
        (&mut ui.status, 102, ""),
    ] {
        nwg::Label::builder()
            .text(text)
            .position((22, y))
            .size((910, 20))
            .parent(&ui.window)
            .build(label)?;
    }
    let row1 = [
        (&mut ui.install_network, "Instalar driver de red", 190),
        (&mut ui.test_connection, "Probar conexión", 150),
        (&mut ui.wifi, "Abrir Wi-Fi", 130),
        (&mut ui.mobile, "Internet desde el celular", 200),
    ];
    let mut x = 20;
    for (button, text, width) in row1 {
        nwg::Button::builder()
            .text(text)
            .position((x, 132))
            .size((width, 34))
            .parent(&ui.window)
            .build(button)?;
        x += width + 10;
    }
    let row2 = [
        (&mut ui.scan, "Escanear todo", 150),
        (&mut ui.install_selected, "Instalar seleccionados", 190),
        (&mut ui.export, "Exportar reporte", 150),
    ];
    let mut x = 20;
    for (button, text, width) in row2 {
        nwg::Button::builder()
            .text(text)
            .position((x, 176))
            .size((width, 34))
            .parent(&ui.window)
            .build(button)?;
        x += width + 10;
    }
    nwg::ListView::builder()
        .position((20, 222))
        .size((920, 290))
        .list_style(nwg::ListViewStyle::Detailed)
        .ex_flags(nwg::ListViewExFlags::FULL_ROW_SELECT | nwg::ListViewExFlags::GRID)
        .flags(
            nwg::ListViewFlags::VISIBLE
                | nwg::ListViewFlags::TAB_STOP
                | nwg::ListViewFlags::ALWAYS_SHOW_SELECTION,
        )
        .parent(&ui.window)
        .build(&mut ui.list)?;
    ui.list.set_headers_enabled(true);
    for (index, (text, width)) in [
        ("Dispositivo", 330),
        ("Estado", 110),
        ("Actual", 140),
        ("Recomendado", 170),
        ("Origen", 160),
    ]
    .into_iter()
    .enumerate()
    {
        ui.list.insert_column(nwg::InsertListViewColumn {
            index: Some(index as i32),
            fmt: None,
            width: Some(width),
            text: Some(text.into()),
        });
    }
    nwg::TextBox::builder()
        .position((20, 522))
        .size((920, 130))
        .readonly(true)
        .flags(
            nwg::TextBoxFlags::VISIBLE
                | nwg::TextBoxFlags::VSCROLL
                | nwg::TextBoxFlags::AUTOVSCROLL
                | nwg::TextBoxFlags::TAB_STOP,
        )
        .text("Selecciona un dispositivo para ver por qué se recomienda su driver.")
        .parent(&ui.window)
        .build(&mut ui.details)?;
    nwg::Notice::builder()
        .parent(&ui.window)
        .build(&mut ui.notice)?;
    Ok(())
}

impl App {
    fn busy(&self, busy: bool, text: &str) {
        for button in [
            &self.ui.install_network,
            &self.ui.test_connection,
            &self.ui.scan,
            &self.ui.install_selected,
            &self.ui.export,
        ] {
            button.set_enabled(!busy);
        }
        self.ui.status.set_text(text);
    }

    /// Ejecuta `work` en otro hilo con la sesión bloqueada y avisa a la ventana al terminar.
    fn spawn(
        &self,
        text: &str,
        work: impl FnOnce(&mut Session<'static>, &Reporter) -> Message + Send + 'static,
    ) {
        self.busy(true, text);
        let session = self.session.clone();
        let reporter = Reporter {
            sender: self.sender.clone(),
            notice: self.ui.notice.sender(),
        };
        thread::spawn(move || {
            let message = match session.lock() {
                Ok(mut session) => work(&mut session, &reporter),
                Err(_) => Message::Error(
                    "la sesión quedó en un estado inválido; vuelve a abrir Drivward".into(),
                ),
            };
            reporter.send(message);
        });
    }

    fn start(&self) {
        let resume = self.resume;
        self.spawn("Leyendo el equipo…", move |session, _| {
            if let Err(error) = session.refresh() {
                return Message::Error(format!("No se pudo leer el equipo: {error}"));
            }
            let connectivity = session.connectivity();
            if resume || connectivity == Connectivity::Online {
                let pending = if resume {
                    session.take_pending()
                } else {
                    Vec::new()
                };
                let mut items = session.scan(connectivity == Connectivity::Online);
                if !pending.is_empty() {
                    items.retain(|item| {
                        pending.contains(&item.device.instance_id) || item.actionable()
                    });
                }
                return Message::Scan(items);
            }
            Message::Network(connectivity, session.network_items())
        });
        if let Ok(session) = self.session.try_lock() {
            self.ui.library.set_text(&session.library_summary());
        }
    }

    fn receive(&self) {
        while let Ok(message) = self.receiver.try_recv() {
            match message {
                Message::Status(text) => self.ui.status.set_text(&text),
                Message::Network(connectivity, items) => {
                    self.busy(false, "");
                    self.show_network(connectivity, items);
                }
                Message::Scan(items) => {
                    self.busy(
                        false,
                        "Escaneo terminado. Selecciona los drivers que apruebas instalar.",
                    );
                    if let Ok(session) = self.session.try_lock() {
                        self.ui.library.set_text(&session.library_summary());
                    }
                    self.fill(&items);
                    *self.items.borrow_mut() = items;
                }
                Message::Done(text) => {
                    self.busy(false, &text);
                    nwg::modal_info_message(&self.ui.window, "Drivward", &text);
                }
                Message::Error(text) => {
                    self.busy(false, &text);
                    nwg::modal_error_message(&self.ui.window, "Drivward", &text);
                }
            }
        }
    }

    fn show_network(&self, connectivity: Connectivity, items: Vec<Item>) {
        self.ui
            .network
            .set_text(&format!("Red: {}", connectivity.label()));
        let installable = items.iter().any(Item::actionable);
        self.ui.install_network.set_enabled(installable);
        let status = match (connectivity, items.is_empty(), installable) {
            (Connectivity::Online, _, _) => "Hay internet. Pulsa \"Escanear todo\" para revisar el resto.",
            (_, true, _) => "No hay adaptadores de red sin driver. Conéctate a una red con \"Abrir Wi-Fi\" o usa el celular.",
            (_, false, true) => "Encontramos el driver de red. Pulsa \"Instalar driver de red\" para aprobarlo.",
            (_, false, false) => "La librería no tiene un driver compatible. Usa el celular para conectarte.",
        };
        self.ui.status.set_text(status);
        self.fill(&items);
        *self.items.borrow_mut() = items.clone();
        *self.network_items.borrow_mut() = items;
    }

    fn fill(&self, items: &[Item]) {
        let list = &self.ui.list;
        list.set_redraw(false);
        list.clear();
        for item in items {
            let current = item
                .device
                .driver
                .as_ref()
                .and_then(|d| d.version.clone())
                .filter(|v| !v.is_empty());
            let recommended = item.recommended();
            let row = [
                item.device.display_name().to_owned(),
                item.state.label().to_owned(),
                current.unwrap_or_else(|| "-".into()),
                recommended
                    .map(|c| {
                        c.version
                            .map(|v| v.to_string())
                            .or(c.date.clone())
                            .unwrap_or_else(|| "-".into())
                    })
                    .unwrap_or_else(|| "-".into()),
                recommended
                    .map(|c| c.source.label().to_owned())
                    .unwrap_or_else(|| "-".into()),
            ];
            list.insert_items_row(None, &row);
        }
        list.set_redraw(true);
    }

    fn show_details(&self) {
        let items = self.items.borrow();
        let Some(item) = self.ui.list.selected_item().and_then(|row| items.get(row)) else {
            return;
        };
        let mut text = format!(
            "{}\r\nHardware ID: {}\r\n\r\n",
            item.device.display_name(),
            item.device
                .hardware_ids
                .first()
                .map(String::as_str)
                .unwrap_or("-")
        );
        match &item.selection.recommendation {
            Some(recommendation) => {
                text.push_str(&format!("Recomendado: {}\r\n", recommendation.candidate.id));
                for reason in &recommendation.reasons {
                    text.push_str(&format!("• {reason}\r\n"));
                }
            }
            None => text.push_str("No hay un driver compatible en las fuentes disponibles.\r\n"),
        }
        for rejected in &item.selection.rejected {
            text.push_str(&format!(
                "Descartado {}: {}\r\n",
                rejected.candidate_id, rejected.reason
            ));
        }
        self.ui.details.set_text(&text);
    }

    fn check_network(&self) {
        self.spawn("Probando la conexión…", |session, _| {
            let _ = session.refresh();
            Message::Network(session.connectivity(), session.network_items())
        });
    }

    fn open_wifi(&self) {
        if let Err(error) = self.platform.open_wifi_settings() {
            nwg::modal_error_message(&self.ui.window, "Drivward", &error.to_string());
        }
    }

    fn mobile_help(&self) {
        nwg::modal_info_message(&self.ui.window, "Internet desde el celular", MOBILE_HELP);
        let _ = self.platform.open_network_settings();
    }

    /// Pide aprobación explícita antes de instalar (PORT-13).
    fn confirm(&self, items: &[Item]) -> bool {
        let list = items
            .iter()
            .map(|item| {
                let candidate = item
                    .recommended()
                    .expect("solo se confirman filas con recomendación");
                format!(
                    "• {} → {} ({})",
                    item.device.display_name(),
                    candidate.id,
                    candidate.source.label()
                )
            })
            .collect::<Vec<_>>()
            .join("\r\n");
        let params = nwg::MessageParams {
            title: "Aprobar instalación",
            content: &format!(
                "Se instalarán estos drivers:\r\n\r\n{list}\r\n\r\nAntes se respaldan los drivers actuales y se intenta crear un punto de restauración. ¿Continuar?"
            ),
            buttons: nwg::MessageButtons::YesNo,
            icons: nwg::MessageIcons::Question,
        };
        nwg::modal_message(&self.ui.window, &params) == nwg::MessageChoice::Yes
    }

    fn install_network(&self) {
        let items: Vec<Item> = self
            .network_items
            .borrow()
            .iter()
            .filter(|i| i.actionable())
            .take(1)
            .cloned()
            .collect();
        if items.is_empty() || !self.confirm(&items) {
            return;
        }
        self.spawn(
            "Instalando el driver de red…",
            move |session, reporter| {
                let record = session.install(&items[0]);
                reporter.status("Probando la conexión…");
                // El adaptador tarda unos segundos en obtener dirección después de instalar.
                let mut connectivity = session.connectivity();
                for _ in 0..5 {
                    if connectivity == Connectivity::Online {
                        break;
                    }
                    thread::sleep(std::time::Duration::from_secs(3));
                    connectivity = session.connectivity();
                }
                let _ = session.refresh();
                match record.outcome {
                    Outcome::Failed(error) => {
                        Message::Error(format!("No se pudo instalar: {error}"))
                    }
                    _ if connectivity == Connectivity::Online => Message::Scan(session.scan(true)),
                    _ => Message::Network(connectivity, session.network_items()),
                }
            },
        );
    }

    fn install_selected(&self) {
        let rows = self.ui.list.selected_items();
        let items: Vec<Item> = {
            let all = self.items.borrow();
            rows.iter()
                .filter_map(|row| all.get(*row))
                .filter(|i| i.actionable())
                .cloned()
                .collect()
        };
        if items.is_empty() {
            nwg::modal_info_message(
                &self.ui.window,
                "Drivward",
                "Selecciona en la lista uno o más dispositivos con driver recomendado.",
            );
            return;
        }
        if !self.confirm(&items) {
            return;
        }
        self.spawn("Instalando…", move |session, reporter| {
            let mut summary = Vec::new();
            let mut pending = Vec::new();
            for (position, item) in items.iter().enumerate() {
                reporter.status(format!("Instalando {}…", item.device.display_name()));
                let record = session.install(item);
                let result = match &record.outcome {
                    Outcome::Success => "instalado".to_owned(),
                    Outcome::RebootRequired => "instalado, requiere reinicio".to_owned(),
                    Outcome::Skipped(why) => format!("omitido: {why}"),
                    Outcome::Failed(why) => format!("falló: {why}"),
                };
                summary.push(format!("• {}: {result}", item.device.display_name()));
                if record.outcome == Outcome::RebootRequired {
                    pending = items[position + 1..].iter().map(|i| i.device.instance_id.clone()).collect();
                    break;
                }
            }
            if !pending.is_empty() {
                let exe = std::env::current_exe().unwrap_or_default();
                summary.push(match session.save_pending(&pending, &exe) {
                    Ok(()) => format!("\r\nReinicia la compu. Drivward se abrirá solo para seguir con {} drivers.", pending.len()),
                    Err(error) => format!("\r\nReinicia la compu y vuelve a abrir Drivward. ({error})"),
                });
            }
            let _ = session.refresh();
            reporter.send(Message::Scan(session.scan(false)));
            Message::Done(summary.join("\r\n"))
        });
    }

    fn scan(&self) {
        self.spawn("Escaneando dispositivos…", |session, _| {
            if let Err(error) = session.refresh() {
                return Message::Error(error);
            }
            let online = session.connectivity() == Connectivity::Online;
            Message::Scan(session.scan(online))
        });
    }

    fn export(&self) {
        let items = self.items.borrow().clone();
        self.spawn("Guardando el reporte…", move |session, _| {
            match session.export_report(&items) {
                Ok(path) => {
                    let _ = std::process::Command::new("explorer.exe")
                        .arg(&path)
                        .spawn();
                    Message::Done(format!("Reporte guardado en {}", path.display()))
                }
                Err(error) => Message::Error(format!("No se pudo guardar el reporte: {error}")),
            }
        });
    }
}

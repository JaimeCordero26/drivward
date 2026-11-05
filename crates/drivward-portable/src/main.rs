//! Drivward Portable: corre desde la USB, sin instalación (PORT-01).
//!
//! Sin argumentos abre la ventana nativa. Con `--consola` ofrece el mismo flujo en texto
//! para técnicos y para scripts.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod flow;
#[cfg(windows)]
mod gui;

use flow::{Item, Session};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

/// Carpeta donde está el ejecutable: la raíz de la portable en la USB.
pub fn portable_root() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let resume = args.iter().any(|a| a == "--reanudar");
    if args
        .iter()
        .any(|a| a == "--ayuda" || a == "--help" || a == "-h")
    {
        println!("Uso: drivward-portable [--consola] [--reanudar]");
        println!("  --consola   flujo en modo texto");
        println!("  --reanudar  continúa la instalación pendiente después de reiniciar");
        return ExitCode::SUCCESS;
    }
    if args.iter().any(|a| a == "--consola") {
        return console(resume);
    }
    #[cfg(windows)]
    {
        gui::run(resume);
        ExitCode::SUCCESS
    }
    #[cfg(not(windows))]
    {
        console(resume)
    }
}

fn ask(question: &str) -> String {
    print!("{question} ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    let _ = io::stdin().lock().read_line(&mut line);
    line.trim().to_lowercase()
}

fn print_items(items: &[Item]) {
    for (number, item) in items.iter().enumerate() {
        let recommended = item
            .recommended()
            .map(|c| {
                format!(
                    "{} ({})",
                    c.version
                        .map(|v| v.to_string())
                        .or(c.date.clone())
                        .unwrap_or_default(),
                    c.source.label()
                )
            })
            .unwrap_or_else(|| "-".into());
        println!(
            "{:>3}. [{}] {} → {}",
            number + 1,
            item.state.label(),
            item.device.display_name(),
            recommended
        );
    }
}

fn console(resume: bool) -> ExitCode {
    let platform = drivward_platform::native();
    let root = portable_root();
    let mut session = match Session::open(platform.as_ref(), &root) {
        Ok(session) => session,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(2);
        }
    };
    println!("Drivward Portable {}", env!("CARGO_PKG_VERSION"));
    println!("{}", session.library_summary());
    if let Err(error) = session.refresh() {
        eprintln!("No se pudo leer el equipo: {error}");
        return ExitCode::from(2);
    }
    let pending = if resume {
        session.take_pending()
    } else {
        Vec::new()
    };

    let mut connectivity = session.connectivity();
    println!("Red: {}", connectivity.label());
    if connectivity != drivward_platform::Connectivity::Online {
        let network = session.network_items();
        if network.is_empty() {
            println!("No hay adaptadores de red sin driver. Conéctate a una red o comparte internet desde el celular por USB.");
        } else {
            print_items(&network);
            for item in network.iter().filter(|item| item.actionable()) {
                let question = format!(
                    "¿Instalar el driver recomendado para {}? (s/n)",
                    item.device.display_name()
                );
                if ask(&question) == "s" {
                    let record = session.install(item);
                    println!("Resultado: {:?}", record.outcome);
                }
            }
            connectivity = session.connectivity();
            println!("Red: {}", connectivity.label());
        }
    }

    let _ = session.refresh();
    let items = session.scan(connectivity == drivward_platform::Connectivity::Online);
    let actionable: Vec<&Item> = items.iter().filter(|item| item.actionable()).collect();
    println!("\nDispositivos que necesitan atención:");
    let owned: Vec<Item> = actionable.iter().map(|item| (*item).clone()).collect();
    print_items(&owned);

    let chosen: Vec<&Item> = if resume && !pending.is_empty() {
        owned
            .iter()
            .filter(|item| pending.contains(&item.device.instance_id))
            .collect()
    } else {
        let answer = ask("Números a instalar separados por espacio (vacío para ninguno):");
        answer
            .split_whitespace()
            .filter_map(|n| n.parse::<usize>().ok())
            .filter_map(|n| owned.get(n.wrapping_sub(1)))
            .collect()
    };
    let mut reboot = Vec::new();
    for (position, item) in chosen.iter().enumerate() {
        let record = session.install(item);
        println!("{}: {:?}", item.device.display_name(), record.outcome);
        if record.outcome == drivward_core::journal::Outcome::RebootRequired {
            reboot = chosen[position + 1..]
                .iter()
                .map(|i| i.device.instance_id.clone())
                .collect();
            break;
        }
    }
    if !reboot.is_empty() {
        let exe = std::env::current_exe().unwrap_or_default();
        match session.save_pending(&reboot, &exe) {
            Ok(()) => println!(
                "Hace falta reiniciar. Drivward seguirá con {} drivers al volver.",
                reboot.len()
            ),
            Err(error) => {
                println!("Hace falta reiniciar. No se pudo programar la reanudación: {error}")
            }
        }
    }
    let _ = session.refresh();
    let final_items = session.scan(false);
    match session.export_report(&final_items) {
        Ok(path) => println!("Reporte: {}", path.display()),
        Err(error) => println!("No se pudo guardar el reporte: {error}"),
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests;

//! `drivward-service install | uninstall | run | console`

use std::process::ExitCode;

fn main() -> ExitCode {
    let command = std::env::args().nth(1).unwrap_or_default();
    #[cfg(windows)]
    {
        let result = match command.as_str() {
            "install" => drivward_service::service::install(),
            "uninstall" => drivward_service::service::uninstall(),
            "run" => drivward_service::service::run(),
            "console" => drivward_service::service::console(),
            _ => {
                eprintln!("Uso: drivward-service install | uninstall | run | console");
                return ExitCode::from(2);
            }
        };
        match result {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = command;
        eprintln!("El servicio de Drivward solo funciona en Windows.");
        ExitCode::FAILURE
    }
}

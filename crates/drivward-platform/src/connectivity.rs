//! Prueba de conexión real a internet (PORT-09), igual que la que usa Windows.

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

const HOST: &str = "www.msftconnecttest.com";
const PATH: &str = "/connecttest.txt";
const EXPECTED: &str = "Microsoft Connect Test";
const TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Connectivity {
    Online,
    /// Hay red, pero un portal cautivo pide iniciar sesión.
    CaptivePortal,
    /// Hay DNS pero el servidor no responde.
    NoInternet,
    /// No se puede resolver nombres: normalmente no hay red.
    NoNetwork,
}

impl Connectivity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Online => "Conectado a internet",
            Self::CaptivePortal => "La red pide iniciar sesión en el navegador",
            Self::NoInternet => "Hay red, pero no hay internet",
            Self::NoNetwork => "Sin red",
        }
    }
}

pub fn check() -> Connectivity {
    let Ok(mut addresses) = (HOST, 80).to_socket_addrs() else {
        return Connectivity::NoNetwork;
    };
    let Some(address) = addresses.next() else {
        return Connectivity::NoNetwork;
    };
    match fetch(address) {
        Some(body) if body.contains(EXPECTED) => Connectivity::Online,
        Some(_) => Connectivity::CaptivePortal,
        None => Connectivity::NoInternet,
    }
}

fn fetch(address: std::net::SocketAddr) -> Option<String> {
    let mut stream = TcpStream::connect_timeout(&address, TIMEOUT).ok()?;
    stream.set_read_timeout(Some(TIMEOUT)).ok()?;
    stream.set_write_timeout(Some(TIMEOUT)).ok()?;
    let request = format!(
        "GET {PATH} HTTP/1.1\r\nHost: {HOST}\r\nConnection: close\r\nUser-Agent: Drivward\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).ok()?;
    let mut response = Vec::new();
    stream.take(64 * 1024).read_to_end(&mut response).ok()?;
    Some(String::from_utf8_lossy(&response).into_owned())
}

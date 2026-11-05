use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use clap::{Parser, Subcommand};
use drivward_core::catalog::SignedCatalog;
use drivward_core::index;
use ed25519_dalek::{Signer, SigningKey};
use std::path::PathBuf;
use std::process::ExitCode;

mod prepare;

#[derive(Parser)]
#[command(
    name = "drivward-index",
    version,
    about = "Prepara la librería de drivers y firma catálogos"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Analiza una carpeta y escribe index.json dentro de ella
    Build { root: PathBuf },
    /// Comprueba que index.json coincide con los archivos
    Verify { root: PathBuf },
    /// Descarga los paquetes oficiales de un manifiesto, los extrae y genera el índice
    Prepare {
        /// Manifiesto con URL, SHA-256 y licencia de cada paquete
        manifest: PathBuf,
        /// Carpeta `drivers` de la USB
        library: PathBuf,
        /// Solo paquetes cuya licencia permite redistribuirlos
        #[arg(long)]
        redistributable: bool,
    },
    /// Firma un catálogo JSON con una clave Ed25519 de 32 bytes
    Sign {
        /// Archivo con la semilla de la clave privada (32 bytes). Debe guardarse fuera de los recolectores (API-07)
        #[arg(long)]
        key: PathBuf,
        catalog: PathBuf,
        output: PathBuf,
    },
    /// Muestra la clave pública en base64 a partir de la semilla privada
    PublicKey {
        #[arg(long)]
        key: PathBuf,
    },
}

fn main() -> ExitCode {
    let result = match Cli::parse().command {
        Command::Build { root } => run_build(root).map_err(|e| e.to_string()),
        Command::Verify { root } => run_verify(root).map_err(|e| e.to_string()),
        Command::Prepare {
            manifest,
            library,
            redistributable,
        } => run_prepare(manifest, library, redistributable),
        Command::Sign {
            key,
            catalog,
            output,
        } => run_sign(key, catalog, output),
        Command::PublicKey { key } => load_key(&key).map(|key| {
            println!("{}", STANDARD.encode(key.verifying_key().to_bytes()));
            ExitCode::SUCCESS
        }),
    };
    match result {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(2)
        }
    }
}

fn run_build(root: PathBuf) -> Result<ExitCode, index::IndexError> {
    let built = index::build(&root)?;
    index::write(&root, &built)?;
    println!(
        "Índice listo: {} paquetes, {} rechazados, {} archivos",
        built.packages.len(),
        built.rejected.len(),
        built.files.len()
    );
    for item in &built.rejected {
        println!("rechazado: {}: {}", item.inf, item.reason);
    }
    Ok(ExitCode::SUCCESS)
}

fn run_verify(root: PathBuf) -> Result<ExitCode, index::IndexError> {
    let loaded = index::load(&root)?;
    let result = index::verify(&root, &loaded)?;
    let groups = [
        ("faltante", &result.missing),
        ("modificado", &result.changed),
        ("inesperado", &result.unexpected),
        ("ruta insegura", &result.unsafe_paths),
    ];
    for (label, paths) in groups {
        for path in paths {
            println!("{label}: {path}");
        }
    }
    if result.is_valid() {
        println!("Índice válido: {} archivos", loaded.files.len());
        return Ok(ExitCode::SUCCESS);
    }
    println!("Índice inválido");
    Ok(ExitCode::FAILURE)
}

fn run_prepare(
    manifest: PathBuf,
    library: PathBuf,
    redistributable: bool,
) -> Result<ExitCode, String> {
    let text =
        std::fs::read_to_string(&manifest).map_err(|e| format!("{}: {e}", manifest.display()))?;
    let manifest: prepare::Manifest =
        serde_json::from_str(&text).map_err(|e| format!("manifiesto inválido: {e}"))?;
    let results = prepare::prepare(&manifest, &library, redistributable)?;
    let mut failed = false;
    for item in &results {
        match &item.result {
            Ok(()) => println!("listo: {}", item.id),
            Err(error) => {
                failed = true;
                println!("omitido: {}: {error}", item.id);
            }
        }
    }
    run_build(library).map_err(|e| e.to_string())?;
    Ok(if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

fn load_key(path: &PathBuf) -> Result<SigningKey, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let seed: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| "la clave debe tener exactamente 32 bytes".to_owned())?;
    Ok(SigningKey::from_bytes(&seed))
}

fn run_sign(key: PathBuf, catalog: PathBuf, output: PathBuf) -> Result<ExitCode, String> {
    let key = load_key(&key)?;
    let payload =
        std::fs::read_to_string(&catalog).map_err(|e| format!("{}: {e}", catalog.display()))?;
    // Se valida el formato antes de firmar para no publicar un catálogo roto.
    serde_json::from_str::<drivward_core::catalog::Catalog>(&payload)
        .map_err(|e| format!("catálogo inválido: {e}"))?;
    let signature = key.sign(payload.as_bytes());
    let signed = SignedCatalog {
        payload,
        signature: STANDARD.encode(signature.to_bytes()),
    };
    let text = serde_json::to_string_pretty(&signed).map_err(|e| e.to_string())?;
    std::fs::write(&output, text).map_err(|e| e.to_string())?;
    println!("Catálogo firmado: {}", output.display());
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::prepare::{place, validate, Manifest, Source};
    use std::io::Write;

    fn source(sha256: &str) -> Source {
        Source {
            id: "realtek-lan".into(),
            category: "red/ethernet".into(),
            vendor: "Realtek".into(),
            description: "LAN".into(),
            url: "https://example.com/lan.zip".into(),
            sha256: sha256.into(),
            license: "Licencia del fabricante".into(),
            redistributable: false,
        }
    }

    #[test]
    fn rejects_unsafe_manifests() {
        let mut bad = source(&"a".repeat(64));
        bad.url = "http://example.com".into();
        assert!(validate(&Manifest {
            schema_version: 1,
            packages: vec![bad]
        })
        .is_err());
        let mut escape = source(&"a".repeat(64));
        escape.category = "../fuera".into();
        assert!(validate(&Manifest {
            schema_version: 1,
            packages: vec![escape]
        })
        .is_err());
        assert!(validate(&Manifest {
            schema_version: 1,
            packages: vec![source("corto")]
        })
        .is_err());
        assert!(validate(&Manifest {
            schema_version: 1,
            packages: vec![source(&"a".repeat(64))]
        })
        .is_ok());
    }

    #[test]
    fn places_zip_only_when_hash_matches() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("lan.zip");
        {
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
            zip.start_file("net.inf", zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"[Version]").unwrap();
            zip.finish().unwrap();
        }
        let bytes = std::fs::read(&archive).unwrap();
        let hash: String = <sha2::Sha256 as sha2::Digest>::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let library = dir.path().join("drivers");
        assert!(place(&source(&"0".repeat(64)), &archive, &library).is_err());
        place(&source(&hash), &archive, &library).unwrap();
        assert!(library.join("red/ethernet/realtek-lan/net.inf").exists());
    }
}

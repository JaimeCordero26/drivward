//! Incrusta el manifiesto: pide permisos de administrador al abrir (PORT-02),
//! activa los controles visuales modernos y el escalado por monitor (RNF-08).

fn main() {
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        use embed_manifest::manifest::{DpiAwareness, ExecutionLevel};
        use embed_manifest::{embed_manifest, new_manifest};
        let manifest = new_manifest("Drivward.Portable")
            .requested_execution_level(ExecutionLevel::RequireAdministrator)
            .dpi_awareness(DpiAwareness::PerMonitorV2);
        embed_manifest(manifest).expect("no se pudo incrustar el manifiesto");
    }
    println!("cargo:rerun-if-changed=build.rs");
}

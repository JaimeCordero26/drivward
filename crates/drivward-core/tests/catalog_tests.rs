use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use drivward_core::catalog::{self, CatalogError, SignedCatalog};
use ed25519_dalek::{Signer, SigningKey};

const PAYLOAD: &str = r#"{"schema_version":1,"shard":"intel","generated_at":"2026-11-01","entries":[
{"id":"intel-wifi-23.80","provider":"Intel","source":"vendor","version":"23.80.0.4","date":"2026-09-10",
"architecture":"x64","min_build":19041,"hardware_ids":["pci\\ven_8086&dev_a0f0"],"oem":null,
"url":"https://downloadmirror.intel.com/example.zip","sha256":"00","license":"Intel EULA","status":"recommended"}]}"#;

fn signed(payload: &str) -> (SignedCatalog, [u8; 32]) {
    let key = SigningKey::from_bytes(&[7u8; 32]);
    let signature = key.sign(payload.as_bytes());
    (
        SignedCatalog {
            payload: payload.into(),
            signature: STANDARD.encode(signature.to_bytes()),
        },
        key.verifying_key().to_bytes(),
    )
}

#[test]
fn accepts_valid_signature() {
    let (signed, public) = signed(PAYLOAD);
    let catalog = catalog::verify(&signed, &public).unwrap();
    let candidates = catalog.candidates();
    assert_eq!(candidates[0].hardware_ids, ["PCI\\VEN_8086&DEV_A0F0"]);
}

#[test]
fn rejects_tampered_payload() {
    let (mut signed, public) = signed(PAYLOAD);
    signed.payload = signed.payload.replace("23.80.0.4", "99.0.0.0");
    assert!(matches!(
        catalog::verify(&signed, &public),
        Err(CatalogError::BadSignature)
    ));
}

#[test]
fn rejects_plain_http() {
    let (signed, public) = signed(&PAYLOAD.replace("https://", "http://"));
    assert!(matches!(
        catalog::verify(&signed, &public),
        Err(CatalogError::InsecureUrl(_))
    ));
}

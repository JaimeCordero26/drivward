use drivward_core::index;
use std::fs;
use std::path::Path;

const FIXTURE: &str = include_str!("fixtures/realtek_like.inf");

fn library() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let package = dir.path().join("wifi/vendor");
    fs::create_dir_all(&package).unwrap();
    fs::write(package.join("net.inf"), FIXTURE).unwrap();
    fs::write(package.join("net.sys"), b"driver").unwrap();
    fs::write(package.join("broken.inf"), "[Strings]\n").unwrap();
    dir
}

fn built(root: &Path) -> index::Index {
    let built = index::build(root).unwrap();
    index::write(root, &built).unwrap();
    index::load(root).unwrap()
}

#[test]
fn builds_portable_sorted_index() {
    let dir = library();
    let loaded = built(dir.path());
    assert_eq!(loaded.packages.len(), 1);
    assert_eq!(loaded.packages[0].inf, "wifi/vendor/net.inf");
    assert_eq!(loaded.rejected[0].inf, "wifi/vendor/broken.inf");
    assert_eq!(loaded.files.len(), 3);
    assert_eq!(
        loaded.files["wifi/vendor/net.sys"].sha256,
        "b4def8217cadae26d4da633fd2a4e58e326cbb5d570afdc3989484da07af3579"
    );
}

#[test]
fn building_twice_gives_same_index() {
    let dir = library();
    let first = built(dir.path());
    assert_eq!(index::build(dir.path()).unwrap(), first);
}

#[test]
fn accepts_untouched_library() {
    let dir = library();
    let loaded = built(dir.path());
    assert!(index::verify(dir.path(), &loaded).unwrap().is_valid());
}

#[test]
fn detects_changed_missing_and_extra_files() {
    let dir = library();
    let loaded = built(dir.path());
    let package = dir.path().join("wifi/vendor");
    fs::write(package.join("net.sys"), b"altered").unwrap();
    fs::remove_file(package.join("net.inf")).unwrap();
    fs::write(package.join("extra.dll"), b"new").unwrap();
    let result = index::verify(dir.path(), &loaded).unwrap();
    assert_eq!(result.changed, ["wifi/vendor/net.sys"]);
    assert_eq!(result.missing, ["wifi/vendor/net.inf"]);
    assert_eq!(result.unexpected, ["wifi/vendor/extra.dll"]);
}

#[test]
fn rejects_paths_that_escape_the_library() {
    let dir = library();
    let mut loaded = built(dir.path());
    let entry = loaded.files["wifi/vendor/net.sys"].clone();
    for path in ["../outside", "/etc/passwd", "C:/Windows/x", "a\\..\\b"] {
        loaded.files.insert(path.to_owned(), entry.clone());
    }
    let result = index::verify(dir.path(), &loaded).unwrap();
    assert_eq!(result.unsafe_paths.len(), 4);
    assert!(result.missing.is_empty());
}

#[test]
fn rejects_unknown_schema() {
    let dir = library();
    built(dir.path());
    let path = dir.path().join(index::INDEX_FILE);
    let text = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        text.replace("\"schema_version\": 1", "\"schema_version\": 9"),
    )
    .unwrap();
    assert!(matches!(
        index::load(dir.path()),
        Err(index::IndexError::UnsupportedSchema(9))
    ));
}

#[test]
fn library_candidates_come_from_index() {
    let dir = library();
    let loaded = built(dir.path());
    let candidates = drivward_core::library::candidates(&loaded);
    let x64 = candidates.iter().find(|c| c.architecture == "x64").unwrap();
    assert!(x64
        .hardware_ids
        .contains(&"PCI\\VEN_10EC&DEV_8168".to_owned()));
    assert_eq!(x64.min_build, Some(19041));
    assert!(candidates.iter().any(|c| c.architecture == "arm64"));
}

#[test]
fn verifies_a_single_folder() {
    let dir = library();
    let loaded = built(dir.path());
    assert!(index::verify_folder(dir.path(), &loaded, "wifi/vendor")
        .unwrap()
        .is_valid());
    fs::write(dir.path().join("wifi/vendor/net.sys"), b"altered").unwrap();
    fs::write(dir.path().join("wifi/vendor/extra.dll"), b"x").unwrap();
    let result = index::verify_folder(dir.path(), &loaded, "wifi/vendor").unwrap();
    assert_eq!(result.changed, ["wifi/vendor/net.sys"]);
    assert_eq!(result.unexpected, ["wifi/vendor/extra.dll"]);
}

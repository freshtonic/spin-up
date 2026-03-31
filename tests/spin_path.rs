use std::fs;
use std::str::FromStr;

use spin_up::spin_path::{SpinPath, SpinPathError};
use tempfile::TempDir;

#[test]
fn test_spin_path_from_single_directory() {
    let tmp = TempDir::new().unwrap();
    let path_str = tmp.path().to_str().unwrap();

    let spin_path = SpinPath::from_str(path_str).unwrap();
    assert_eq!(spin_path.dirs(), &[tmp.path().to_path_buf()]);
}

#[test]
fn test_spin_path_from_multiple_directories() {
    let tmp1 = TempDir::new().unwrap();
    let tmp2 = TempDir::new().unwrap();
    let path_str = format!(
        "{}:{}",
        tmp1.path().to_str().unwrap(),
        tmp2.path().to_str().unwrap()
    );

    let spin_path = SpinPath::from_str(&path_str).unwrap();
    assert_eq!(
        spin_path.dirs(),
        &[tmp1.path().to_path_buf(), tmp2.path().to_path_buf()]
    );
}

#[test]
fn test_spin_path_nonexistent_directory_is_error() {
    let result = SpinPath::from_str("/nonexistent/path/that/does/not/exist");
    assert!(result.is_err());
}

#[test]
fn test_spin_path_empty_string_is_error() {
    let result = SpinPath::from_str("");
    assert!(result.is_err());
}

#[test]
fn test_spin_path_skips_empty_segments() {
    let tmp = TempDir::new().unwrap();
    let path_str = format!("{}::", tmp.path().to_str().unwrap());

    let spin_path = SpinPath::from_str(&path_str).unwrap();
    assert_eq!(spin_path.dirs(), &[tmp.path().to_path_buf()]);
}

#[test]
fn test_resolve_module_finds_spin_file() {
    let tmp = TempDir::new().unwrap();
    let spin_file = tmp.path().join("postgres.spin");
    fs::write(&spin_file, "# placeholder").unwrap();

    let spin_path = SpinPath::from_str(tmp.path().to_str().unwrap()).unwrap();
    let resolved = spin_path.resolve("postgres").unwrap();
    assert_eq!(resolved, spin_file);
}

#[test]
fn test_resolve_module_first_match_wins() {
    let tmp1 = TempDir::new().unwrap();
    let tmp2 = TempDir::new().unwrap();
    let file1 = tmp1.path().join("postgres.spin");
    let file2 = tmp2.path().join("postgres.spin");
    fs::write(&file1, "# first").unwrap();
    fs::write(&file2, "# second").unwrap();

    let path_str = format!(
        "{}:{}",
        tmp1.path().to_str().unwrap(),
        tmp2.path().to_str().unwrap()
    );
    let spin_path = SpinPath::from_str(&path_str).unwrap();
    let resolved = spin_path.resolve("postgres").unwrap();
    assert_eq!(resolved, file1);
}

#[test]
fn test_resolve_module_not_found() {
    let tmp = TempDir::new().unwrap();
    let spin_path = SpinPath::from_str(tmp.path().to_str().unwrap()).unwrap();
    let result = spin_path.resolve("nonexistent");
    assert!(matches!(
        result.unwrap_err(),
        SpinPathError::ModuleNotFound(_)
    ));
}

#[test]
fn test_resolve_spin_prefixed_modules_from_disk() {
    let tmp = TempDir::new().unwrap();
    let spin_file = tmp.path().join("spin-custom.spin");
    fs::write(&spin_file, "# placeholder").unwrap();

    let spin_path = SpinPath::from_str(tmp.path().to_str().unwrap()).unwrap();
    let result = spin_path.resolve("spin-custom");
    assert_eq!(result.unwrap(), spin_file);
}

#[test]
fn test_resolve_allows_spin_core_prefix() {
    let tmp = TempDir::new().unwrap();
    let spin_path = SpinPath::from_str(tmp.path().to_str().unwrap()).unwrap();
    let result = spin_path.resolve("spin-core-types");
    // spin-core-* modules are built-in, not on disk.
    // Should be ModuleNotFound, NOT ReservedPrefix.
    assert!(matches!(
        result.unwrap_err(),
        SpinPathError::ModuleNotFound(_)
    ));
}

use std::str::FromStr;

use spin_up::spin_path::SpinPath;
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

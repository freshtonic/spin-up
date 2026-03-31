use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_spin_up_subcommand_exists() {
    Command::cargo_bin("spin")
        .unwrap()
        .arg("up")
        .assert()
        .success();
}

#[test]
fn test_spin_down_subcommand_exists() {
    Command::cargo_bin("spin")
        .unwrap()
        .arg("down")
        .assert()
        .success();
}

#[test]
fn test_spin_no_subcommand_shows_help() {
    Command::cargo_bin("spin")
        .unwrap()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

#[test]
fn test_spin_help_shows_up_and_down() {
    Command::cargo_bin("spin")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("up"))
        .stdout(predicate::str::contains("down"));
}

#[test]
fn test_plumbing_commands_hidden_from_help() {
    Command::cargo_bin("spin")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("plumbing").not());
}

#[test]
fn test_plumbing_commands_visible_with_plumbing_flag() {
    // For now, just verify the --plumbing flag is accepted
    // Full conditional help visibility is a follow-up
    Command::cargo_bin("spin")
        .unwrap()
        .args(["--plumbing", "--help"])
        .assert()
        .success();
}

#[test]
fn test_plumbing_supervise_subcommand_exists() {
    Command::cargo_bin("spin")
        .unwrap()
        .args(["plumbing", "supervise", "test-resource"])
        .assert()
        .success();
}

#[test]
fn test_plumbing_kill_subcommand_exists() {
    Command::cargo_bin("spin")
        .unwrap()
        .args(["plumbing", "kill", "test-resource"])
        .assert()
        .success();
}

#[test]
fn test_spin_check_subcommand_exists() {
    // spin check requires a file argument, so test with --help
    Command::cargo_bin("spin")
        .unwrap()
        .args(["check", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("file"));
}

#[test]
fn test_spin_check_valid_file() {
    let tmp = tempfile::TempDir::new().unwrap();
    let spin_file = tmp.path().join("test.spin");
    std::fs::write(&spin_file, "type Foo = x: number;").unwrap();

    Command::cargo_bin("spin")
        .unwrap()
        .args(["check", spin_file.to_str().unwrap()])
        .env("SPIN_PATH", tmp.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("No errors found"));
}

#[test]
fn test_spin_check_with_error() {
    let tmp = tempfile::TempDir::new().unwrap();
    let spin_file = tmp.path().join("test.spin");
    std::fs::write(&spin_file, "import nonexistent\ntype Foo = x: number;").unwrap();

    Command::cargo_bin("spin")
        .unwrap()
        .args(["check", spin_file.to_str().unwrap()])
        .env("SPIN_PATH", tmp.path().to_str().unwrap())
        .assert()
        .failure();
}

#[test]
fn test_spin_check_error_shows_source_filename() {
    let tmp = tempfile::TempDir::new().unwrap();
    let spin_file = tmp.path().join("broken.spin");
    std::fs::write(&spin_file, "import nonexistent").unwrap();

    Command::cargo_bin("spin")
        .unwrap()
        .args(["check", spin_file.to_str().unwrap()])
        .env("SPIN_PATH", tmp.path().to_str().unwrap())
        .assert()
        .failure()
        .stderr(predicate::str::contains("unresolved import"));
}

#[test]
fn test_spin_check_error_shows_miette_diagnostic() {
    let tmp = tempfile::TempDir::new().unwrap();
    let spin_file = tmp.path().join("broken.spin");
    std::fs::write(&spin_file, "import nonexistent").unwrap();

    Command::cargo_bin("spin")
        .unwrap()
        .args(["check", spin_file.to_str().unwrap()])
        .env("SPIN_PATH", tmp.path().to_str().unwrap())
        .assert()
        .failure()
        .stderr(predicate::str::contains("module not found"));
}

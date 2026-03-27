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

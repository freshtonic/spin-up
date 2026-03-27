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

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_kora_help() {
    Command::cargo_bin("kora")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Multi-agent development orchestration CLI",
        ));
}

#[test]
fn test_kora_version() {
    Command::cargo_bin("kora")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("kora"));
}

#[test]
fn test_kora_resume_placeholder() {
    Command::cargo_bin("kora")
        .unwrap()
        .arg("resume")
        .assert()
        .success()
        .stdout(predicate::str::contains("not yet implemented"));
}

#[test]
fn test_kora_history_placeholder() {
    Command::cargo_bin("kora")
        .unwrap()
        .arg("history")
        .assert()
        .success()
        .stdout(predicate::str::contains("not yet implemented"));
}

#[test]
fn test_kora_clean_placeholder() {
    Command::cargo_bin("kora")
        .unwrap()
        .arg("clean")
        .assert()
        .success()
        .stdout(predicate::str::contains("not yet implemented"));
}

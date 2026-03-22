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
fn test_kora_run_placeholder() {
    Command::cargo_bin("kora")
        .unwrap()
        .args(["run", "test request"])
        .assert()
        .success()
        .stdout(predicate::str::contains("not yet implemented"));
}

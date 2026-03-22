use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

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
fn test_kora_resume_no_runs() {
    let tmp = TempDir::new().unwrap();
    Command::cargo_bin("kora")
        .unwrap()
        .arg("resume")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("no interrupted runs"));
}

#[test]
fn test_kora_history_no_runs() {
    let tmp = TempDir::new().unwrap();
    Command::cargo_bin("kora")
        .unwrap()
        .arg("history")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("no run history"));
}

#[test]
fn test_kora_clean_no_runs() {
    let tmp = TempDir::new().unwrap();
    Command::cargo_bin("kora")
        .unwrap()
        .arg("clean")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(
            predicate::str::contains("no run data to clean").or(predicate::str::contains(
                "no completed or failed runs to clean",
            )),
        );
}

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_launch_https_url() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["launch", "--no-launch", "https://example.com"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "URL validated: https://example.com/",
        ));
}

#[test]
fn test_launch_system_default_warning() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args([
        "launch",
        "--temp-profile",
        "--no-launch",
        "https://example.com",
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains("require specifying a browser"));
}

#[test]
fn test_browser_list() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["browser", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Detected browsers:"));
}

#[test]
fn test_browser_check_not_found() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["browser", "check", "definitely-not-installed"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("not found"));
}

#[test]
fn test_help_commands() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("launch"))
        .stdout(predicate::str::contains("browser"))
        .stdout(predicate::str::contains("profile"));
}

#[test]
fn test_launch_help() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["launch", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Open URLs in browsers"));
}

#[test]
fn test_browser_help() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["browser", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Manage browsers"));
}

#[test]
fn test_profile_help() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["profile", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Manage browser profiles"));
}

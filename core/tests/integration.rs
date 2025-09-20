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
        .stderr(predicate::str::contains("Detected browsers:"));
}

#[test]
fn test_browser_check_not_found() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["browser", "check", "definitely-not-installed"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
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

// ============================================================================
// Flag Conflict Tests
// ============================================================================

fn assert_conflict(args: &[&str]) {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    let mut full_args = vec!["launch", "https://example.com"];
    full_args.extend_from_slice(args);
    cmd.args(full_args)
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_browser_selection_conflicts() {
    // --system-default conflicts
    assert_conflict(&["--system-default", "--browser", "chrome"]);
    assert_conflict(&["--system-default", "--channel", "beta"]);

    // --no-system-default conflicts
    assert_conflict(&["--no-system-default", "--system-default"]);
    assert_conflict(&["--no-system-default", "--browser", "firefox"]);
    assert_conflict(&["--no-system-default", "--channel", "stable"]);
}

#[test]
fn test_profile_type_conflicts() {
    // --profile conflicts
    assert_conflict(&["--profile", "Work", "--temp-profile"]);
    assert_conflict(&["--profile", "Dev", "--guest"]);

    // --user-dir conflicts
    assert_conflict(&["--user-dir", "/tmp/custom", "--temp-profile"]);
    assert_conflict(&["--user-dir", "/tmp/test", "--guest"]);

    // --temp-profile conflicts
    assert_conflict(&["--temp-profile", "--guest"]);

    // Multiple conflicts
    assert_conflict(&["--profile", "Work", "--temp-profile", "--guest"]);
}

#[test]
fn test_complex_multi_category_conflicts() {
    assert_conflict(&[
        "--system-default",
        "--browser",
        "chrome",
        "--profile",
        "Work",
        "--temp-profile",
    ]);
}

// ============================================================================
// JSON Format Flag Conflict Tests
// ============================================================================

#[test]
fn test_json_format_conflict_error_structure() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args([
        "launch",
        "https://example.com",
        "--format",
        "json",
        "--system-default",
        "--browser",
        "chrome",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("cannot be used with"));
}

fn assert_success(args: &[&str]) {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    let mut full_args = vec!["launch", "https://example.com"];
    full_args.extend_from_slice(args);
    full_args.push("--no-launch");
    cmd.args(full_args).assert().success();
}

#[test]
fn test_valid_flag_combinations() {
    // Valid combinations should work
    assert_success(&["--browser", "chrome", "--profile", "Work"]);
    assert_success(&["--system-default"]);
    assert_success(&["--no-system-default"]);
    assert_success(&["--temp-profile"]);
    assert_success(&["--guest"]);
}

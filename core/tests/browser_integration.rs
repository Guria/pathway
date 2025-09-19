use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

/// Checks whether the given browser is available by running `pathway browser check <browser>`.
///
/// Returns `true` when the invoked command exits successfully; returns `false` if the command
/// fails, or if locating/running the test binary or capturing output fails. Intended for use in
/// tests to conditionally skip browser-dependent cases.
///
/// # Examples
///
/// ```
/// let available = is_browser_available("chrome");
/// // `available` will be true if `pathway browser check chrome` succeeds, otherwise false.
/// let _ = available;
/// ```
fn is_browser_available(browser: &str) -> bool {
    Command::cargo_bin("pathway")
        .unwrap()
        .args(["browser", "check", browser])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Integration test that verifies `pathway launch --browser chrome --no-launch <url>`
/// reports it would launch in Google Chrome.
///
/// Skips the test early if Chrome is not available on the host system.
///
/// # Examples
///
/// ```no_run
/// // Runs the CLI and checks stderr contains the "Would launch in Google Chrome" hint.
/// let mut cmd = assert_cmd::Command::cargo_bin("pathway").unwrap();
/// cmd.args(["launch", "--browser", "chrome", "--no-launch", "https://example.com"])
///     .assert()
///     .success()
///     .stderr(predicate::str::contains("Would launch in Google Chrome"));
/// ```
#[test]
fn test_launch_with_browser() {
    if !is_browser_available("chrome") {
        eprintln!("Skipping test: Chrome is not available on this system");
        return;
    }

    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args([
        "launch",
        "--browser",
        "chrome",
        "--no-launch",
        "https://example.com",
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains("Would launch in Google Chrome"));
}

#[test]
fn test_launch_with_profile() {
    if !is_browser_available("chrome") {
        eprintln!("Skipping test: Chrome is not available on this system");
        return;
    }

    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args([
        "launch",
        "--browser",
        "chrome",
        "--profile",
        "Default",
        "--no-launch",
        "https://example.com",
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains("with profile 'Default'"));
}

#[test]
fn test_launch_temp_profile() {
    if !is_browser_available("chrome") {
        eprintln!("Skipping test: Chrome is not available on this system");
        return;
    }

    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args([
        "launch",
        "--browser",
        "chrome",
        "--temp-profile",
        "--no-launch",
        "https://example.com",
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains("temporary profile"));
}

#[test]
fn test_launch_window_options() {
    if !is_browser_available("chrome") {
        eprintln!("Skipping test: Chrome is not available on this system");
        return;
    }

    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args([
        "launch",
        "--browser",
        "chrome",
        "--new-window",
        "--incognito",
        "--no-launch",
        "https://example.com",
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains(
        "URL validated: https://example.com/",
    ))
    .stderr(predicate::str::contains("Launch skipped (--no-launch)"))
    .stderr(predicate::str::contains("Would launch in Google Chrome"));
}

#[test]
fn test_launch_window_options_json() {
    if !is_browser_available("chrome") {
        eprintln!("Skipping test: Chrome is not available on this system");
        return;
    }

    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args([
        "--format",
        "json",
        "launch",
        "--browser",
        "chrome",
        "--new-window",
        "--incognito",
        "--no-launch",
        "https://example.com",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains(r#""new_window": true"#))
    .stdout(predicate::str::contains(r#""incognito": true"#))
    .stdout(predicate::str::contains(r#""status": "skipped""#))
    .stdout(predicate::str::contains(
        r#""message": "Launch skipped (--no-launch)""#,
    ));
}

/// Verifies that `pathway browser check chrome` reports Chrome is available.
///
/// Skips the test if Chrome isn't detected on the host system.
#[test]
fn test_browser_check() {
    if !is_browser_available("chrome") {
        eprintln!("Skipping test: Chrome is not available on this system");
        return;
    }

    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["browser", "check", "chrome"])
        .assert()
        .success()
        .stderr(predicate::str::contains("is available"));
}

#[test]
fn test_profile_list() {
    if !is_browser_available("chrome") {
        eprintln!("Skipping test: Chrome is not available on this system");
        return;
    }

    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["profile", "--browser", "chrome", "list"])
        .assert()
        .success()
        .stderr(predicate::str::contains("profiles:"));
}

/// Asserts that `pathway profile --browser chrome --format json list` emits JSON with `"action": "list-profiles"`.
///
/// The test is skipped early if Chrome is not available on the system.
///
/// # Examples
///
/// ```
/// // Invoke the CLI and assert the command succeeds (example usage; mirrors the test).
/// let mut cmd = assert_cmd::Command::cargo_bin("pathway").unwrap();
/// cmd.args(["profile", "--browser", "chrome", "--format", "json", "list"])
///     .assert()
///     .success();
/// ```
#[test]
fn test_profile_list_json() {
    if !is_browser_available("chrome") {
        eprintln!("Skipping test: Chrome is not available on this system");
        return;
    }

    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["profile", "--browser", "chrome", "--format", "json", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""action": "list-profiles""#));
}

#[test]
fn test_profile_custom_dir() {
    if !is_browser_available("chrome") {
        eprintln!("Skipping test: Chrome is not available on this system");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let test_profile_path = temp_dir.path().join("test-profile");
    let test_profile_str = test_profile_path
        .to_str()
        .expect("Failed to convert temporary path to string");

    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args([
        "profile",
        "--browser",
        "chrome",
        "--user-dir",
        test_profile_str,
        "list",
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains("profiles:"));

    // temp_dir is automatically cleaned up when it goes out of scope
}

#[test]
fn test_launch_safari_warnings() {
    if !is_browser_available("safari") {
        eprintln!("Skipping test: Safari is not available on this system");
        return;
    }

    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args([
        "launch",
        "--browser",
        "safari",
        "--temp-profile",
        "--no-launch",
        "https://example.com",
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains("Safari does not support"));
}

#[test]
fn test_launch_firefox_guest_warning() {
    if !is_browser_available("firefox") {
        eprintln!("Skipping test: Firefox is not available on this system");
        return;
    }

    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args([
        "launch",
        "--browser",
        "firefox",
        "--guest",
        "--no-launch",
        "https://example.com",
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains(
        "Firefox does not support guest mode",
    ));
}

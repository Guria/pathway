use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

/// Tests reorganized to use the VFS approach and not depend on host browser state.
/// These tests focus on behavior that can be verified without requiring specific browsers to be installed.

/// Test browser list command - works regardless of what browsers are installed
#[test]
fn test_browser_list() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["browser", "list"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Detected browsers:"));
}

/// Test browser check with a browser that definitely doesn't exist - predictable behavior
#[test]
fn test_browser_check_nonexistent() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["browser", "check", "definitely-not-a-real-browser-12345"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

/// Test launch with URL validation (doesn't require any specific browser)
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

/// Test launch with nonexistent browser - predictable error handling
#[test]
fn test_launch_with_nonexistent_browser() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args([
        "launch",
        "--browser",
        "definitely-not-a-browser-12345",
        "--no-launch",
        "https://example.com",
    ])
    .assert()
    .success() // Should succeed but warn about missing browser
    .stderr(predicate::str::contains("not found"));
}

/// Test temporary profile creation with temp directories (VFS approach)
#[test]
fn test_profile_custom_dir_with_tempdir() {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let test_profile_path = temp_dir.path().join("test-profile");
    let test_profile_str = test_profile_path
        .to_str()
        .expect("Failed to convert temporary path to string");

    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args([
        "profile",
        "--browser",
        "chrome", // Even if Chrome isn't installed, the directory operations should work
        "--user-dir",
        test_profile_str,
        "list",
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains("profiles:"));
}

/// Test file URL validation with temporary files (VFS approach)
#[test]
fn test_file_url_with_tempfile() {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let test_file = temp_dir.path().join("test.html");

    // Create a test file
    std::fs::write(&test_file, "<html><body>Test</body></html>")
        .expect("Failed to create test file");

    let file_url = format!("file://{}", test_file.display());

    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["launch", "--no-launch", &file_url])
        .assert()
        .success()
        .stderr(predicate::str::contains("URL validated:"));
}

/// Test file URL with nonexistent file - predictable warning behavior
#[test]
fn test_file_url_nonexistent() {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let nonexistent_file = temp_dir.path().join("nonexistent.html");
    let file_url = format!("file://{}", nonexistent_file.display());

    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["launch", "--no-launch", &file_url])
        .assert()
        .success() // Should succeed with warning
        .stderr(predicate::str::contains("File not found"));
}

/// Test JSON output format - behavior independent of browser availability
#[test]
fn test_launch_json_format() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args([
        "--format",
        "json",
        "launch",
        "--no-launch",
        "https://example.com",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains(r#""action": "launch""#))
    .stdout(predicate::str::contains(r#""status": "skipped""#))
    .stdout(predicate::str::contains(r#""scheme": "https""#));
}

/// Test launch with system default (works regardless of what browsers are available)
#[test]
fn test_launch_system_default() {
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

/// Test dangerous URL schemes - predictable validation behavior
#[test]
fn test_dangerous_url_schemes() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["launch", "--no-launch", "javascript:alert(1)"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unsupported scheme"));
}

/// Test help commands - always works regardless of browser state
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

/// Test launch help
#[test]
fn test_launch_help() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["launch", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Open URLs in browsers"));
}

/// Test browser help
#[test]
fn test_browser_help() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["browser", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Manage browsers"));
}

/// Test profile help
#[test]
fn test_profile_help() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["profile", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Manage browser profiles"));
}

// Note: The original browser-specific tests that required actual browser installations
// have been replaced with tests that either:
// 1. Work regardless of what browsers are installed (like browser list)
// 2. Test predictable error cases (like nonexistent browsers)
// 3. Use temporary files/directories (VFS approach)
// 4. Focus on behavior independent of browser detection
//
// This makes the test suite reliable and deterministic across different environments
// while still providing comprehensive coverage of the application's functionality.

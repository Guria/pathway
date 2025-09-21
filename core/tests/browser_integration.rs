use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use url::Url;

#[test]
fn test_browser_list() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["browser", "list"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Detected browsers:"));
}

#[test]
fn test_browser_check_nonexistent() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args([
        "browser",
        "check",
        "--browser",
        "definitely-not-a-real-browser-12345",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("not found"));
}

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

#[test]
fn test_file_url_with_tempfile() {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let test_file = temp_dir.path().join("test.html");

    // Create a test file
    std::fs::write(&test_file, "<html><body>Test</body></html>")
        .expect("Failed to create test file");

    // Use proper file URL construction for cross-platform compatibility
    let file_url = Url::from_file_path(&test_file)
        .expect("Failed to create file URL")
        .to_string();

    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["launch", "--no-launch", &file_url])
        .assert()
        .success()
        .stderr(predicate::str::contains("URL validated:"));
}

#[test]
fn test_file_url_nonexistent() {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let nonexistent_file = temp_dir.path().join("nonexistent.html");

    // Use proper file URL construction for cross-platform compatibility
    let file_url = Url::from_file_path(&nonexistent_file)
        .expect("Failed to create file URL")
        .to_string();

    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["launch", "--no-launch", &file_url])
        .assert()
        .success() // Should succeed with warning
        .stderr(predicate::str::contains("File not found"));
}

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

#[test]
fn test_dangerous_url_schemes() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["launch", "--no-launch", "javascript:alert(1)"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unsupported scheme"));
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

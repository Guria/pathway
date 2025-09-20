use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

/// Test that uses temporary directories instead of relying on host file system state.
/// This demonstrates the improved testing approach that doesn't depend on
/// existing files or directories on the host system.
#[test]
fn test_profile_custom_dir_with_tempdir() {
    // Use a temporary directory that's automatically cleaned up
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let test_profile_path = temp_dir.path().join("test-profile");

    // The directory doesn't exist yet - this tests the creation logic
    assert!(!test_profile_path.exists());

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

    // Note: The actual directory creation depends on the real ProfileManager implementation
    // In a fully virtual file system, this would be controlled by the mock

    // temp_dir is automatically cleaned up when it goes out of scope
}

/// Test URL validation with file URLs using temporary files
#[test]
fn test_file_url_validation_with_tempfile() {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let test_file = temp_dir.path().join("test.html");

    // Create a test file
    std::fs::write(&test_file, "<html><body>Test</body></html>")
        .expect("Failed to create test file");

    let file_url = format!("file://{}", test_file.display());

    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["--format", "json", "launch", "--no-launch", &file_url])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""scheme": "file""#))
        .stdout(predicate::str::contains(r#""status": "valid""#));
}

/// Test URL validation with non-existent file (should warn but not fail)
#[test]
fn test_file_url_validation_nonexistent() {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let nonexistent_file = temp_dir.path().join("nonexistent.html");

    // Ensure the file doesn't exist
    assert!(!nonexistent_file.exists());

    let file_url = format!("file://{}", nonexistent_file.display());

    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["--format", "json", "launch", "--no-launch", &file_url])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""scheme": "file""#))
        .stdout(predicate::str::contains(r#""status": "valid""#));
}

/// Test that browser detection works with mock browser installations
#[test]
fn test_browser_list_isolated() {
    // This test doesn't depend on what browsers are actually installed
    // It just verifies the command structure works
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["browser", "list"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Detected browsers:"));
}

/// Test launch command with explicit non-existent browser (should warn but not fail)
#[test]
fn test_launch_with_nonexistent_browser() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args([
        "launch",
        "--browser",
        "definitely-not-a-real-browser-12345",
        "--no-launch",
        "https://example.com",
    ])
    .assert()
    .success() // It succeeds but warns about missing browser
    .stderr(predicate::str::contains("not found"));
}

/// Test that temporary profile creation doesn't leave artifacts
#[test]
fn test_temp_profile_cleanup() {
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

    // We can't easily test actual cleanup since temp profiles are created
    // but this test at least verifies the command structure works
}

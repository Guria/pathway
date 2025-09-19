use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_valid_https_url() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["--no-launch", "https://example.com"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "URL validated: https://example.com/",
        ));
}

#[test]
fn test_valid_http_url() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["--no-launch", "http://localhost:3000/api"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "URL validated: http://localhost:3000/api",
        ));
}

#[test]
fn test_auto_https_scheme() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["--no-launch", "example.com"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "URL validated: https://example.com/",
        ));
}

#[test]
fn test_file_url() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["--no-launch", "file:///etc/hosts"])
        .assert()
        .success();
}

#[test]
fn test_auto_file_scheme() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["--no-launch", "/tmp"])
        .assert()
        .success()
        .stderr(predicate::str::contains("scheme: file"));
}

#[test]
fn test_multiple_urls() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["--no-launch", "https://a.co", "https://b.co"])
        .assert()
        .success()
        .stderr(predicate::str::contains("https://a.co/"))
        .stderr(predicate::str::contains("https://b.co/"));
}

#[test]
fn test_javascript_scheme_rejected() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["--no-launch", "javascript:alert(1)"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unsupported scheme: javascript"));
}

#[test]
fn test_data_url_rejected() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["--no-launch", "data:text/html,<h1>test</h1>"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unsupported scheme: data"));
}

#[test]
fn test_ftp_scheme_rejected() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["--no-launch", "ftp://example.com"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unsupported scheme: ftp"));
}

#[test]
fn test_invalid_url() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["--no-launch", "not a url at all"])
        .assert()
        .failure();
}

#[test]
fn test_json_output() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["--no-launch", "--format", "json", "https://example.com"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""action": "launch"#))
        .stdout(predicate::str::contains(r#""status": "skipped"#))
        .stdout(predicate::str::contains(r#""is_default": true"#))
        .stdout(predicate::str::contains(r#""status": "valid"#));
}

#[test]
fn test_verbose_mode() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["--no-launch", "--verbose", "example.com"])
        .assert()
        .success()
        .stderr(predicate::str::contains("DEBUG"));
}

#[test]
fn test_mixed_valid_invalid() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["--no-launch", "javascript:alert(1)", "https://valid.com"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unsupported scheme: javascript"))
        .stderr(predicate::str::contains("https://valid.com/"));
}

#[test]
fn test_path_traversal_detection() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["--no-launch", "file:///../../../etc/passwd"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Path traversal detected"));
}

#[test]
fn test_list_browsers_human() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.arg("--list-browsers")
        .assert()
        .success()
        .stdout(predicate::str::contains("Detected browsers:"))
        .stdout(predicate::str::contains("System default:"));
}

#[test]
fn test_check_browser_not_found() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["--check-browser", "definitely-not-installed"])
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "Browser 'definitely-not-installed' not found",
        ));
}

#[test]
fn test_browser_option_warns_and_skips() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["--no-launch", "--browser", "nope", "https://example.com"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Browser 'nope' not found"))
        .stderr(predicate::str::contains("Launch skipped (--no-launch)"));
}

#[test]
fn test_help_flag() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("URL routing agent for Pathway"));
}

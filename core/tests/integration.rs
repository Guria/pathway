use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_valid_https_url() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.arg("https://example.com")
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "URL validated: https://example.com/",
        ));
}

#[test]
fn test_valid_http_url() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.arg("http://localhost:3000/api")
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "URL validated: http://localhost:3000/api",
        ));
}

#[test]
fn test_auto_https_scheme() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.arg("example.com")
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "URL validated: https://example.com/",
        ));
}

#[test]
fn test_file_url() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.arg("file:///etc/hosts").assert().success();
}

#[test]
fn test_auto_file_scheme() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.arg("/tmp")
        .assert()
        .success()
        .stderr(predicate::str::contains("scheme: file"));
}

#[test]
fn test_multiple_urls() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["https://a.co", "https://b.co"])
        .assert()
        .success()
        .stderr(predicate::str::contains("https://a.co/"))
        .stderr(predicate::str::contains("https://b.co/"));
}

#[test]
fn test_javascript_scheme_rejected() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.arg("javascript:alert(1)")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unsupported scheme: javascript"));
}

#[test]
fn test_data_url_rejected() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.arg("data:text/html,<h1>test</h1>")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unsupported scheme: data"));
}

#[test]
fn test_ftp_scheme_rejected() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.arg("ftp://example.com")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unsupported scheme: ftp"));
}

#[test]
fn test_invalid_url() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.arg("not a url at all").assert().failure();
}

#[test]
fn test_json_output() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["--format", "json", "https://example.com"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""status":"valid"#))
        .stdout(predicate::str::contains(r#""scheme":"https"#));
}

#[test]
fn test_verbose_mode() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["--verbose", "example.com"])
        .assert()
        .success()
        .stderr(predicate::str::contains("DEBUG"));
}

#[test]
fn test_mixed_valid_invalid() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.args(["javascript:alert(1)", "https://valid.com"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unsupported scheme: javascript"))
        .stderr(predicate::str::contains("https://valid.com/"));
}

#[test]
fn test_path_traversal_detection() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.arg("file:///../../../etc/passwd")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Path traversal detected"));
}

#[test]
fn test_help_flag() {
    let mut cmd = Command::cargo_bin("pathway").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("URL routing agent for Pathway"));
}

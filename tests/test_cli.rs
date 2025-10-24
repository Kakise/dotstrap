use assert_cmd::Command;

#[test]
fn test_main() {
    Command::cargo_bin("dotstrap")
        .unwrap()
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_dry_run() {
    #[cfg(target_os = "macos")]
    Command::cargo_bin("dotstrap")
        .unwrap()
        .arg("tests/config-brew")
        .arg("--dry-run")
        .assert()
        .success();

    #[cfg(not(target_os = "macos"))]
    Command::cargo_bin("dotstrap")
        .unwrap()
        .arg("tests/config-brew")
        .arg("--dry-run")
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "dotstrap failed: Homebrew is not installed or not executable\n",
        ));
}

#[test]
fn test_dry_run_with_invalid_args() {
    Command::cargo_bin("dotstrap")
        .unwrap()
        .arg("--dry-run")
        .arg("--invalid-flag")
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "unexpected argument '--invalid-flag'",
        ));
}

#[test]
fn test_dry_run_invalid_config() {
    Command::cargo_bin("dotstrap")
        .unwrap()
        .arg("tests/config-invalid")
        .arg("--dry-run")
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "fatal: repository 'tests/config-invalid' does not exist",
        ));
}

#[test]
fn test_main_with_invalid_args() {
    Command::cargo_bin("dotstrap")
        .unwrap()
        .arg("--invalid-flag")
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "unexpected argument '--invalid-flag'",
        ));
}

#[test]
fn test_generate_zsh_completions() {
    Command::cargo_bin("dotstrap")
        .unwrap()
        .arg("--generate-completions")
        .arg("zsh")
        .assert()
        .success()
        .stdout(predicates::str::contains("_dotstrap"));
}

//! Binary entry point that wires the CLI to the library application layer.

#[cfg(not(test))]
use dotstrap::execute_cli;

#[cfg(not(test))]
fn main() {
    std::process::exit(execute_cli(std::env::args()));
}

#[cfg(test)]
mod tests {
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
}

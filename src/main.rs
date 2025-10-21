//! Binary entry point that wires the CLI to the library application layer.

use std::ffi::OsString;

use clap::Parser;

use dotstrap::{Cli, run};

fn main() {
    std::process::exit(execute(std::env::args()));
}

fn execute<I, T>(args: I) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::parse_from(args);
    match run(cli) {
        Ok(report) => {
            if report.dry_run {
                println!(
                    "Dry run complete: {} templates evaluated.",
                    report.rendered.len()
                );
            }
            0
        }
        Err(err) => {
            eprintln!("dotstrap failed: {err}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use tempfile::tempdir;

    fn create_repo() -> tempfile::TempDir {
        let repo = tempdir().unwrap();
        fs::create_dir_all(repo.path().join("templates")).unwrap();
        fs::write(repo.path().join("templates/config.hbs"), "name={{name}}\n").unwrap();
        fs::write(
            repo.path().join("manifest.yaml"),
            "version: 1\ntemplates:\n  - source: templates/config.hbs\n    destination: .config\n",
        )
        .unwrap();
        fs::write(repo.path().join("values.yaml"), "name: sample\n").unwrap();
        repo
    }

    #[test]
    fn execute_returns_success_code_on_dry_run() {
        let repo = create_repo();
        let home = tempdir().unwrap();
        let args = vec![
            "dotstrap".into(),
            repo.path().to_string_lossy().into_owned(),
            "--home".into(),
            home.path().to_string_lossy().into_owned(),
            "--skip-brew".into(),
            "--dry-run".into(),
        ];
        let code = execute(args);
        assert_eq!(code, 0);
    }

    #[test]
    fn execute_returns_error_code_on_failure() {
        let repo = tempdir().unwrap();
        let home = tempdir().unwrap();
        let args = vec![
            "dotstrap".into(),
            repo.path().to_string_lossy().into_owned(),
            "--home".into(),
            home.path().to_string_lossy().into_owned(),
            "--skip-brew".into(),
            "--dry-run".into(),
        ];
        let code = execute(args);
        assert_eq!(code, 1);
    }
}

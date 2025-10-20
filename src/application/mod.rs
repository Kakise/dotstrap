//! Application layer orchestration for the dotstrap workflow.
//!
//! This module wires together the CLI inputs, configuration loading,
//! templating, linking, and optional package installation steps to produce a
//! single [`ExecutionReport`].

use std::path::PathBuf;

use crate::cli::Cli;
use crate::config;
use crate::errors::{DotstrapError, Result};
use crate::infrastructure::command::{CommandExecutor, SystemCommandExecutor};
use crate::infrastructure::{repository, secrets};
use crate::services::{brew, linker, templating};

/// Summary of the operations performed during a dotstrap run.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ExecutionReport {
    /// Template destinations rendered from the manifest.
    pub rendered: Vec<PathBuf>,
    /// Fully qualified paths linked into the target home directory.
    pub linked: Vec<PathBuf>,
    /// Homebrew commands executed or planned.
    pub brew_commands: Vec<String>,
    /// Indicates that the run was executed in dry-run mode.
    pub dry_run: bool,
}

/// Run dotstrap using the system command executor.
pub fn run(cli: Cli) -> Result<ExecutionReport> {
    let executor = SystemCommandExecutor;
    run_with_executor(cli, &executor)
}

/// Run dotstrap using the provided [`CommandExecutor`].
pub fn run_with_executor<E>(cli: Cli, executor: &E) -> Result<ExecutionReport>
where
    E: CommandExecutor,
{
    let home_dir = match cli.home.clone() {
        Some(path) => path,
        None => home::home_dir().ok_or(DotstrapError::HomeNotFound)?,
    };

    let repo = repository::resolve_repository(&cli.source, executor)?;
    let manifest = config::load_manifest(repo.path())?;
    let values = config::load_values(repo.path())?;
    let secrets = secrets::load_secrets(repo.path(), &home_dir)?;
    let context = templating::build_context(&values, &secrets);
    let rendered_set = templating::render_templates(repo.path(), &manifest, &context)?;
    let linked = linker::link_templates(&home_dir, &rendered_set, cli.dry_run)?;
    let rendered_destinations = manifest
        .templates
        .iter()
        .map(|t| t.destination.clone())
        .collect();

    let brew_commands = if cli.skip_brew {
        Vec::new()
    } else {
        match config::load_brew_spec(repo.path())? {
            Some(spec) => brew::install_brew(&spec, executor, cli.dry_run)?,
            None => Vec::new(),
        }
    };

    Ok(ExecutionReport {
        rendered: rendered_destinations,
        linked,
        brew_commands,
        dry_run: cli.dry_run,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::command::RecordingCommandExecutor;
    use std::fs;
    use std::sync::Mutex;

    fn set_env(key: &str, value: &str) {
        unsafe { std::env::set_var(key, value) };
    }

    fn remove_env(key: &str) {
        unsafe { std::env::remove_var(key) };
    }

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn create_repo() -> tempfile::TempDir {
        let repo = tempfile::tempdir().unwrap();
        fs::write(
            repo.path().join("manifest.yaml"),
            r#"
version: 1
templates:
  - source: templates/config.hbs
    destination: .configfile
"#,
        )
        .unwrap();
        fs::create_dir_all(repo.path().join("templates")).unwrap();
        fs::write(
            repo.path().join("templates/config.hbs"),
            "name={{name}} secret={{secrets.token}}",
        )
        .unwrap();
        fs::write(repo.path().join("values.yaml"), "name: sample\n").unwrap();
        fs::create_dir_all(repo.path().join("secrets")).unwrap();
        fs::write(
            repo.path().join("secrets/secrets.yaml"),
            "token:\n  from: env\n  key: DOTSTRAP_TEST_TOKEN\n",
        )
        .unwrap();
        fs::create_dir_all(repo.path().join("brew")).unwrap();
        fs::write(
            repo.path().join("brew/packages.yaml"),
            r#"
formulae:
  - ripgrep
"#,
        )
        .unwrap();
        repo
    }

    fn create_repo_without_brew() -> tempfile::TempDir {
        let repo = create_repo();
        fs::remove_dir_all(repo.path().join("brew")).unwrap();
        repo
    }

    #[test]
    fn end_to_end_dry_run() {
        let repo = create_repo();
        let _guard = ENV_LOCK.lock().unwrap();
        set_env("DOTSTRAP_TEST_TOKEN", "secret");
        let exec = RecordingCommandExecutor::default();
        let home = tempfile::tempdir().unwrap();
        let cli = Cli {
            source: repo.path().to_string_lossy().to_string(),
            home: Some(home.path().to_path_buf()),
            skip_brew: false,
            dry_run: true,
        };
        let report = run_with_executor(cli, &exec).unwrap();
        assert_eq!(report.rendered, vec![PathBuf::from(".configfile")]);
        assert_eq!(report.linked, vec![home.path().join(".configfile")]);
        assert_eq!(
            report.brew_commands,
            vec!["brew update", "brew install ripgrep"]
        );
        remove_env("DOTSTRAP_TEST_TOKEN");
        drop(_guard);
    }

    #[test]
    fn honours_skip_brew() {
        let repo = create_repo();
        let _guard = ENV_LOCK.lock().unwrap();
        set_env("DOTSTRAP_TEST_TOKEN", "secret");
        let exec = RecordingCommandExecutor::default();
        let home = tempfile::tempdir().unwrap();
        let cli = Cli {
            source: repo.path().to_string_lossy().to_string(),
            home: Some(home.path().to_path_buf()),
            skip_brew: true,
            dry_run: true,
        };
        let report = run_with_executor(cli, &exec).unwrap();
        assert!(report.brew_commands.is_empty());
        remove_env("DOTSTRAP_TEST_TOKEN");
        drop(_guard);
    }

    #[test]
    fn falls_back_to_home_directory() {
        let repo = create_repo_without_brew();
        let _guard = ENV_LOCK.lock().unwrap();
        set_env("DOTSTRAP_TEST_TOKEN", "secret");
        let exec = RecordingCommandExecutor::default();
        let cli = Cli {
            source: repo.path().to_string_lossy().to_string(),
            home: None,
            skip_brew: true,
            dry_run: true,
        };
        let report = run_with_executor(cli, &exec).unwrap();
        let default_home = home::home_dir().unwrap();
        assert!(
            report
                .linked
                .iter()
                .all(|path| path.starts_with(&default_home))
        );
        remove_env("DOTSTRAP_TEST_TOKEN");
        drop(_guard);
    }

    #[test]
    fn executes_linking_when_not_dry_run() {
        let repo = create_repo_without_brew();
        let _guard = ENV_LOCK.lock().unwrap();
        set_env("DOTSTRAP_TEST_TOKEN", "secret");
        let exec = RecordingCommandExecutor::default();
        let home = tempfile::tempdir().unwrap();
        let cli = Cli {
            source: repo.path().to_string_lossy().to_string(),
            home: Some(home.path().to_path_buf()),
            skip_brew: true,
            dry_run: false,
        };
        let report = run_with_executor(cli, &exec).unwrap();
        let target = home.path().join(".configfile");
        assert!(target.exists());
        #[cfg(unix)]
        {
            assert!(target.symlink_metadata().unwrap().file_type().is_symlink());
        }
        assert_eq!(report.rendered, vec![PathBuf::from(".configfile")]);
        remove_env("DOTSTRAP_TEST_TOKEN");
        drop(_guard);
    }

    #[test]
    fn handles_missing_brew_manifest() {
        let repo = create_repo_without_brew();
        let _guard = ENV_LOCK.lock().unwrap();
        set_env("DOTSTRAP_TEST_TOKEN", "secret");
        let exec = RecordingCommandExecutor::default();
        let home = tempfile::tempdir().unwrap();
        let cli = Cli {
            source: repo.path().to_string_lossy().to_string(),
            home: Some(home.path().to_path_buf()),
            skip_brew: false,
            dry_run: true,
        };
        let report = run_with_executor(cli, &exec).unwrap();
        assert!(report.brew_commands.is_empty());
        remove_env("DOTSTRAP_TEST_TOKEN");
        drop(_guard);
    }
}

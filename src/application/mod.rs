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
    let Cli {
        source,
        home,
        skip_brew,
        dry_run,
        generate_completions: _,
    } = cli;

    let source = source.expect("source argument is validated by clap");

    let home_dir = match home {
        Some(path) => path,
        None => home::home_dir().ok_or(DotstrapError::HomeNotFound)?,
    };

    let repo = repository::resolve_repository(&source, executor)?;
    let manifest = config::load_manifest(repo.path())?;
    let values = config::load_values(repo.path())?;
    let secrets = secrets::load_secrets(repo.path(), &home_dir)?;
    let context = templating::build_context(&values, &secrets);
    let rendered_set = templating::render_templates(repo.path(), &manifest, &context)?;
    let linked = linker::link_templates(&home_dir, &rendered_set, dry_run)?;
    let rendered_destinations = manifest
        .templates
        .iter()
        .map(|t| t.destination.clone())
        .collect();

    let brew_commands = if skip_brew {
        Vec::new()
    } else {
        match config::load_brew_spec(repo.path())? {
            Some(spec) => brew::install_brew(&spec, executor, dry_run)?,
            None => Vec::new(),
        }
    };

    Ok(ExecutionReport {
        rendered: rendered_destinations,
        linked,
        brew_commands,
        dry_run,
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    struct MockExecutor();

    impl super::CommandExecutor for MockExecutor {
        fn run(&self, _program: &str, _args: &[&str]) -> super::Result<()> {
            Ok(())
        }
    }

    fn create_test_cli(
        source: Option<&str>,
        home_dir: Option<std::path::PathBuf>,
        brew: bool,
    ) -> super::Cli {
        super::Cli {
            source: Some("tests/".to_owned() + source.unwrap_or("empty-config")),
            home: home_dir.to_owned(),
            skip_brew: brew,
            dry_run: true,
            generate_completions: None,
        }
    }

    #[test]
    fn test_run() {
        let cli = create_test_cli(None, None, true);
        let result = super::run(cli);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_with_executor() {
        let executor = MockExecutor();
        let result = super::run_with_executor(
            create_test_cli(None, Some(PathBuf::from("/home/user")), true),
            &executor,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_with_executor_brew_enabled() {
        let executor = MockExecutor();
        let result =
            super::run_with_executor(create_test_cli(Some("config-brew"), None, false), &executor);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_with_executor_no_brew() {
        let executor = MockExecutor();
        let result = super::run_with_executor(create_test_cli(None, None, false), &executor);
        assert!(result.is_ok());
    }
}

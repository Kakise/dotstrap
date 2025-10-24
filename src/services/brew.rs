//! Service responsible for installing Homebrew taps, formulae, and casks.

use crate::config::BrewSpec;
use crate::errors::{DotstrapError, Result};
use crate::infrastructure::command::CommandExecutor;

/// Prepare and optionally execute the Homebrew commands required by the spec.
pub fn install_brew(
    spec: &BrewSpec,
    executor: &dyn CommandExecutor,
    dry_run: bool,
) -> Result<Vec<String>> {
    let mut executed = Vec::new();
    if spec.taps.is_empty() && spec.formulae.is_empty() && spec.casks.is_empty() {
        return Ok(executed);
    }
    ensure_available(executor)?;
    maybe_run(executor, dry_run, &mut executed, "brew", &["update"])?;
    for tap in &spec.taps {
        maybe_run(
            executor,
            dry_run,
            &mut executed,
            "brew",
            &["tap", tap, "--force"],
        )?;
    }
    for formula in &spec.formulae {
        maybe_run(
            executor,
            dry_run,
            &mut executed,
            "brew",
            &["install", formula],
        )?;
    }
    for cask in &spec.casks {
        maybe_run(
            executor,
            dry_run,
            &mut executed,
            "brew",
            &["install", "--cask", cask],
        )?;
    }
    Ok(executed)
}

fn ensure_available(executor: &dyn CommandExecutor) -> Result<()> {
    executor
        .run("brew", &["--version"])
        .map_err(|_| DotstrapError::BrewUnavailable)
}

fn maybe_run(
    executor: &dyn CommandExecutor,
    dry_run: bool,
    log: &mut Vec<String>,
    program: &str,
    args: &[&str],
) -> Result<()> {
    let command_string = format!("{program} {}", args.join(" "));
    log.push(command_string);
    if dry_run {
        return Ok(());
    }
    executor.run(program, args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BrewSpec;
    use crate::errors::DotstrapError;
    use crate::infrastructure::command::RecordingCommandExecutor;

    #[test]
    fn install_brew_returns_empty_when_spec_is_empty() {
        let executor = RecordingCommandExecutor::default();
        let spec = BrewSpec::default();

        let executed =
            install_brew(&spec, &executor, false).expect("expected success for empty spec");

        assert!(executed.is_empty(), "no commands should be logged");
        assert!(
            executor.calls().is_empty(),
            "executor should not be invoked when spec is empty"
        );
    }

    #[test]
    fn install_brew_executes_commands_in_order() {
        let executor = RecordingCommandExecutor::default();
        let spec = BrewSpec {
            taps: vec!["homebrew/cask".into()],
            formulae: vec!["fzf".into()],
            casks: vec!["iterm2".into()],
        };

        let executed =
            install_brew(&spec, &executor, false).expect("expected installation to succeed");

        let expected_logged = vec![
            "brew update".to_string(),
            "brew tap homebrew/cask --force".to_string(),
            "brew install fzf".to_string(),
            "brew install --cask iterm2".to_string(),
        ];
        assert_eq!(executed, expected_logged);

        let calls = executor.calls();
        assert_eq!(
            calls.len(),
            1 + expected_logged.len(),
            "brew should be invoked for version check plus each command"
        );
        assert_eq!(
            calls[0],
            ("brew".to_string(), vec!["--version".to_string()]),
            "first call should verify brew availability"
        );
        assert_eq!(
            calls[1],
            ("brew".to_string(), vec!["update".to_string()]),
            "brew update should run after availability check"
        );
        assert_eq!(
            calls[2],
            (
                "brew".to_string(),
                vec!["tap".to_string(), "homebrew/cask".to_string()]
            )
        );
        assert_eq!(
            calls[3],
            (
                "brew".to_string(),
                vec!["install".to_string(), "fzf".to_string()]
            )
        );
        assert_eq!(
            calls[4],
            (
                "brew".to_string(),
                vec![
                    "install".to_string(),
                    "--cask".to_string(),
                    "iterm2".to_string()
                ]
            )
        );
    }

    #[test]
    fn install_brew_returns_brew_unavailable_when_version_check_fails() {
        let executor = RecordingCommandExecutor::with_failure("brew");
        let spec = BrewSpec {
            taps: vec!["tap/failed".into()],
            formulae: vec![],
            casks: vec![],
        };

        let error =
            install_brew(&spec, &executor, false).expect_err("expected BrewUnavailable error");

        assert!(
            matches!(error, DotstrapError::BrewUnavailable),
            "error should map to BrewUnavailable"
        );
        let calls = executor.calls();
        assert_eq!(
            calls.len(),
            1,
            "only the availability check should be attempted"
        );
        assert_eq!(
            calls[0],
            ("brew".to_string(), vec!["--version".to_string()])
        );
    }
}

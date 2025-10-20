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
        maybe_run(executor, dry_run, &mut executed, "brew", &["tap", tap])?;
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
    use crate::infrastructure::command::RecordingCommandExecutor;

    #[test]
    fn skips_when_empty() {
        let exec = RecordingCommandExecutor::default();
        let spec = BrewSpec::default();
        let commands = install_brew(&spec, &exec, false).unwrap();
        assert!(commands.is_empty());
    }

    #[test]
    fn records_and_executes_commands() {
        let exec = RecordingCommandExecutor::default();
        let spec = BrewSpec {
            taps: vec!["homebrew/cask".into()],
            formulae: vec!["ripgrep".into()],
            casks: vec!["iterm2".into()],
        };
        let commands = install_brew(&spec, &exec, true).unwrap();
        assert_eq!(
            commands,
            vec![
                "brew update".to_string(),
                "brew tap homebrew/cask".to_string(),
                "brew install ripgrep".to_string(),
                "brew install --cask iterm2".to_string()
            ]
        );
    }

    #[test]
    fn fails_when_brew_missing() {
        let exec = RecordingCommandExecutor::with_failure("brew");
        let spec = BrewSpec {
            taps: vec!["homebrew/cask".into()],
            formulae: vec![],
            casks: vec![],
        };
        let err = install_brew(&spec, &exec, false).unwrap_err();
        matches!(err, DotstrapError::BrewUnavailable);
    }

    #[test]
    fn executes_commands_when_not_dry_run() {
        let exec = RecordingCommandExecutor::default();
        let spec = BrewSpec {
            taps: vec![],
            formulae: vec!["ripgrep".into()],
            casks: vec![],
        };
        let commands = install_brew(&spec, &exec, false).unwrap();
        assert_eq!(commands, vec!["brew update", "brew install ripgrep"]);
        let calls = exec.calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0], ("brew".into(), vec!["--version".into()]));
        assert_eq!(calls[1], ("brew".into(), vec!["update".into()]));
        assert_eq!(
            calls[2],
            ("brew".into(), vec!["install".into(), "ripgrep".into()])
        );
    }
}

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

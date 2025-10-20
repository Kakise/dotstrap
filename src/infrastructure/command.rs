//! Command execution abstractions used by services that invoke external tools.

use std::process::Command;

use crate::errors::{DotstrapError, Result};

/// Generic abstraction around spawning commands, enabling mocks during tests.
pub trait CommandExecutor {
    fn run(&self, program: &str, args: &[&str]) -> Result<()>;
}

/// Command executor that proxies to [`std::process::Command`].
#[derive(Default)]
pub struct SystemCommandExecutor;

impl CommandExecutor for SystemCommandExecutor {
    fn run(&self, program: &str, args: &[&str]) -> Result<()> {
        let mut cmd = Command::new(program);
        cmd.args(args);
        let status = cmd
            .status()
            .map_err(|err| DotstrapError::CommandIo(program.to_string(), err))?;
        if status.success() {
            Ok(())
        } else {
            let code = status.code().unwrap_or(-1);
            Err(DotstrapError::CommandFailed {
                program: program.to_string(),
                status: code,
            })
        }
    }
}

/// A command executor used for tests that records invocations.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Default)]
pub struct RecordingCommandExecutor {
    calls: std::cell::RefCell<Vec<(String, Vec<String>)>>,
    fail_on: std::cell::RefCell<Option<String>>,
}

#[cfg_attr(not(test), allow(dead_code))]
impl RecordingCommandExecutor {
    pub fn with_failure(program: &str) -> Self {
        RecordingCommandExecutor {
            calls: std::cell::RefCell::new(Vec::new()),
            fail_on: std::cell::RefCell::new(Some(program.to_string())),
        }
    }

    pub fn calls(&self) -> Vec<(String, Vec<String>)> {
        self.calls.borrow().clone()
    }
}

impl CommandExecutor for RecordingCommandExecutor {
    fn run(&self, program: &str, args: &[&str]) -> Result<()> {
        self.calls.borrow_mut().push((
            program.to_string(),
            args.iter().map(|s| s.to_string()).collect(),
        ));
        if self
            .fail_on
            .borrow()
            .as_ref()
            .map(|p| p == program)
            .unwrap_or(false)
        {
            Err(DotstrapError::CommandFailed {
                program: program.to_string(),
                status: 1,
            })
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn system_executor_runs_true() {
        let exec = SystemCommandExecutor::default();
        exec.run("true", &[]).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn system_executor_reports_failure() {
        let exec = SystemCommandExecutor::default();
        let err = exec.run("false", &[]).unwrap_err();
        matches!(err, DotstrapError::CommandFailed { .. });
    }
}

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
    use crate::errors::DotstrapError;

    #[cfg(windows)]
    fn success_command() -> (&'static str, &'static [&'static str]) {
        ("cmd", &["/C", "exit 0"])
    }

    #[cfg(not(windows))]
    fn success_command() -> (&'static str, &'static [&'static str]) {
        ("sh", &["-c", "exit 0"])
    }

    #[cfg(windows)]
    fn failure_command() -> (&'static str, &'static [&'static str], i32) {
        ("cmd", &["/C", "exit 1"], 1)
    }

    #[cfg(not(windows))]
    fn failure_command() -> (&'static str, &'static [&'static str], i32) {
        ("sh", &["-c", "exit 42"], 42)
    }

    #[test]
    fn system_command_executor_returns_ok_on_success() {
        let executor = SystemCommandExecutor;
        let (program, args) = success_command();

        let result = executor.run(program, args);

        assert!(result.is_ok(), "expected success running `{program}`");
    }

    #[test]
    fn system_command_executor_returns_command_failed_error_on_non_zero_exit() {
        let executor = SystemCommandExecutor;
        let (program, args, expected_status) = failure_command();

        let error = executor
            .run(program, args)
            .expect_err("expected failure running command");

        assert!(
            matches!(error, DotstrapError::CommandFailed { program, status } if program == program && status == expected_status)
        );
    }

    #[test]
    fn recording_executor_tracks_invocations() {
        let executor = RecordingCommandExecutor::default();

        executor.run("git", &["status", "--short"]).unwrap();
        executor.run("brew", &["update"]).unwrap();

        let calls = executor.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(
            calls[0],
            (
                "git".to_string(),
                vec!["status".to_string(), "--short".to_string()]
            )
        );
        assert_eq!(calls[1], ("brew".to_string(), vec!["update".to_string()]));
    }

    #[test]
    fn recording_executor_can_be_configured_to_fail_specific_program() {
        let executor = RecordingCommandExecutor::with_failure("git");

        let error = executor
            .run("git", &["status"])
            .expect_err("expected failure for configured program");

        assert!(
            matches!(error, DotstrapError::CommandFailed { program, status } if program == "git" && status == 1)
        );

        let calls = executor.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "git");
        assert_eq!(calls[0].1, vec!["status".to_string()]);
    }
}

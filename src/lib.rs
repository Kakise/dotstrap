#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! Core library entry point for dotstrap.

use clap::{CommandFactory, Parser};
use clap_complete::generate;
use std::io::{self, Write};

pub mod application;
pub mod cli;
pub mod config;
pub mod errors;
pub mod infrastructure;
pub mod services;

pub use application::{ExecutionReport, run, run_with_executor};
pub use cli::Cli;
pub use errors::{DotstrapError, Result};

/// Execute the CLI entrypoint using the provided iterator of arguments.
pub fn execute_cli<I, T>(args: I) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let args_vec: Vec<std::ffi::OsString> = args.into_iter().map(|arg| arg.into()).collect();
    let cli = match Cli::try_parse_from(args_vec) {
        Ok(cli) => cli,
        Err(error) => {
            let _ = error.print();
            return if error.use_stderr() { 1 } else { 0 };
        }
    };

    if let Some(shell) = cli.generate_completions {
        let mut command = Cli::command();
        command.set_bin_name("dotstrap");
        let mut stdout = io::stdout();
        generate(shell, &mut command, "dotstrap", &mut stdout);
        if let Err(err) = stdout.flush() {
            eprintln!("failed to flush completions to stdout: {err}");
            return 1;
        }
        return 0;
    }

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

#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! Core library entry point for dotstrap.

#[cfg(not(test))]
use clap::Parser;

pub mod application;
pub mod cli;
pub mod config;
pub mod errors;
pub mod infrastructure;
pub mod services;

#[cfg(not(test))]
pub use application::{ExecutionReport, run, run_with_executor};
pub use cli::Cli;
pub use errors::{DotstrapError, Result};

/// Execute the CLI entrypoint using the provided iterator of arguments.
#[cfg(not(test))]
pub fn execute_cli<I, T>(args: I) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
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

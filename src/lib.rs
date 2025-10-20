#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! Core library entry point for dotstrap.

pub mod application;
pub mod cli;
pub mod config;
pub mod errors;
pub mod infrastructure;
pub mod services;

pub use application::{ExecutionReport, run, run_with_executor};
pub use cli::Cli;
pub use errors::{DotstrapError, Result};

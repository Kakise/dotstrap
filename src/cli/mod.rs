//! Command-line interface definition for dotstrap.

use std::path::PathBuf;

use clap::Parser;

/// Command line interface definition for dotstrap.
#[derive(Debug, Parser)]
#[command(author, version, about = "Synchronise dotfiles from a template repository.", long_about = None)]
pub struct Cli {
    /// Git repository URL or local path containing dotstrap manifest and templates.
    pub source: String,

    /// Override the target home directory (defaults to the current user's home).
    #[arg(long, value_name = "PATH")]
    pub home: Option<PathBuf>,

    /// Skip installing Homebrew packages.
    #[arg(long)]
    pub skip_brew: bool,

    /// Print the operations without changing the system.
    #[arg(long)]
    pub dry_run: bool,
}

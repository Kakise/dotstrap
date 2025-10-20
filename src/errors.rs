//! Shared error type exposed across the crate.

use std::path::PathBuf;

use thiserror::Error;

/// Error type covering every failure mode of the dotstrap workflow.
#[derive(Debug, Error)]
pub enum DotstrapError {
    #[error("failed to determine home directory")]
    HomeNotFound,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("command `{program}` failed with status {status}")]
    CommandFailed { program: String, status: i32 },

    #[error("failed to execute command `{0}`: {1}")]
    CommandIo(String, #[source] std::io::Error),

    #[error("failed to parse yaml file `{path}`: {source}")]
    Yaml {
        source: serde_yaml::Error,
        path: PathBuf,
    },

    #[error("template render failure for `{path}`: {source}")]
    Template {
        source: handlebars::RenderError,
        path: PathBuf,
    },

    #[error("template compilation failure for `{path}`: {source}")]
    TemplateCompile {
        source: handlebars::TemplateError,
        path: PathBuf,
    },

    #[error("manifest `{0}` is missing templates section")]
    ManifestMissingTemplates(PathBuf),

    #[error("manifest `{path}` declares unsupported version {version}")]
    UnsupportedManifestVersion { path: PathBuf, version: u8 },

    #[error("secret `{name}` is not available from {provider}")]
    MissingSecret { name: String, provider: String },

    #[error("Homebrew is not installed or not executable")]
    BrewUnavailable,

    #[error("brew manifest file `{0}` not found")]
    BrewManifestMissing(PathBuf),
}

pub type Result<T> = std::result::Result<T, DotstrapError>;

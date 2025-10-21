//! Repository resolution utilities for local paths and remote git sources.

use std::path::{Path, PathBuf};

use tempfile::TempDir;

use super::command::CommandExecutor;
use crate::errors::Result;

/// Handle representing a resolved configuration repository.
pub struct RepoHandle {
    pub path: PathBuf,
    _tempdir: Option<TempDir>,
}

impl RepoHandle {
    /// Path to the resolved repository contents.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Resolve the repository described by the user-provided source.
pub fn resolve_repository(source: &str, executor: &dyn CommandExecutor) -> Result<RepoHandle> {
    let path = PathBuf::from(source);
    if path.exists() {
        return Ok(RepoHandle {
            path: path.canonicalize()?,
            _tempdir: None,
        });
    }
    clone_remote(source, executor)
}

fn clone_remote(source: &str, executor: &dyn CommandExecutor) -> Result<RepoHandle> {
    let tempdir = TempDir::new()?;
    let target_dir = tempdir.path().join("repo");
    let target_str = target_dir.to_string_lossy().to_string();
    executor.run("git", &["clone", "--depth", "1", source, &target_str])?;
    Ok(RepoHandle {
        path: target_dir,
        _tempdir: Some(tempdir),
    })
}

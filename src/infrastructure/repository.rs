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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::command::RecordingCommandExecutor;

    #[test]
    fn resolve_repository_returns_canonical_path_for_existing_directory() {
        let executor = RecordingCommandExecutor::default();
        let tempdir = tempfile::tempdir().expect("failed to create temporary directory");

        let handle = resolve_repository(tempdir.path().to_str().unwrap(), &executor)
            .expect("expected repository resolution to succeed");

        let expected = tempdir
            .path()
            .canonicalize()
            .expect("failed to canonicalize tempdir path");
        assert_eq!(handle.path(), expected.as_path());
        assert!(handle.path().exists());
        assert!(executor.calls().is_empty());
    }

    #[test]
    fn resolve_repository_clones_remote_source() {
        let executor = RecordingCommandExecutor::default();
        let source = "git@github.com:example/dotstrap-test.git";

        let handle = resolve_repository(source, &executor)
            .expect("expected remote repository resolution to succeed");

        let calls = executor.calls();
        assert_eq!(calls.len(), 1);
        let (program, args) = &calls[0];
        assert_eq!(program, "git");
        assert_eq!(args.len(), 5);
        assert_eq!(args[0], "clone");
        assert_eq!(args[1], "--depth");
        assert_eq!(args[2], "1");
        assert_eq!(args[3], source);
        let expected_target = handle.path().display().to_string();
        assert_eq!(args[4], expected_target);

        assert!(handle.path().ends_with("repo"));
        let tempdir_parent = handle
            .path()
            .parent()
            .expect("repo directory should have a parent");
        assert!(tempdir_parent.exists());
    }
}

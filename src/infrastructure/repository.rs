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
    use std::cell::RefCell;
    use std::fs;
    use tempfile::tempdir;
    use walkdir::WalkDir;

    struct FakeClone {
        fixture: PathBuf,
        calls: RefCell<usize>,
    }

    impl FakeClone {
        fn new(fixture: PathBuf) -> Self {
            Self {
                fixture,
                calls: RefCell::new(0),
            }
        }

        fn calls(&self) -> usize {
            *self.calls.borrow()
        }
    }

    impl CommandExecutor for FakeClone {
        fn run(&self, _program: &str, args: &[&str]) -> Result<()> {
            *self.calls.borrow_mut() += 1;
            let dest = PathBuf::from(args.last().unwrap());
            fs::create_dir_all(&dest)?;
            for entry in WalkDir::new(&self.fixture) {
                let entry = entry.unwrap();
                if entry.file_type().is_file() {
                    let relative = entry.path().strip_prefix(&self.fixture).unwrap();
                    let target = dest.join(relative);
                    if let Some(parent) = target.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::copy(entry.path(), target)?;
                }
            }
            Ok(())
        }
    }

    #[test]
    fn returns_existing_path() {
        let repo = tempdir().unwrap();
        let handle = resolve_repository(
            repo.path().to_str().unwrap(),
            &FakeClone::new(repo.path().to_path_buf()),
        )
        .unwrap();
        assert_eq!(handle.path(), &repo.path().canonicalize().unwrap());
    }

    #[test]
    fn clones_remote_repo() {
        let fixture = tempdir().unwrap();
        fs::write(fixture.path().join("manifest.yaml"), "version: 1").unwrap();
        let exec = FakeClone::new(fixture.path().to_path_buf());
        let handle = resolve_repository("https://example.com/repo.git", &exec).unwrap();
        assert!(handle.path().exists());
        assert_eq!(exec.calls(), 1);
    }
}

//! Service that stages rendered templates and links them into the target home.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::errors::{DotstrapError, Result};
use crate::services::templating::RenderedSet;

/// Link all rendered templates into the provided `home` directory.
pub fn link_templates(home: &Path, rendered: &RenderedSet, dry_run: bool) -> Result<Vec<PathBuf>> {
    let mut linked = Vec::new();
    let stage_root = home.join(".dotstrap/generated");
    if !dry_run {
        fs::create_dir_all(&stage_root)?;
    }
    for item in &rendered.templates {
        let destination = home.join(&item.template.destination);
        linked.push(destination.clone());
        if dry_run {
            continue;
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        if destination.exists() || destination.is_symlink() {
            reconcile_existing(&destination)?;
        }
        let stage_path = stage_root.join(&item.template.destination);
        if let Some(parent) = stage_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&item.rendered_path, &stage_path)?;
        apply_mode(&stage_path, item.template.mode)?;
        create_symlink(&stage_path, &destination)?;
    }
    Ok(linked)
}

fn reconcile_existing(path: &Path) -> Result<()> {
    if path.is_symlink() {
        fs::remove_file(path)?;
        return Ok(());
    }
    if !path.exists() {
        return Ok(());
    }
    let backup_dir = path
        .parent()
        .map(|p| p.join(".dotstrap-backups"))
        .unwrap_or_else(|| PathBuf::from(".dotstrap-backups"));
    fs::create_dir_all(&backup_dir)?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "config".into());
    let backup_path = backup_dir.join(format!("{file_name}.{timestamp}.bak"));
    fs::rename(path, backup_path)?;
    Ok(())
}

fn apply_mode(rendered: &Path, mode: Option<u32>) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Some(mode) = mode {
            let mut perms = fs::metadata(rendered)?.permissions();
            perms.set_mode(mode);
            fs::set_permissions(rendered, perms)?;
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (rendered, mode);
    }
    Ok(())
}

fn create_symlink(source: &Path, destination: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(source, destination).map_err(DotstrapError::Io)
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::symlink_file;
        symlink_file(source, destination).map_err(DotstrapError::Io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TemplateMapping;
    use crate::services::templating::{RenderedSet, RenderedTemplate};
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn build_rendered_set(destination: PathBuf, mode: Option<u32>, contents: &str) -> RenderedSet {
        let rendered_tempdir = TempDir::new().expect("failed to create rendered tempdir");
        let rendered_path = rendered_tempdir.path().join("rendered.txt");
        fs::write(&rendered_path, contents).expect("failed to seed rendered template");
        let mapping = TemplateMapping {
            source: PathBuf::from("source.txt"),
            destination,
            mode,
        };
        RenderedSet {
            _tempdir: rendered_tempdir,
            templates: vec![RenderedTemplate {
                template: mapping,
                rendered_path,
            }],
        }
    }

    #[test]
    fn link_templates_dry_run_returns_destinations_without_side_effects() {
        let home = TempDir::new().expect("failed to create home tempdir");
        let destination = PathBuf::from(".config/app.conf");
        let rendered_set = build_rendered_set(destination.clone(), None, "ignored");

        let linked =
            link_templates(home.path(), &rendered_set, true).expect("dry run should succeed");

        let expected_destination = home.path().join(&destination);
        assert_eq!(linked, vec![expected_destination.clone()]);
        assert!(
            !expected_destination.exists(),
            "dry run must not create destination files"
        );
        assert!(
            !home.path().join(".dotstrap").exists(),
            "dry run must not create staging directories"
        );
    }

    #[cfg(unix)]
    #[test]
    fn link_templates_creates_symlinks_and_backups_existing_files() {
        use std::os::unix::fs::PermissionsExt;

        let home = TempDir::new().expect("failed to create home tempdir");
        let destination = PathBuf::from(".config/app.conf");
        let rendered_set = build_rendered_set(destination.clone(), Some(0o700), "new contents");

        let destination_path = home.path().join(&destination);
        if let Some(parent) = destination_path.parent() {
            fs::create_dir_all(parent).expect("failed to create destination parent");
        }
        fs::write(&destination_path, "old contents").expect("failed to seed existing file");

        let linked =
            link_templates(home.path(), &rendered_set, false).expect("linking should succeed");

        let expected_destination = home.path().join(&destination);
        assert_eq!(linked, vec![expected_destination.clone()]);

        let metadata = fs::symlink_metadata(&expected_destination).expect("destination metadata");
        assert!(
            metadata.file_type().is_symlink(),
            "destination should be a symlink"
        );

        let stage_path = home.path().join(".dotstrap/generated").join(&destination);
        assert!(
            stage_path.exists(),
            "rendered content should be staged for linking"
        );
        let stage_contents = fs::read_to_string(&stage_path).expect("stage file must exist");
        assert_eq!(stage_contents, "new contents");

        let target = fs::read_link(&expected_destination).expect("failed to read symlink target");
        assert_eq!(target, stage_path);

        let mode = fs::metadata(&stage_path)
            .expect("stage metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o700, "stage file should apply requested mode");

        let backup_dir = destination_path
            .parent()
            .expect("destination should have a parent")
            .join(".dotstrap-backups");
        let backups: Vec<_> = fs::read_dir(&backup_dir)
            .expect("backup directory must exist")
            .collect();
        assert_eq!(backups.len(), 1, "exactly one backup should be created");
        let backup_path = backups[0].as_ref().expect("backup entry").path();
        let backup_contents =
            fs::read_to_string(&backup_path).expect("backup file should preserve contents");
        assert_eq!(backup_contents, "old contents");
    }
}

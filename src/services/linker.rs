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
    use tempfile::TempDir;

    fn rendered_set_for(
        tempdir: TempDir,
        entries: Vec<(PathBuf, PathBuf, Option<u32>)>,
    ) -> RenderedSet {
        let templates = entries
            .into_iter()
            .enumerate()
            .map(|(idx, (source, destination, mode))| RenderedTemplate {
                template: TemplateMapping {
                    source,
                    destination,
                    mode,
                },
                rendered_path: tempdir.path().join(format!("rendered_{idx}")),
            })
            .collect();
        RenderedSet {
            _tempdir: tempdir,
            templates,
        }
    }

    #[test]
    fn creates_symlinks() {
        let staging = tempfile::tempdir().unwrap();
        fs::write(staging.path().join("rendered_0"), "content").unwrap();
        let rendered = rendered_set_for(
            staging,
            vec![(
                PathBuf::from("templates/test"),
                PathBuf::from(".test"),
                None,
            )],
        );
        let home = tempfile::tempdir().unwrap();
        let linked = link_templates(home.path(), &rendered, false).unwrap();
        assert_eq!(linked.len(), 1);
        let target = home.path().join(".test");
        assert!(target.exists());
        let stage_path = home.path().join(".dotstrap/generated/.test");
        assert_eq!(fs::read_to_string(&stage_path).unwrap(), "content");
        #[cfg(unix)]
        {
            assert_eq!(std::fs::read_link(&target).unwrap(), stage_path);
        }
    }

    #[test]
    fn backs_up_existing_files() {
        let staging = tempfile::tempdir().unwrap();
        fs::write(staging.path().join("rendered_0"), "content").unwrap();
        let rendered = rendered_set_for(
            staging,
            vec![(
                PathBuf::from("templates/test"),
                PathBuf::from(".test"),
                None,
            )],
        );
        let home = tempfile::tempdir().unwrap();
        let dest = home.path().join(".test");
        fs::write(&dest, "old").unwrap();
        link_templates(home.path(), &rendered, false).unwrap();
        let backup_dir = home.path().join(".dotstrap-backups");
        assert!(backup_dir.exists());
        assert_eq!(fs::read_to_string(dest).unwrap(), "content");
        assert!(home.path().join(".dotstrap/generated/.test").exists());
    }

    #[test]
    fn dry_run_skips_changes() {
        let staging = tempfile::tempdir().unwrap();
        fs::write(staging.path().join("rendered_0"), "content").unwrap();
        let rendered = rendered_set_for(
            staging,
            vec![(
                PathBuf::from("templates/test"),
                PathBuf::from(".test"),
                None,
            )],
        );
        let home = tempfile::tempdir().unwrap();
        link_templates(home.path(), &rendered, true).unwrap();
        assert!(!home.path().join(".test").exists());
        assert!(!home.path().join(".dotstrap").exists());
    }

    #[cfg(unix)]
    #[test]
    fn replaces_existing_symlink_without_backup() {
        use std::os::unix::fs::symlink;
        let staging = tempfile::tempdir().unwrap();
        fs::write(staging.path().join("rendered_0"), "content").unwrap();
        let rendered = rendered_set_for(
            staging,
            vec![(
                PathBuf::from("templates/test"),
                PathBuf::from(".test"),
                None,
            )],
        );
        let home = tempfile::tempdir().unwrap();
        let dest = home.path().join(".test");
        let existing_target = home.path().join("old");
        fs::write(&existing_target, "old").unwrap();
        symlink(&existing_target, &dest).unwrap();
        link_templates(home.path(), &rendered, false).unwrap();
        assert!(!home.path().join(".dotstrap-backups").exists());
        assert!(home.path().join(".dotstrap/generated/.test").exists());
    }

    #[test]
    fn applies_mode() {
        let staging = tempfile::tempdir().unwrap();
        fs::write(staging.path().join("rendered_0"), "content").unwrap();
        let rendered = rendered_set_for(
            staging,
            vec![(
                PathBuf::from("templates/test"),
                PathBuf::from(".test"),
                Some(0o700),
            )],
        );
        let home = tempfile::tempdir().unwrap();
        link_templates(home.path(), &rendered, false).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let metadata = home
                .path()
                .join(".dotstrap/generated/.test")
                .metadata()
                .unwrap();
            assert_eq!(metadata.mode() & 0o777, 0o700);
        }
    }
}

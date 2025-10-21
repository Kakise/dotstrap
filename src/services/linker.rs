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

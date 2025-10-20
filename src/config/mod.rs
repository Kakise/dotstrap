//! Configuration loading helpers and strongly typed configuration models.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::errors::{DotstrapError, Result};

const MANIFEST_NAME: &str = "manifest.yaml";
const VALUES_NAME: &str = "values.yaml";
const BREW_PATH: &str = "brew/packages.yaml";

/// Manifest describing how templates should be rendered and linked.
#[derive(Debug, Deserialize, Clone)]
pub struct Manifest {
    pub version: u8,
    #[serde(default)]
    pub templates: Vec<TemplateMapping>,
}

/// Mapping between a template source file and its destination.
#[derive(Debug, Deserialize, Clone)]
pub struct TemplateMapping {
    pub source: PathBuf,
    pub destination: PathBuf,
    #[serde(default)]
    pub mode: Option<u32>,
}

/// Declarative definition of Homebrew taps, formulae, and casks.
#[derive(Debug, Deserialize, Default, Clone)]
pub struct BrewSpec {
    #[serde(default)]
    pub taps: Vec<String>,
    #[serde(default)]
    pub formulae: Vec<String>,
    #[serde(default)]
    pub casks: Vec<String>,
}

/// Load and validate the manifest from the repository root.
pub fn load_manifest(repo: &Path) -> Result<Manifest> {
    let path = repo.join(MANIFEST_NAME);
    let bytes = fs::read(&path)?;
    let manifest: Manifest =
        serde_yaml::from_slice(&bytes).map_err(|source| DotstrapError::Yaml {
            source,
            path: path.clone(),
        })?;
    if manifest.version != 1 {
        return Err(DotstrapError::UnsupportedManifestVersion {
            path: path.clone(),
            version: manifest.version,
        });
    }
    if manifest.templates.is_empty() {
        return Err(DotstrapError::ManifestMissingTemplates(path));
    }
    Ok(manifest)
}

/// Load shared values that seed the templating context.
pub fn load_values(repo: &Path) -> Result<HashMap<String, serde_json::Value>> {
    let path = repo.join(VALUES_NAME);
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let bytes = fs::read(&path)?;
    let json_value: serde_json::Value =
        serde_yaml::from_slice(&bytes).map_err(|source| DotstrapError::Yaml {
            source,
            path: path.clone(),
        })?;
    match json_value {
        serde_json::Value::Object(map) => Ok(map.into_iter().collect()),
        _ => Ok(HashMap::new()),
    }
}

/// Load the optional Homebrew specification from the repository root.
pub fn load_brew_spec(repo: &Path) -> Result<Option<BrewSpec>> {
    let path = repo.join(BREW_PATH);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path)?;
    let spec: BrewSpec = serde_yaml::from_slice(&bytes).map_err(|source| DotstrapError::Yaml {
        source,
        path: path.clone(),
    })?;
    Ok(Some(spec))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn reads_manifest() {
        let repo = tempdir().unwrap();
        fs::write(
            repo.path().join(MANIFEST_NAME),
            "version: 1\ntemplates:\n  - source: src\n    destination: dest\n",
        )
        .unwrap();
        let manifest = load_manifest(repo.path()).unwrap();
        assert_eq!(manifest.templates.len(), 1);
    }

    #[test]
    fn rejects_unknown_version() {
        let repo = tempdir().unwrap();
        fs::write(
            repo.path().join(MANIFEST_NAME),
            "version: 2\ntemplates:\n  - source: src\n    destination: dest\n",
        )
        .unwrap();
        let err = load_manifest(repo.path()).unwrap_err();
        matches!(err, DotstrapError::UnsupportedManifestVersion { .. });
    }

    #[test]
    fn fails_when_manifest_empty() {
        let repo = tempdir().unwrap();
        fs::write(
            repo.path().join(MANIFEST_NAME),
            "version: 1\ntemplates: []\n",
        )
        .unwrap();
        let err = load_manifest(repo.path()).unwrap_err();
        matches!(err, DotstrapError::ManifestMissingTemplates(_));
    }

    #[test]
    fn load_values_handles_missing_file() {
        let repo = tempdir().unwrap();
        let values = load_values(repo.path()).unwrap();
        assert!(values.is_empty());
    }

    #[test]
    fn load_values_reads_map() {
        let repo = tempdir().unwrap();
        fs::write(repo.path().join(VALUES_NAME), "name: Kakise\n").unwrap();
        let values = load_values(repo.path()).unwrap();
        assert_eq!(values.get("name").unwrap(), "Kakise");
    }

    #[test]
    fn load_brew_spec_optional() {
        let repo = tempdir().unwrap();
        assert!(load_brew_spec(repo.path()).unwrap().is_none());
    }

    #[test]
    fn load_brew_spec_reads_file() {
        let repo = tempdir().unwrap();
        fs::create_dir_all(repo.path().join("brew")).unwrap();
        fs::write(repo.path().join(BREW_PATH), "formulae:\n  - ripgrep\n").unwrap();
        let spec = load_brew_spec(repo.path()).unwrap().unwrap();
        assert_eq!(spec.formulae, vec!["ripgrep"]);
    }
}

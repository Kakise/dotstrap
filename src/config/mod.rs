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

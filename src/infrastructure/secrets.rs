//! Secret resolution helpers backed by environment variables or files.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::errors::{DotstrapError, Result};

const SECRETS_PATH: &str = "secrets/secrets.yaml";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase", tag = "from")]
enum SecretSource {
    Env {
        key: String,
        #[serde(default)]
        optional: bool,
    },
    File {
        path: PathBuf,
    },
}

/// Load secrets declared in `secrets/secrets.yaml` and surface them as JSON values.
pub fn load_secrets(repo: &Path, home: &Path) -> Result<HashMap<String, serde_json::Value>> {
    let path = repo.join(SECRETS_PATH);
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let bytes = fs::read(&path)?;
    let entries: HashMap<String, SecretSource> =
        serde_yaml::from_slice(&bytes).map_err(|source| DotstrapError::Yaml {
            source,
            path: path.clone(),
        })?;
    let mut secrets = HashMap::new();
    for (name, source) in entries {
        match source {
            SecretSource::Env { key, optional } => match std::env::var(&key) {
                Ok(value) => {
                    secrets.insert(name, serde_json::Value::String(value));
                }
                Err(_) if optional => {}
                Err(_) => {
                    return Err(DotstrapError::MissingSecret {
                        name,
                        provider: format!("environment variable {key}"),
                    });
                }
            },
            SecretSource::File { path: secret_path } => {
                let resolved = expand_path(&secret_path, home, repo);
                let contents = fs::read_to_string(&resolved)?;
                secrets.insert(name, serde_json::Value::String(contents.trim().to_string()));
            }
        }
    }
    Ok(secrets)
}

fn expand_path(path: &Path, home: &Path, repo: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    if let Some(stripped) = path_str.strip_prefix("~/") {
        return home.join(stripped);
    }
    if path.is_relative() {
        repo.join(path)
    } else {
        path.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use crate::infrastructure::secrets::{expand_path, load_secrets};
    use serial_test::serial;
    use std::collections::HashMap;
    use std::path::Path;

    #[test]
    fn test_load_secrets_empty() {
        let home = Path::new("/home/user");
        let repo = Path::new("/home/user/repo");
        let result = load_secrets(repo, home);
        assert_eq!(result.unwrap(), HashMap::new());
    }

    #[test]
    #[serial]
    fn test_load_secrets_tpl_not_found() {
        let home = Path::new("/home/user");
        let repo = Path::new("tests/dotstrap-config-example");
        unsafe {
            std::env::remove_var("DOTSTRAP_GITHUB_TOKEN");
        }
        let result = load_secrets(repo, home);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_load_secrets_tpl_found() {
        let home = Path::new("/home/user");
        let repo = Path::new("tests/dotstrap-config-example");
        unsafe {
            std::env::set_var("DOTSTRAP_GITHUB_TOKEN", "fake-token");
        }
        let result = load_secrets(repo, home);
        assert!(result.is_ok());
        let result_map = result.unwrap();
        assert_eq!(result_map.len(), 2);
        assert_eq!(
            result_map.get("github_token"),
            Some(&serde_json::Value::String("fake-token".to_string()))
        );
        assert_eq!(
            result_map.get("file_secret"),
            Some(&serde_json::Value::String("fake-file-secret".to_string()))
        )
    }

    #[test]
    fn test_load_secrets_invalid_yaml() {
        let home = Path::new("/home/user");
        let repo = Path::new("tests/erroneous-config");
        let result = load_secrets(repo, home);
        assert!(result.is_err());
        let result = result.unwrap_err();
        assert_eq!(
            result.to_string(),
            "failed to parse yaml file `tests/erroneous-config/secrets/secrets.yaml`: invalid type: string \"SYNTAX_ERROR\", expected a map"
        );
    }

    #[test]
    fn test_expand_path_with_relative_path() {
        let home = Path::new("/home/user");
        let repo = Path::new("/home/user/repo");
        let path = Path::new("etc/secret");
        let expanded = expand_path(path, home, repo);
        assert_eq!(expanded, Path::new("/home/user/repo/etc/secret"));
    }

    #[test]
    fn test_expand_path_with_absolute_path() {
        let home = Path::new("/home/user");
        let repo = Path::new("/home/user/repo");
        let path = Path::new("/etc/secret");
        let expanded = expand_path(path, home, repo);
        assert_eq!(expanded, Path::new("/etc/secret"));
    }

    #[test]
    fn test_expand_path_with_home_path() {
        let home = Path::new("/home/user");
        let repo = Path::new("/home/user/repo");
        let path = Path::new("~/.ssh/id_rsa");
        let expanded = expand_path(path, home, repo);
        assert_eq!(expanded, Path::new("/home/user/.ssh/id_rsa"));
    }
}

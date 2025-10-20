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
    use super::*;
    use std::fs;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn set_env(key: &str, value: &str) {
        unsafe { std::env::set_var(key, value) };
    }

    fn remove_env(key: &str) {
        unsafe { std::env::remove_var(key) };
    }

    #[test]
    fn expands_tilde_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tempfile::tempdir().unwrap();
        let path = Path::new("~/secret.txt");
        let resolved = expand_path(path, tmp.path(), repo.path());
        assert_eq!(resolved, tmp.path().join("secret.txt"));
    }

    #[test]
    fn expands_relative_paths() {
        let repo = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let path = Path::new("relative.txt");
        assert_eq!(
            expand_path(path, home.path(), repo.path()),
            repo.path().join("relative.txt")
        );
    }

    #[test]
    fn loads_env_secret() {
        let repo = tempfile::tempdir().unwrap();
        let path = repo.path().join(SECRETS_PATH);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "token:\n  from: env\n  key: DOTSTRAP_TOKEN\n").unwrap();
        let _guard = ENV_LOCK.lock().unwrap();
        set_env("DOTSTRAP_TOKEN", "value");
        let home_dir = home::home_dir().unwrap();
        let secrets = load_secrets(repo.path(), home_dir.as_path()).unwrap();
        assert_eq!(
            secrets.get("token"),
            Some(&serde_json::Value::String("value".into()))
        );
        remove_env("DOTSTRAP_TOKEN");
        drop(_guard);
    }

    #[test]
    fn loads_file_secret() {
        let repo = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let secrets_dir = repo.path().join("secrets");
        std::fs::create_dir_all(&secrets_dir).unwrap();
        let secret_file = secrets_dir.join("secret.txt");
        fs::write(&secret_file, "very-secret").unwrap();
        fs::write(
            repo.path().join(SECRETS_PATH),
            "token:\n  from: file\n  path: secrets/secret.txt\n",
        )
        .unwrap();
        let secrets = load_secrets(repo.path(), home.path()).unwrap();
        assert_eq!(
            secrets.get("token"),
            Some(&serde_json::Value::String("very-secret".into()))
        );
    }

    #[test]
    fn errors_when_secret_missing() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join("secrets")).unwrap();
        fs::write(
            repo.path().join(SECRETS_PATH),
            "token:\n  from: env\n  key: DOES_NOT_EXIST\n",
        )
        .unwrap();
        let err = load_secrets(repo.path(), repo.path()).unwrap_err();
        matches!(err, DotstrapError::MissingSecret { .. });
    }
}

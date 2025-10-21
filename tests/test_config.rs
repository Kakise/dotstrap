mod test_application;

#[cfg(test)]
mod tests {
    use dotstrap::DotstrapError;
    use dotstrap::config::{load_brew_spec, load_manifest, load_values};
    use std::path::Path;

    #[test]
    fn test_manifest_incorrect_version() {
        let path = Path::new("tests/erroneous-config/manifest-unsupported");
        let result = load_manifest(path);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DotstrapError::UnsupportedManifestVersion { .. }
        ));
    }

    #[test]
    fn test_manifest_missing_templates() {
        let path = Path::new("tests/erroneous-config/manifest-no-templates");
        let result = load_manifest(path);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DotstrapError::ManifestMissingTemplates { .. }
        ));
    }

    #[test]
    fn test_manifest_invalid() {
        let path = Path::new("tests/erroneous-config/manifest-invalid");
        let result = load_manifest(path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DotstrapError::Yaml { .. }));
    }

    #[test]
    fn test_values_invalid() {
        let path = Path::new("tests/erroneous-config/values-invalid");
        let result = load_values(path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DotstrapError::Yaml { .. }));
    }

    #[test]
    fn test_values_empty() {
        let path = Path::new("tests/erroneous-config/values-empty");
        let result = load_values(path);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_values_not_found() {
        let path = Path::new("tests/erroneous-config/values-not-found");
        let result = load_values(path);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_brew_spec_invalid() {
        let path = Path::new("tests/erroneous-config/brew-invalid");
        let result = load_brew_spec(path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DotstrapError::Yaml { .. }));
    }
}

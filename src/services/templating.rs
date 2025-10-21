//! Template rendering service built on top of Handlebars.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use handlebars::Handlebars;
use serde_json::Value;
use tempfile::TempDir;

use crate::config::{Manifest, TemplateMapping};
use crate::errors::{DotstrapError, Result};

/// Link between a manifest entry and its rendered file.
pub struct RenderedTemplate {
    pub template: TemplateMapping,
    pub rendered_path: PathBuf,
}

/// Collection of rendered templates backed by a temporary directory.
pub struct RenderedSet {
    pub(crate) _tempdir: TempDir,
    pub templates: Vec<RenderedTemplate>,
}

/// Merge declarative values and secrets into the template context.
pub fn build_context(values: &HashMap<String, Value>, secrets: &HashMap<String, Value>) -> Value {
    let mut root = serde_json::Map::new();
    for (key, value) in values {
        root.insert(key.clone(), value.clone());
    }
    let mut secrets_map = serde_json::Map::new();
    for (key, value) in secrets {
        secrets_map.insert(key.clone(), value.clone());
    }
    root.insert("secrets".into(), Value::Object(secrets_map));
    Value::Object(root)
}

/// Render all templates declared in the manifest into a temporary directory.
pub fn render_templates(repo: &Path, manifest: &Manifest, context: &Value) -> Result<RenderedSet> {
    let tempdir = TempDir::new()?;
    let mut rendered = Vec::new();
    let mut engine = Handlebars::new();

    for (idx, template) in manifest.templates.iter().enumerate() {
        let template_path = repo.join(&template.source);
        let contents = std::fs::read_to_string(&template_path)?;
        let template_name = format!("template_{idx}");
        engine
            .register_template_string(&template_name, contents)
            .map_err(|source| DotstrapError::TemplateCompile {
                source,
                path: template_path.clone(),
            })?;
        let rendered_contents =
            engine
                .render(&template_name, context)
                .map_err(|source| DotstrapError::Template {
                    source,
                    path: template_path.clone(),
                })?;
        let generated_path = tempdir.path().join(format!("rendered_{idx}"));
        std::fs::write(&generated_path, rendered_contents)?;
        rendered.push(RenderedTemplate {
            template: template.clone(),
            rendered_path: generated_path,
        });
    }

    Ok(RenderedSet {
        _tempdir: tempdir,
        templates: rendered,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn build_context_merges_values_and_secrets() {
        let mut values = HashMap::new();
        values.insert("user".to_string(), json!("dotstrap"));
        let mut secrets = HashMap::new();
        secrets.insert("token".to_string(), json!("secret"));

        let context = build_context(&values, &secrets);

        assert_eq!(context["user"], json!("dotstrap"));
        assert_eq!(context["secrets"]["token"], json!("secret"));
        assert_eq!(
            context
                .get("secrets")
                .and_then(|s| s.as_object())
                .map(|o| o.len()),
            Some(1),
            "secrets map should contain only the inserted secret"
        );
    }

    #[test]
    fn render_templates_generates_expected_files() {
        let repo_dir = TempDir::new().expect("failed to create repo tempdir");
        let template_path = repo_dir.path().join("greeting.hbs");
        fs::write(&template_path, "Hello {{name}}!").expect("failed to write template");

        let manifest = Manifest {
            version: 1,
            templates: vec![TemplateMapping {
                source: PathBuf::from("greeting.hbs"),
                destination: PathBuf::from(".config/greeting.txt"),
                mode: Some(0o640),
            }],
        };
        let context = json!({ "name": "Dotstrap" });

        let rendered_set = render_templates(repo_dir.path(), &manifest, &context)
            .expect("rendering should succeed");

        assert_eq!(rendered_set.templates.len(), 1, "one template expected");
        let rendered = &rendered_set.templates[0];
        assert_eq!(
            rendered.template.destination,
            PathBuf::from(".config/greeting.txt")
        );
        let contents =
            fs::read_to_string(&rendered.rendered_path).expect("rendered file must exist");
        assert_eq!(contents, "Hello Dotstrap!");
    }

    #[test]
    fn render_templates_propagates_compile_errors() {
        let repo_dir = TempDir::new().expect("failed to create repo tempdir");
        let template_path = repo_dir.path().join("broken.hbs");
        fs::write(&template_path, "{{#if user}}Hello{{/iff}}").expect("failed to write template");

        let manifest = Manifest {
            version: 1,
            templates: vec![TemplateMapping {
                source: PathBuf::from("broken.hbs"),
                destination: PathBuf::from("ignored.txt"),
                mode: None,
            }],
        };
        let context = json!({ "user": true });

        let error = match render_templates(repo_dir.path(), &manifest, &context) {
            Err(err) => err,
            Ok(_) => panic!("expected a compile error due to mismatched block"),
        };

        match error {
            DotstrapError::TemplateCompile { path, .. } => {
                assert_eq!(path, template_path);
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }
}

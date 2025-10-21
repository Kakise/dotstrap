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

use std::{fs, path::Path};

use anyhow::{Context, Result};
use chrono::Utc;
use comfy_table::{Attribute, Cell};
use serde::{Deserialize, Serialize};

use crate::{
  context::AppContext,
  templates::AnesisTemplate,
  utils::{
    picker::{ItemKind, PickItem},
    ui::{self, catalog_table},
  },
};

#[derive(Serialize, Deserialize)]
pub struct TemplatesCache {
  #[serde(rename = "lastUpdated")]
  pub last_updated: String,
  pub templates: Vec<CachedTemplate>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedTemplate {
  pub name: String,
  pub version: String,
  pub source: String,
  pub path: String,
  pub commit_sha: String,
}

impl CachedTemplate {
  pub fn to_pick_item(&self) -> PickItem {
    let meta = if self.version.is_empty() {
      String::new()
    } else {
      format!(" · v{}", self.version)
    };
    PickItem {
      kind: ItemKind::Template,
      id: self.name.clone(),
      name: self.name.clone(),
      meta,
      description: String::new(),
      haystack: format!("{} {}", self.name, self.source).to_lowercase(),
    }
  }
}

fn parse_index(path: &Path) -> Result<TemplatesCache> {
  let content = fs::read_to_string(path)?;
  serde_json::from_str(&content).with_context(|| {
    format!(
      "The template index '{}' is corrupt; delete it and re-run `anesis template install <name>` to rebuild it",
      path.display()
    )
  })
}

fn save_index(path: &Path, cache: &TemplatesCache) -> Result<()> {
  crate::utils::atomic::write_atomic(path, serde_json::to_string_pretty(cache)?.as_bytes())
}

pub fn read_installed_templates(template_path: &Path) -> Result<Vec<CachedTemplate>> {
  let templates_json = template_path.join("anesis-templates.json");
  if !templates_json.exists() {
    return Ok(Vec::new());
  }
  Ok(parse_index(&templates_json)?.templates)
}

pub fn update_templates_cache(
  template_path: &Path,
  path: &Path,
  commit_sha: &str,
) -> Result<CachedTemplate> {
  let anesis_json = template_path.join(path).join("anesis.template.json");
  let content = fs::read_to_string(&anesis_json)
    .with_context(|| format!("Failed to read {}", anesis_json.display()))?;
  let template_info: AnesisTemplate = serde_json::from_str(&content)?;

  crate::compat::check_anesis_version(&template_info.name, &template_info.anesis_version)?;

  let requested_name = path.to_string_lossy();
  anyhow::ensure!(
    template_info.name == requested_name,
    "template '{requested_name}' was installed, but its manifest declares a different name \
     ('{}'); refusing to cache it to avoid a directory/name mismatch",
    template_info.name
  );

  let templates_json = template_path.join("anesis-templates.json");
  crate::utils::atomic::with_file_lock(&templates_json, || {
    let mut templates_info: TemplatesCache = if templates_json.exists() {
      parse_index(&templates_json)?
    } else {
      TemplatesCache {
        last_updated: Utc::now().to_rfc3339(),
        templates: Vec::new(),
      }
    };

    templates_info.last_updated = Utc::now().to_rfc3339();

    templates_info
      .templates
      .retain(|t| t.name != template_info.name);
    let cached_template = CachedTemplate {
      name: template_info.name,
      version: template_info.version,
      source: template_info.repository.url,
      path: path.to_string_lossy().to_string(),
      commit_sha: commit_sha.to_string(),
    };
    templates_info.templates.push(cached_template.clone());

    save_index(&templates_json, &templates_info)?;

    Ok(cached_template)
  })
}

pub fn get_cached_template(ctx: &AppContext, name: &str) -> Result<Option<CachedTemplate>> {
  let templates_json = ctx.paths.templates.join("anesis-templates.json");

  if !templates_json.exists() {
    return Ok(None);
  }

  Ok(
    parse_index(&templates_json)?
      .templates
      .into_iter()
      .find(|t| t.name == name),
  )
}

pub fn remove_template_from_cache(template_path: &Path, template_name: &str) -> Result<()> {
  let templates_json = template_path.join("anesis-templates.json");

  if !templates_json.exists() {
    return Err(anyhow::anyhow!(
      "Template '{}' is not installed",
      template_name
    ));
  }

  crate::utils::atomic::with_file_lock(&templates_json, || {
    remove_locked(template_path, template_name, &templates_json)
  })?;
  ui::success(format!("Removed template '{template_name}'"));
  Ok(())
}

fn remove_locked(template_path: &Path, template_name: &str, templates_json: &Path) -> Result<()> {
  let mut templates_info = parse_index(templates_json)?;

  let exists = templates_info
    .templates
    .iter()
    .any(|t| t.name == template_name);
  if !exists {
    return Err(anyhow::anyhow!(
      "Template '{}' is not installed",
      template_name
    ));
  }

  templates_info.last_updated = Utc::now().to_rfc3339();

  if let Some(t) = templates_info
    .templates
    .iter()
    .find(|t| t.name == template_name)
  {
    let cleanup_path = template_path.join(&t.path);
    if cleanup_path.exists() {
      if let Err(e) = fs::remove_dir_all(&cleanup_path) {
        eprintln!("Failed to remove: {}", e);
      }
      let mut current = cleanup_path.parent();
      while let Some(parent) = current {
        if parent == template_path {
          break;
        }
        if fs::remove_dir(parent).is_err() {
          break;
        }
        current = parent.parent();
      }
    }
  }

  templates_info
    .templates
    .retain(|template| template.name != template_name);

  save_index(templates_json, &templates_info)
}

pub fn get_installed_templates(template_path: &Path) -> Result<()> {
  let templates_json = template_path.join("anesis-templates.json");

  let templates_info: TemplatesCache = if templates_json.exists() {
    parse_index(&templates_json)?
  } else {
    TemplatesCache {
      last_updated: Utc::now().to_rfc3339(),
      templates: Vec::new(),
    }
  };

  if templates_info.templates.is_empty() {
    println!("No templates installed yet.");
    return Ok(());
  }

  let mut table = catalog_table();

  table.set_header(vec![
    Cell::new("Name").add_attribute(Attribute::Bold),
    Cell::new("Version").add_attribute(Attribute::Bold),
    Cell::new("Source").add_attribute(Attribute::Bold),
  ]);

  for template in templates_info.templates {
    table.add_row(vec![
      Cell::new(&template.name),
      Cell::new(&template.version),
      Cell::new("registry"),
    ]);
  }

  println!(
    "\nInstalled templates (last updated: {}):",
    templates_info.last_updated
  );
  println!("{table}");

  Ok(())
}

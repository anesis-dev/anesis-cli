use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct StackManifest {
  pub schema_version: String,
  pub id: String,
  pub name: String,
  #[serde(default)]
  pub description: String,
  pub version: String,
  pub author: Author,
  pub template: String,
  #[serde(default)]
  pub addons: Vec<StackAddon>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Author {
  pub name: String,
  pub github: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StackAddon {
  pub id: String,
  #[serde(default = "default_command")]
  pub command: String,

  #[serde(default)]
  pub inputs: HashMap<String, String>,
}

fn default_command() -> String {
  "install".to_string()
}

pub fn load_stack(path: &Path) -> Result<StackManifest> {
  let file = if path.is_dir() {
    path.join("anesis.stack.json")
  } else {
    path.to_path_buf()
  };
  let raw = fs::read_to_string(&file)
    .with_context(|| format!("could not read stack manifest at {}", file.display()))?;
  let stack: StackManifest = serde_json::from_str(&raw)
    .with_context(|| format!("invalid stack manifest {}", file.display()))?;
  validate(&stack)?;
  Ok(stack)
}

pub fn validate(stack: &StackManifest) -> Result<()> {
  crate::compat::check_schema_version("stack", &stack.id, &stack.schema_version)?;
  crate::utils::validate::validate_registry_id("stack", &stack.id)?;

  if stack.template.trim().is_empty() {
    return Err(anyhow!("stack '{}' declares no template", stack.id));
  }
  crate::utils::validate::validate_template_name(&stack.template)?;
  for (i, addon) in stack.addons.iter().enumerate() {
    if addon.id.trim().is_empty() {
      return Err(anyhow!(
        "stack '{}': addon #{} has an empty id",
        stack.id,
        i + 1
      ));
    }
    crate::utils::validate::validate_registry_id("addon", &addon.id)?;
  }
  Ok(())
}

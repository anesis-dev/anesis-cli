use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub mod generator;
pub mod install;
pub mod loader;
pub mod publish;
pub mod update;

pub struct TemplateFile {
  pub path: PathBuf,
  pub contents: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct OxideTemplate {
  pub name: String,
  pub version: String,
  #[serde(rename = "oxideVersion")]
  pub oxide_version: String,
  pub official: bool,
  pub repository: OxideTemplateRepository,
  pub metadata: OxideTemplateMetadata,
}

#[derive(Serialize, Deserialize)]
pub struct OxideTemplateRepository {
  pub url: String,
}

#[derive(Serialize, Deserialize)]
pub struct OxideTemplateMetadata {
  #[serde(rename = "displayName")]
  pub display_name: String,
  pub description: String,
}

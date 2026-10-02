use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub mod cache;
pub mod catalog;
pub mod generator;
pub mod info;
pub mod install;
pub mod link;
pub mod loader;
pub mod publish;
pub mod republish;

pub struct TemplateFile {
  pub path: PathBuf,
  pub contents: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct AnesisTemplate {
  pub name: String,
  pub version: String,
  #[serde(rename = "anesisVersion")]
  pub anesis_version: String,
  #[serde(default)]
  pub author: AnesisTemplateAuthor,
  pub repository: AnesisTemplateRepository,
  #[serde(default)]
  pub specialization: String,
  #[serde(default)]
  pub scope: String,
  #[serde(default)]
  pub technologies: Vec<String>,
  #[serde(default)]
  pub languages: Vec<String>,
  #[serde(rename = "type", default)]
  pub template_type: String,
  pub metadata: AnesisTemplateMetadata,
  #[serde(default)]
  pub inputs: Vec<crate::addons::manifest::InputDef>,
  #[serde(default)]
  pub exclude: Vec<ExcludeBlock>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AnesisTemplateAuthor {
  #[serde(default)]
  pub name: String,
  #[serde(default)]
  pub github: String,
}

#[derive(Serialize, Deserialize)]
pub struct ExcludeBlock {
  pub when: String,
  pub paths: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct AnesisTemplateRepository {
  pub url: String,
}

#[derive(Serialize, Deserialize)]
pub struct AnesisTemplateMetadata {
  #[serde(rename = "displayName")]
  pub display_name: String,
  pub description: String,
  #[serde(default)]
  pub tags: Vec<String>,
}

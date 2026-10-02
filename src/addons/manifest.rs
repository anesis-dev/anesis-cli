use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Author {
  Structured { name: String, github: String },
  Name(String),
}

impl Author {
  pub fn github(&self) -> Option<&str> {
    match self {
      Author::Structured { github, .. } => Some(github),
      Author::Name(_) => None,
    }
  }
}

impl From<&str> for Author {
  fn from(name: &str) -> Self {
    Author::Name(name.to_string())
  }
}

impl From<String> for Author {
  fn from(name: String) -> Self {
    Author::Name(name)
  }
}

impl std::fmt::Display for Author {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Author::Structured { name, .. } => write!(f, "{name}"),
      Author::Name(name) => write!(f, "{name}"),
    }
  }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddonManifest {
  pub schema_version: String,
  pub id: String,
  pub name: String,
  pub version: String,
  pub description: String,
  pub author: Author,
  #[serde(default)]
  pub requires: Vec<String>,
  #[serde(default)]
  pub inputs: Vec<InputDef>,
  #[serde(default)]
  pub detect: Vec<DetectBlock>,
  #[serde(default)]
  pub variants: Vec<Variant>,
}

pub fn parse(content: &str) -> Result<AddonManifest> {
  let manifest: AddonManifest =
    serde_json::from_str(content).context("Invalid anesis.addon.json structure")?;
  validate(&manifest)?;
  Ok(manifest)
}

pub fn validate(manifest: &AddonManifest) -> Result<()> {
  crate::compat::check_schema_version("addon", &manifest.id, &manifest.schema_version)?;
  crate::utils::validate::validate_registry_id("addon", &manifest.id)?;
  Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InputDef {
  pub name: String,
  #[serde(rename = "type")]
  pub input_type: InputType,
  #[serde(default)]
  pub description: String,
  pub default: Option<String>,
  #[serde(default)]
  pub required: bool,
  #[serde(default)]
  pub options: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputType {
  Text,
  Boolean,
  Select,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DetectBlock {
  pub id: String,
  pub rules: Vec<DetectRule>,
  #[serde(rename = "match", default)]
  pub match_mode: MatchMode,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DetectRule {
  FileExists {
    file: String,
    #[serde(default)]
    negate: bool,
  },
  FileContains {
    file: String,
    contains: String,
    #[serde(default)]
    negate: bool,
  },
  JsonContains {
    file: String,
    key_path: String,
    value: Option<String>,
    #[serde(default)]
    negate: bool,
  },
  TomlContains {
    file: String,
    key_path: String,
    value: Option<String>,
    #[serde(default)]
    negate: bool,
  },
  YamlContains {
    file: String,
    key_path: String,
    value: Option<String>,
    #[serde(default)]
    negate: bool,
  },
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchMode {
  All,
  #[default]
  Any,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Variant {
  pub when: Option<String>,
  #[serde(default)]
  pub commands: Vec<AddonCommand>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddonCommand {
  pub name: String,
  #[serde(default)]
  pub description: String,
  #[serde(default)]
  pub once: bool,
  #[serde(default)]
  pub requires_commands: Vec<String>,
  #[serde(default)]
  pub inputs: Vec<InputDef>,
  #[serde(default)]
  pub steps: Vec<StepEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepEntry {
  #[serde(flatten)]
  pub kind: Step,
  #[serde(default)]
  pub when: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Step {
  Copy(CopyStep),
  Create(CreateStep),
  Inject(InjectStep),
  Replace(ReplaceStep),
  Append(AppendStep),
  Delete(DeleteStep),
  Rename(RenameStep),
  Move(MoveStep),
  Packages(PackagesStep),
  Run(RunStep),
  JsonPatch(JsonPatchStep),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyStep {
  pub src: String,
  pub dest: String,
  #[serde(default)]
  pub if_exists: IfExists,
  #[serde(default = "default_true")]
  pub render: bool,
}

fn default_true() -> bool {
  true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStep {
  pub path: String,
  pub content: String,
  #[serde(default)]
  pub if_exists: IfExists,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectStep {
  pub target: Target,
  pub content: String,
  pub after: Option<String>,
  pub before: Option<String>,
  #[serde(default)]
  pub if_not_found: IfNotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaceStep {
  pub target: Target,
  pub find: String,
  pub replace: String,
  #[serde(default)]
  pub if_not_found: IfNotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendStep {
  pub target: Target,
  pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteStep {
  pub target: Target,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameStep {
  pub from: String,
  pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveStep {
  pub from: String,
  pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackagesStep {
  #[serde(default)]
  pub dependencies: Vec<String>,
  #[serde(default)]
  pub dev_dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunStep {
  pub command: String,
  #[serde(default)]
  pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonPatchStep {
  pub path: String,
  #[serde(default)]
  pub set: std::collections::HashMap<String, serde_json::Value>,
  #[serde(default)]
  pub remove: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Target {
  File { file: String },
  Glob { glob: String },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IfExists {
  Ask,
  #[default]
  Overwrite,
  Skip,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IfNotFound {
  #[default]
  WarnAndAsk,
  Skip,
  Error,
}

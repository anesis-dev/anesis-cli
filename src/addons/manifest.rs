use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AddonManifest {
  pub schema_version: String,
  pub id: String,
  pub name: String,
  pub version: String,
  pub description: String,
  pub author: String,
  #[serde(default)]
  pub requires: Vec<String>,
  #[serde(default)]
  pub inputs: Vec<InputDef>,
  #[serde(default)]
  pub detect: Vec<DetectBlock>,
  #[serde(default)]
  pub variants: Vec<Variant>,
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
  pub steps: Vec<Step>,
}

#[derive(Debug, Serialize, Deserialize)]
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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CopyStep {
  pub src: String,
  pub dest: String,
  #[serde(default)]
  pub if_exists: IfExists,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateStep {
  pub path: String,
  pub content: String,
  #[serde(default)]
  pub if_exists: IfExists,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InjectStep {
  pub target: Target,
  pub content: String,
  pub after: Option<String>,
  pub before: Option<String>,
  #[serde(default)]
  pub if_not_found: IfNotFound,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReplaceStep {
  pub target: Target,
  pub find: String,
  pub replace: String,
  #[serde(default)]
  pub if_not_found: IfNotFound,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppendStep {
  pub target: Target,
  pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteStep {
  pub target: Target,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RenameStep {
  pub from: String,
  pub to: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MoveStep {
  pub from: String,
  pub to: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
  File { file: String },
  Glob { glob: String },
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IfExists {
  Ask,
  #[default]
  Overwrite,
  Skip,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IfNotFound {
  #[default]
  WarnAndAsk,
  Skip,
  Error,
}

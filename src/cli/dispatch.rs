use std::collections::HashMap;

use super::commands::{AddonCommands, Commands, StackCommands, TemplateCommands};

pub fn is_json_mode(command: &Commands) -> bool {
  matches!(
    command,
    Commands::Account { json: true }
      | Commands::Info { json: true }
      | Commands::Status { json: true }
      | Commands::Search { json: true, .. }
      | Commands::Outdated { json: true }
      | Commands::Doctor { json: true }
      | Commands::Why { json: true, .. }
      | Commands::Template {
        command: TemplateCommands::List { json: true } | TemplateCommands::Info { json: true, .. }
      }
      | Commands::Addon {
        command: AddonCommands::List { json: true } | AddonCommands::Info { json: true, .. }
      }
      | Commands::Stack {
        command: StackCommands::List { json: true } | StackCommands::Info { json: true, .. }
      }
  )
}

pub fn skip_version_notice(command: &Commands, quiet: bool) -> bool {
  is_json_mode(command)
    || quiet
    || matches!(
      command,
      Commands::Upgrade | Commands::Completions { .. } | Commands::Man { .. }
    )
}

pub fn parse_inputs(pairs: &[String]) -> anyhow::Result<HashMap<String, String>> {
  let mut map = HashMap::new();
  for pair in pairs {
    let (name, value) = pair
      .split_once('=')
      .ok_or_else(|| anyhow::anyhow!("Invalid --input '{pair}'; expected NAME=VALUE"))?;
    map.insert(name.to_string(), value.to_string());
  }
  Ok(map)
}

use crate::utils::template_engine::TemplateContext;
use std::path::Path;

use serde_json::Value;

use crate::addons::manifest::JsonPatchStep;

use super::{Rollback, StepFailure, StepResult};

pub fn execute_json_patch(
  step: &JsonPatchStep,
  project_root: &Path,
  ctx: &TemplateContext,
) -> StepResult {
  let rendered_path = super::render_string(&step.path, ctx)?;
  let path = super::safe_join(project_root, &rendered_path, "json_patch path")?;

  let original = std::fs::read(&path).map_err(StepFailure::without_rollbacks)?;
  let mut value: Value = serde_json::from_slice(&original).map_err(|e| {
    StepFailure::without_rollbacks(anyhow::anyhow!("'{rendered_path}' is not valid JSON: {e}"))
  })?;

  for (key_path, new_value) in &step.set {
    set_at(&mut value, key_path, new_value.clone());
  }
  for key_path in &step.remove {
    remove_at(&mut value, key_path);
  }

  let mut rendered =
    serde_json::to_string_pretty(&value).map_err(StepFailure::without_rollbacks)?;
  rendered.push('\n');

  let rollbacks = vec![Rollback::restore_file(path.clone(), original)];
  if let Err(e) = std::fs::write(&path, rendered) {
    return Err(StepFailure::new(e, rollbacks));
  }
  Ok(rollbacks)
}

fn set_at(root: &mut Value, key_path: &str, new_value: Value) {
  let keys: Vec<&str> = key_path.split('.').collect();
  let Some((last, parents)) = keys.split_last() else {
    return;
  };

  let mut current = root;
  for key in parents {
    if !current.is_object() {
      *current = Value::Object(Default::default());
    }
    current = current
      .as_object_mut()
      .unwrap()
      .entry(key.to_string())
      .or_insert_with(|| Value::Object(Default::default()));
  }

  if !current.is_object() {
    *current = Value::Object(Default::default());
  }
  let map = current.as_object_mut().unwrap();
  match (map.get(*last), &new_value) {
    (Some(Value::Object(existing)), Value::Object(incoming)) => {
      let mut merged = existing.clone();
      for (k, v) in incoming {
        merged.insert(k.clone(), v.clone());
      }
      map.insert((*last).to_string(), Value::Object(merged));
    }
    _ => {
      map.insert((*last).to_string(), new_value);
    }
  }
}

fn remove_at(root: &mut Value, key_path: &str) {
  let keys: Vec<&str> = key_path.split('.').collect();
  let Some((last, parents)) = keys.split_last() else {
    return;
  };

  let mut current = root;
  for key in parents {
    match current.get_mut(*key) {
      Some(next) => current = next,
      None => return,
    }
  }

  if let Some(map) = current.as_object_mut() {
    map.remove(*last);
  }
}

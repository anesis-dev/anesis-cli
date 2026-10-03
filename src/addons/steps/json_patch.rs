use crate::utils::template_engine::TemplateContext;
use std::path::Path;

use anyhow::{Result, anyhow};
use serde::Serialize;
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
    StepFailure::without_rollbacks(anyhow!(
      "'{rendered_path}' is not valid JSON ({e}); json_patch needs strict JSON, so comments and trailing commas are not supported"
    ))
  })?;

  let mut sets: Vec<_> = step.set.iter().collect();
  sets.sort_by(|a, b| a.0.cmp(b.0));
  for (key_path, new_value) in sets {
    set_at(&mut value, key_path, new_value.clone())
      .map_err(|e| StepFailure::without_rollbacks(e.context(format!("in '{rendered_path}'"))))?;
  }
  for key_path in &step.remove {
    remove_at(&mut value, key_path);
  }

  let mut rendered = render_like(&original, &value).map_err(StepFailure::without_rollbacks)?;
  if original.ends_with(b"\n") || original.is_empty() {
    rendered.push('\n');
  }

  let rollbacks = vec![Rollback::restore_file(path.clone(), original)];
  if let Err(e) = crate::utils::atomic::write_file_atomic(&path, rendered.as_bytes()) {
    return Err(StepFailure::new(e, rollbacks));
  }
  Ok(rollbacks)
}

fn detect_indent(original: &[u8]) -> Vec<u8> {
  std::str::from_utf8(original)
    .ok()
    .and_then(|text| {
      text.lines().skip(1).find_map(|line| {
        let indent: String = line
          .chars()
          .take_while(|c| *c == ' ' || *c == '\t')
          .collect();
        (!indent.is_empty() && line.len() > indent.len()).then(|| indent.into_bytes())
      })
    })
    .unwrap_or_else(|| b"  ".to_vec())
}

fn render_like(original: &[u8], value: &Value) -> Result<String> {
  let indent = detect_indent(original);
  let formatter = serde_json::ser::PrettyFormatter::with_indent(&indent);
  let mut out = Vec::new();
  let mut serializer = serde_json::Serializer::with_formatter(&mut out, formatter);
  value.serialize(&mut serializer)?;
  Ok(String::from_utf8(out)?)
}

fn segments(key_path: &str) -> Vec<String> {
  match key_path.strip_prefix('/') {
    Some(pointer) => pointer
      .split('/')
      .map(|s| s.replace("~1", "/").replace("~0", "~"))
      .collect(),
    None => key_path.split('.').map(str::to_string).collect(),
  }
}

fn set_at(root: &mut Value, key_path: &str, new_value: Value) -> Result<()> {
  let keys = segments(key_path);
  let Some((last, parents)) = keys.split_last() else {
    return Ok(());
  };

  let mut current = root;
  for key in parents {
    current = match current {
      Value::Object(map) => map
        .entry(key.clone())
        .or_insert_with(|| Value::Object(Default::default())),
      Value::Array(items) => {
        let idx: usize = key
          .parse()
          .map_err(|_| anyhow!("'{key}' in '{key_path}' is not an array index"))?;
        items
          .get_mut(idx)
          .ok_or_else(|| anyhow!("index {idx} in '{key_path}' is out of range"))?
      }
      _ => {
        return Err(anyhow!(
          "cannot set '{key_path}': '{key}' sits under a value that is neither an object nor an array"
        ));
      }
    };
  }

  match current {
    Value::Object(map) => match (map.get(last), &new_value) {
      (Some(Value::Object(existing)), Value::Object(incoming)) => {
        let mut merged = existing.clone();
        for (k, v) in incoming {
          merged.insert(k.clone(), v.clone());
        }
        map.insert(last.clone(), Value::Object(merged));
      }
      _ => {
        map.insert(last.clone(), new_value);
      }
    },
    Value::Array(items) => {
      let idx: usize = last
        .parse()
        .map_err(|_| anyhow!("'{last}' in '{key_path}' is not an array index"))?;
      let slot = items
        .get_mut(idx)
        .ok_or_else(|| anyhow!("index {idx} in '{key_path}' is out of range"))?;
      *slot = new_value;
    }
    _ => {
      return Err(anyhow!(
        "cannot set '{key_path}': its parent is neither an object nor an array"
      ));
    }
  }
  Ok(())
}

fn remove_at(root: &mut Value, key_path: &str) {
  let keys = segments(key_path);
  let Some((last, parents)) = keys.split_last() else {
    return;
  };

  let mut current = root;
  for key in parents {
    match current.get_mut(key.as_str()) {
      Some(next) => current = next,
      None => return,
    }
  }

  if let Some(map) = current.as_object_mut() {
    map.shift_remove(last.as_str());
  }
}

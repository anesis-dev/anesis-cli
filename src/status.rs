use std::path::Path;

use anyhow::{Context, Result};

use crate::{
  addons::{lock::LockFile, summary::ChangeSummary},
  manifest::AnesisManifest,
  utils::ui::{
    self,
    tree::{TreeNode, render},
  },
};

fn load(project_root: &Path) -> Result<(AnesisManifest, LockFile)> {
  let manifest_path = project_root.join("anesis.json");
  let contents = std::fs::read_to_string(&manifest_path).with_context(|| {
    "not an Anesis project (no anesis.json here). Run `anesis new` to create one, or cd into a project root."
  })?;
  let manifest: AnesisManifest = serde_json::from_str(&contents)?;
  let lock = LockFile::load(project_root)?;
  Ok((manifest, lock))
}

pub fn print_status(project_root: &Path) -> Result<()> {
  let (manifest, lock) = load(project_root)?;

  let mut root = TreeNode::new(format!(
    "{}  {}",
    ui::accent(&manifest.template_name),
    ui::muted(&manifest.template_sha)
  ));

  if lock.addons.is_empty() {
    println!("{}", root.label);
    println!("{}", ui::muted("(no addons applied)"));
    return Ok(());
  }

  for entry in &lock.addons {
    let version = if entry.version.is_empty() {
      String::new()
    } else {
      format!("v{}", entry.version)
    };
    let files: usize = entry
      .commands
      .iter()
      .map(|c| ChangeSummary::from_rollbacks(&c.journal).files_changed())
      .sum();
    let ran = entry.commands_executed();

    let mut label = format!(
      "{}  {}  [{}]",
      ui::accent(&entry.id),
      version,
      entry.variant
    );
    if !ran.is_empty() {
      label.push_str(&format!("  ran: {}", ran.join(", ")));
    }
    label.push_str(&format!("  {files} files"));

    root = root.child(TreeNode::new(label));
  }

  println!("{}", render(&root));
  Ok(())
}

pub fn status_json(project_root: &Path) -> Result<serde_json::Value> {
  let (manifest, lock) = load(project_root)?;
  Ok(serde_json::json!({
    "template": {
      "name": manifest.template_name,
      "sha": manifest.template_sha,
    },
    "addons": lock.addons.iter().map(|e| serde_json::json!({
      "id": e.id,
      "version": e.version,
      "variant": e.variant,
      "commands_executed": e.commands_executed(),
    })).collect::<Vec<_>>(),
  }))
}

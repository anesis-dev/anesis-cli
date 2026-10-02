use anyhow::Result;

use super::{
  cache::{cached_path, read_installed_stacks},
  manifest::load_stack,
  registry::fetch_stack_manifest,
};
use crate::{context::AppContext, utils::ui};

pub fn print_installed_stacks(ctx: &AppContext, json: bool) -> Result<()> {
  let stacks = read_installed_stacks(ctx)?;
  if json {
    println!("{}", serde_json::to_string_pretty(&stacks)?);
    return Ok(());
  }
  if stacks.is_empty() {
    println!("No stacks installed yet. Install one with `anesis stack install <id>`.");
    return Ok(());
  }
  for s in &stacks {
    println!(
      "{}  {} {}",
      ui::accent_bold(&s.id),
      s.name,
      ui::muted(format!("({} + {} addons)", s.template, s.addons.len()))
    );
  }
  Ok(())
}

pub async fn stack_info(ctx: &AppContext, stack_id: &str, json: bool) -> Result<()> {
  let manifest = match fetch_stack_manifest(ctx, stack_id).await {
    Ok(m) => m,
    Err(e) => {
      let cached = cached_path(ctx, stack_id);
      if cached.exists() {
        load_stack(&cached)?
      } else {
        return Err(e);
      }
    }
  };
  if json {
    println!("{}", serde_json::to_string_pretty(&manifest)?);
    return Ok(());
  }
  println!(
    "{} {}",
    ui::bold(&manifest.name),
    ui::muted(format!("({})", manifest.id))
  );
  if !manifest.description.is_empty() {
    println!("{}", manifest.description);
  }
  println!();
  ui::kv("template", &manifest.template);
  println!("{}", ui::muted("addons:"));
  for a in &manifest.addons {
    println!("  {} {}", ui::accent(&a.id), ui::muted(&a.command));
  }
  println!();
  ui::hint(
    "Scaffold it",
    format!("anesis new <dir> --stack {}", manifest.id),
  );
  Ok(())
}

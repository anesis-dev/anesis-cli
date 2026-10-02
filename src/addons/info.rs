use anyhow::Result;

use crate::{
  context::AppContext,
  utils::ui::{self, spinner},
};

use super::{
  install::{install_addon, read_cached_manifest},
  manifest::{AddonManifest, InputDef, InputType},
  runner::step_label,
};

pub async fn addon_info(ctx: &AppContext, addon_id: &str, json: bool) -> Result<()> {
  let addon_dir = ctx.paths.addons.join(addon_id);
  let cached = super::cache::get_cached_addon(&ctx.paths.addons, addon_id)?;
  let manifest = if cached.is_some() && addon_dir.exists() {
    read_cached_manifest(&ctx.paths.addons, addon_id)?
  } else {
    let sp = spinner(format!("Fetching addon '{addon_id}'..."));
    let result = install_addon(ctx, addon_id)
      .await
      .inspect_err(|_| sp.finish_and_clear())?;
    sp.finish_and_clear();
    result.into_manifest()
  };

  if json {
    println!("{}", serde_json::to_string_pretty(&manifest)?);
    return Ok(());
  }

  print_addon_info(&manifest);
  Ok(())
}

fn print_addon_info(m: &AddonManifest) {
  println!("{} {}", ui::bold(&m.name), ui::muted(format!("({})", m.id)));
  println!(
    "{} {} {}",
    ui::accent(format!("v{}", m.version)),
    ui::muted("by"),
    m.author
  );
  if !m.description.is_empty() {
    println!("{}", m.description);
  }
  if !m.requires.is_empty() {
    ui::kv("requires", m.requires.join(", "));
  }

  for variant in &m.variants {
    let when = variant.when.as_deref().unwrap_or("universal");
    println!("\n{} {}", ui::magenta_bold("variant"), ui::accent(when));
    for cmd in &variant.commands {
      let once = if cmd.once {
        ui::muted(" (once)")
      } else {
        String::new()
      };
      print!(
        "  {} {}{}",
        ui::magenta(ui::symbols::bullet()),
        ui::bold(&cmd.name),
        once
      );
      if cmd.description.is_empty() {
        println!();
      } else {
        println!(" {} {}", ui::muted("—"), cmd.description);
      }
      if !cmd.inputs.is_empty() {
        println!("    {}", ui::muted("inputs:"));
        for input in &cmd.inputs {
          println!("      {}", input_line(input));
        }
      }
      if !cmd.steps.is_empty() {
        let total = cmd.steps.len();
        println!("    {}", ui::muted("steps:"));
        for (idx, step) in cmd.steps.iter().enumerate() {
          let when_suffix = step
            .when
            .as_deref()
            .map(|w| format!(" {}", ui::muted(format!("(when: {w})"))))
            .unwrap_or_default();
          print!("      ");
          ui::step(
            idx,
            total,
            format!("{}{}", step_label(&step.kind), when_suffix),
          );
        }
      }
    }
  }
}

#[doc(hidden)]
pub fn input_line_for_tests(input: &InputDef) -> String {
  input_line(input)
}

fn input_line(input: &InputDef) -> String {
  let ty = match input.input_type {
    InputType::Text => "text",
    InputType::Boolean => "boolean",
    InputType::Select => "select",
  };
  let mut extra = Vec::new();
  if input.required {
    extra.push("required".to_string());
  }
  if let Some(default) = &input.default {
    extra.push(format!("default: {default}"));
  }
  if !input.options.is_empty() {
    extra.push(format!("options: {}", input.options.join("/")));
  }
  let suffix = if extra.is_empty() {
    String::new()
  } else {
    format!(" ({})", extra.join(", "))
  };
  format!("{}: {}{}", ui::accent(&input.name), ty, ui::muted(suffix))
}

use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result};

use super::detect::detect_variant;
use super::manifest::{self, AddonCommand, AddonManifest, IfNotFound, Step, Target};

pub fn lint_addon(dir: &Path) -> Result<Vec<String>> {
  let manifest_path = dir.join("anesis.addon.json");
  let content = std::fs::read_to_string(&manifest_path).with_context(|| {
    format!(
      "No 'anesis.addon.json' found at {}",
      manifest_path.display()
    )
  })?;
  let manifest = manifest::parse(&content)?;

  let mut errors = Vec::new();
  check_id_matches_directory(&manifest, dir, &mut errors);
  check_copy_sources(&manifest, dir, &mut errors);
  check_requires_commands(&manifest, &mut errors);
  check_test_fixture_anchors(&manifest, dir, &mut errors);
  Ok(errors)
}

fn commands(manifest: &AddonManifest) -> impl Iterator<Item = &AddonCommand> {
  manifest.variants.iter().flat_map(|v| v.commands.iter())
}

fn check_id_matches_directory(manifest: &AddonManifest, dir: &Path, errors: &mut Vec<String>) {
  let Some(directory) = dir.file_name() else {
    return;
  };
  if directory != manifest.id.as_str() {
    errors.push(format!(
      "addon id '{}' does not match its directory '{}'",
      manifest.id,
      directory.display()
    ));
  }
}

fn check_copy_sources(manifest: &AddonManifest, dir: &Path, errors: &mut Vec<String>) {
  for command in commands(manifest) {
    for entry in &command.steps {
      let Step::Copy(step) = &entry.kind else {
        continue;
      };
      if step.src.contains("{{") {
        continue;
      }
      if !dir.join(&step.src).exists() {
        errors.push(format!(
          "command '{}' copies '{}', which does not exist",
          command.name, step.src
        ));
      }
    }
  }
}

fn check_requires_commands(manifest: &AddonManifest, errors: &mut Vec<String>) {
  let known: HashSet<&str> = commands(manifest).map(|c| c.name.as_str()).collect();
  for command in commands(manifest) {
    for req in &command.requires_commands {
      if !known.contains(req.as_str()) {
        errors.push(format!(
          "command '{}' requires_commands names unknown command '{req}'",
          command.name
        ));
      }
    }
  }
}

fn check_test_fixture_anchors(manifest: &AddonManifest, dir: &Path, errors: &mut Vec<String>) {
  let fixture = dir.join("test-fixture");
  if !fixture.is_dir() {
    return;
  }

  let detected = detect_variant(&manifest.detect, &fixture);
  let variant = manifest
    .variants
    .iter()
    .find(|v| v.when.as_deref() == detected.as_deref())
    .or_else(|| manifest.variants.iter().find(|v| v.when.is_none()));
  let Some(variant) = variant else {
    return;
  };

  for command in &variant.commands {
    if !command.requires_commands.is_empty() {
      continue;
    }
    for entry in &command.steps {
      let Step::Inject(step) = &entry.kind else {
        continue;
      };
      if !matches!(step.if_not_found, IfNotFound::Error) {
        continue;
      }

      let targets = resolve_fixture_target(&step.target, &fixture, &command.name, errors);
      for (label, contents) in targets {
        for (key, anchor) in [("after", &step.after), ("before", &step.before)] {
          let Some(anchor) = anchor else { continue };
          if anchor.contains("{{") {
            continue;
          }
          if !contents.contains(anchor.trim()) {
            errors.push(format!(
              "command '{}': test-fixture '{label}' has no anchor {key} = {anchor:?} \
               (this command would fail against it)",
              command.name
            ));
          }
        }
      }
    }
  }
}

fn resolve_fixture_target(
  target: &Target,
  fixture: &Path,
  command_name: &str,
  errors: &mut Vec<String>,
) -> Vec<(String, String)> {
  match target {
    Target::File { file } => {
      if file.contains("{{") {
        return Vec::new();
      }
      match std::fs::read_to_string(fixture.join(file)) {
        Ok(contents) => vec![(file.clone(), contents)],
        Err(_) => {
          errors.push(format!(
            "command '{command_name}': test-fixture is missing '{file}', which it injects into"
          ));
          Vec::new()
        }
      }
    }
    Target::Glob { glob } => {
      if glob.contains("{{") {
        return Vec::new();
      }
      let joined = fixture.join(glob);
      let matched: Vec<(String, String)> = glob::glob(&joined.to_string_lossy())
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .filter_map(|path| {
          let contents = std::fs::read_to_string(&path).ok()?;
          let label = path
            .strip_prefix(fixture)
            .unwrap_or(&path)
            .display()
            .to_string();
          Some((label, contents))
        })
        .collect();

      if matched.is_empty() {
        errors.push(format!(
          "command '{command_name}': test-fixture has no file matching '{glob}', which it injects into"
        ));
      }
      matched
    }
  }
}

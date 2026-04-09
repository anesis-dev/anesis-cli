use std::{path::Path, sync::LazyLock};

use anyhow::{Result, anyhow};
use regex::Regex;
use url::Url;

static VALID_NAME_CHARS: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9_\-\.]+$").unwrap());

static VALID_TEMPLATE_NAME: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap());

pub fn validate_project_name(name: &str) -> Result<()> {
  if name == "." {
    return Ok(());
  }

  if name.is_empty() {
    return Err(anyhow!("Project name cannot be empty"));
  }

  if name.len() > 255 {
    return Err(anyhow!("Project name is too long (max 255 characters)"));
  }

  if Path::new(name).exists() {
    return Err(anyhow!("Directory '{}' already exists!", name));
  }

  let valid_chars = &*VALID_NAME_CHARS;
  if !valid_chars.is_match(name) {
    return Err(anyhow!(
      "Project name can only contain letters, numbers, hyphens, underscores, and dots"
    ));
  }

  if name.starts_with('.') {
    return Err(anyhow!("Project name cannot start with a dot"));
  }

  if name.ends_with('.') || name.ends_with(' ') {
    return Err(anyhow!("Project name cannot end with a dot or space"));
  }

  let reserved_windows = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
  ];

  let uppercase_name = name.to_uppercase();
  if reserved_windows.contains(&uppercase_name.as_str()) {
    return Err(anyhow!("'{}' is a reserved name in Windows", name));
  }

  Ok(())
}

pub fn is_valid_github_repo_url(input: &str) -> Result<()> {
  let Ok(url) = Url::parse(input) else {
    return Err(anyhow!("Invalid URL format"));
  };

  if url.host_str() != Some("github.com") {
    return Err(anyhow!("URL is not a GitHub domain"));
  }

  let segments: Vec<_> = match url.path_segments() {
    Some(s) => s.collect(),
    None => {
      return Err(anyhow!("Failed to extract path segments from URL"));
    }
  };

  if segments.len() < 2 {
    return Err(anyhow!("URL does not point to a GitHub repository"));
  }

  Ok(())
}

pub fn validate_template_name(template_name: &str) -> Result<()> {
  if !VALID_TEMPLATE_NAME.is_match(template_name) {
    anyhow::bail!(
      "Invalid template name '{}'. Allowed characters: a-z, A-Z, 0-9, '-' and '_'",
      template_name
    );
  }

  Ok(())
}

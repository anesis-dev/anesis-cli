use std::{path::Path, sync::LazyLock};

use anyhow::{Result, anyhow};
use regex::Regex;
use url::Url;

static VALID_NAME_CHARS: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9_\-\.]+$").unwrap());

static VALID_TEMPLATE_NAME: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap());

pub fn ensure_destination_available(name: &str, overwrite: bool) -> Result<()> {
  if name == "." {
    return Ok(());
  }
  let path = Path::new(name);
  if !path.exists() {
    return Ok(());
  }
  if overwrite && path.is_dir() {
    return Ok(());
  }
  if path.is_dir() {
    return Err(anyhow!(
      "Directory '{}' already exists! Pass --overwrite to generate into it.",
      name
    ));
  }
  Err(anyhow!("'{}' already exists and is not a directory", name))
}

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

  let valid_chars = &*VALID_NAME_CHARS;
  if !valid_chars.is_match(name) {
    return Err(anyhow!(
      "Project name can only contain letters, numbers, hyphens, underscores, and dots"
    ));
  }

  if name.starts_with('.') {
    return Err(anyhow!("Project name cannot start with a dot"));
  }

  if name.ends_with('.') {
    return Err(anyhow!("Project name cannot end with a dot"));
  }

  if is_windows_reserved_name(name) {
    return Err(anyhow!("'{}' is a reserved name in Windows", name));
  }

  Ok(())
}

pub fn parse_bool(value: &str) -> Option<bool> {
  match value.trim().to_ascii_lowercase().as_str() {
    "true" | "yes" | "y" | "1" | "on" => Some(true),
    "false" | "no" | "n" | "0" | "off" => Some(false),
    _ => None,
  }
}

pub fn is_windows_reserved_name(name: &str) -> bool {
  const RESERVED: [&str; 8] = [
    "CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$", "CLOCK$", "CONFIG$",
  ];
  let stem = name
    .split('.')
    .next()
    .unwrap_or(name)
    .trim_end()
    .to_uppercase();
  if RESERVED.contains(&stem.as_str()) {
    return true;
  }
  let digit = |c: char| matches!(c, '0'..='9' | '¹' | '²' | '³');
  ["COM", "LPT"].iter().any(|prefix| {
    stem
      .strip_prefix(prefix)
      .is_some_and(|rest| rest.chars().count() == 1 && rest.chars().all(digit))
  })
}

pub fn is_valid_github_repo_url(input: &str) -> Result<()> {
  let Ok(url) = Url::parse(input) else {
    return Err(anyhow!("Invalid URL format"));
  };

  if url.scheme() != "https" {
    return Err(anyhow!("URL must use https"));
  }

  if url.host_str() != Some("github.com") {
    return Err(anyhow!("URL is not a GitHub domain"));
  }

  if !url.username().is_empty()
    || url.password().is_some()
    || url.query().is_some()
    || url.fragment().is_some()
  {
    return Err(anyhow!(
      "URL must not contain credentials, a query string, or a fragment"
    ));
  }

  let segments: Vec<_> = match url.path_segments() {
    Some(s) => s.collect(),
    None => {
      return Err(anyhow!("Failed to extract path segments from URL"));
    }
  };

  let repo = segments.get(1).copied().unwrap_or("");
  let repo_name = repo.strip_suffix(".git").unwrap_or(repo);
  if segments.len() != 2 || segments[0].is_empty() || repo_name.is_empty() {
    return Err(anyhow!(
      "URL does not point to a GitHub repository (expected https://github.com/<owner>/<repo>)"
    ));
  }

  Ok(())
}

pub fn require_https_url(input: &str, label: &str) -> Result<()> {
  let url = Url::parse(input).map_err(|_| anyhow!("{label} is not a valid URL: '{input}'"))?;
  if url.scheme() != "https" {
    return Err(anyhow!(
      "{label} must use https, got scheme '{}': '{input}'",
      url.scheme()
    ));
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

pub fn validate_registry_id(kind: &str, id: &str) -> Result<()> {
  if id.trim().is_empty() {
    return Err(anyhow!("{kind} id cannot be empty"));
  }

  if Path::new(id).is_absolute() {
    return Err(anyhow!(
      "{kind} id '{id}' must be a relative name, not an absolute path"
    ));
  }

  for segment in id.split('/') {
    if segment.is_empty() {
      return Err(anyhow!(
        "{kind} id '{id}' must not contain empty path segments (leading/trailing/double '/')"
      ));
    }
    if segment == "." || segment == ".." {
      return Err(anyhow!(
        "{kind} id '{id}' must not contain '.' or '..' path segments"
      ));
    }
    if !VALID_NAME_CHARS.is_match(segment) {
      return Err(anyhow!(
        "{kind} id '{id}' segment '{segment}' may only contain letters, numbers, hyphens, underscores, and dots"
      ));
    }
  }

  Ok(())
}

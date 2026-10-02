use std::path::Path;

use serde::Serialize;

use crate::{
  addons::cache::read_cache as read_addon_cache,
  auth::token::get_auth_user,
  context::AppContext,
  manifest::AnesisManifest,
  templates::cache::read_installed_templates,
  upgrade::{check_latest_cli_version, is_newer_version},
  utils::{errors::AnesisError, ui},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
  Ok,
  Warn,
  Fail,
}

#[derive(Debug, Clone, Serialize)]
pub struct Check {
  pub name: String,
  pub status: CheckStatus,
  pub detail: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub hint: Option<String>,
}

impl Check {
  fn ok(name: &str, detail: impl Into<String>) -> Self {
    Self {
      name: name.to_string(),
      status: CheckStatus::Ok,
      detail: detail.into(),
      hint: None,
    }
  }

  fn warn(name: &str, detail: impl Into<String>, hint: impl Into<String>) -> Self {
    Self {
      name: name.to_string(),
      status: CheckStatus::Warn,
      detail: detail.into(),
      hint: Some(hint.into()),
    }
  }

  fn fail(name: &str, detail: impl Into<String>, hint: impl Into<String>) -> Self {
    Self {
      name: name.to_string(),
      status: CheckStatus::Fail,
      detail: detail.into(),
      hint: Some(hint.into()),
    }
  }
}

pub async fn run_checks(ctx: &AppContext, project_root: &Path) -> Vec<Check> {
  vec![
    check_cli_version(ctx).await,
    check_backend(ctx).await,
    check_auth(ctx),
    check_home_writable(ctx),
    check_addon_cache(ctx),
    check_template_cache(ctx),
    check_project_consistency(project_root),
    check_tools_in_path(),
  ]
}

async fn check_cli_version(ctx: &AppContext) -> Check {
  let current = env!("CARGO_PKG_VERSION");
  match check_latest_cli_version(&ctx.client).await {
    Ok(latest) => match is_newer_version(current, &latest) {
      Ok(true) => Check::warn(
        "CLI version",
        format!("v{current} installed, v{latest} available"),
        "Run `anesis upgrade` to update.",
      ),
      _ => Check::ok("CLI version", format!("v{current} (up to date)")),
    },
    Err(err) => Check::warn(
      "CLI version",
      format!("could not check for updates ({err:#})"),
      "Check your internet connection.",
    ),
  }
}

async fn check_backend(ctx: &AppContext) -> Check {
  match ctx.client.get(&ctx.backend_url).send().await {
    Ok(resp) if resp.status().is_success() || resp.status().is_client_error() => {
      Check::ok("Backend", format!("reachable at {}", ctx.backend_url))
    }
    Ok(resp) => Check::warn(
      "Backend",
      format!("{} returned {}", ctx.backend_url, resp.status()),
      "The registry may be temporarily unavailable.",
    ),
    Err(err) => Check::fail(
      "Backend",
      format!("could not reach {} ({err})", ctx.backend_url),
      "Check your internet connection and try again.",
    ),
  }
}

fn check_auth(ctx: &AppContext) -> Check {
  match get_auth_user(&ctx.paths.auth) {
    Ok(user) => Check::ok("Authentication", format!("logged in as {}", user.name)),
    Err(err) => {
      let logged_out_reason = err
        .downcast_ref::<AnesisError>()
        .map(|e| matches!(e, AnesisError::SessionExpired));
      match logged_out_reason {
        Some(true) => Check::warn(
          "Authentication",
          "session expired",
          "Run `anesis login` to re-authenticate.",
        ),
        _ => Check::warn(
          "Authentication",
          "not logged in",
          "Run `anesis login` if you plan to publish templates/addons/stacks.",
        ),
      }
    }
  }
}

fn check_home_writable(ctx: &AppContext) -> Check {
  let probe = ctx.paths.home.join(".doctor-write-check");
  match std::fs::write(&probe, b"ok") {
    Ok(()) => {
      let _ = std::fs::remove_file(&probe);
      Check::ok("~/.anesis writable", ctx.paths.home.display().to_string())
    }
    Err(err) => Check::fail(
      "~/.anesis writable",
      format!("{} is not writable ({err})", ctx.paths.home.display()),
      "Fix permissions on your anesis home directory (ANESIS_HOME).",
    ),
  }
}

fn check_addon_cache(ctx: &AppContext) -> Check {
  let cache = match read_addon_cache(&ctx.paths.addons) {
    Ok(cache) => cache,
    Err(err) => {
      return Check::fail(
        "Addon cache",
        format!("could not read addon cache ({err:#})"),
        "Remove and reinstall the affected addon(s).",
      );
    }
  };

  let missing: Vec<&str> = cache
    .addons
    .iter()
    .filter(|a| !ctx.paths.addons.join(&a.path).exists())
    .map(|a| a.id.as_str())
    .collect();

  if missing.is_empty() {
    Check::ok(
      "Addon cache",
      format!("{} addon(s) cached", cache.addons.len()),
    )
  } else {
    Check::warn(
      "Addon cache",
      format!(
        "{} addon(s) indexed but missing on disk: {}",
        missing.len(),
        missing.join(", ")
      ),
      "Run `anesis addon install <id>` to re-fetch them.",
    )
  }
}

fn check_template_cache(ctx: &AppContext) -> Check {
  let templates = match read_installed_templates(&ctx.paths.templates) {
    Ok(t) => t,
    Err(err) => {
      return Check::fail(
        "Template cache",
        format!("could not read template cache ({err:#})"),
        "Remove and reinstall the affected template(s).",
      );
    }
  };

  let missing: Vec<&str> = templates
    .iter()
    .filter(|t| !ctx.paths.templates.join(&t.path).exists())
    .map(|t| t.name.as_str())
    .collect();

  if missing.is_empty() {
    Check::ok(
      "Template cache",
      format!("{} template(s) cached", templates.len()),
    )
  } else {
    Check::warn(
      "Template cache",
      format!(
        "{} template(s) indexed but missing on disk: {}",
        missing.len(),
        missing.join(", ")
      ),
      "Run `anesis template install <name>` to re-fetch them.",
    )
  }
}

fn check_project_consistency(project_root: &Path) -> Check {
  let manifest_path = project_root.join("anesis.json");
  if !manifest_path.exists() {
    return Check::ok("Project consistency", "not inside an Anesis project");
  }

  let manifest: AnesisManifest = match std::fs::read_to_string(&manifest_path)
    .ok()
    .and_then(|s| serde_json::from_str(&s).ok())
  {
    Some(m) => m,
    None => {
      return Check::fail(
        "Project consistency",
        "anesis.json exists but could not be parsed",
        "Check anesis.json for syntax errors.",
      );
    }
  };

  let lock = match crate::addons::lock::LockFile::load(project_root) {
    Ok(l) => l,
    Err(err) => {
      return Check::fail(
        "Project consistency",
        format!("could not read anesis.lock ({err:#})"),
        "Check anesis.lock for corruption.",
      );
    }
  };

  let lock_ids: std::collections::HashSet<&str> =
    lock.addons.iter().map(|a| a.id.as_str()).collect();
  let manifest_ids: std::collections::HashSet<&str> =
    manifest.addons.iter().map(String::as_str).collect();

  let missing_from_lock: Vec<&str> = manifest_ids.difference(&lock_ids).copied().collect();
  let missing_from_manifest: Vec<&str> = lock_ids.difference(&manifest_ids).copied().collect();

  if missing_from_lock.is_empty() && missing_from_manifest.is_empty() {
    Check::ok(
      "Project consistency",
      format!(
        "anesis.json and anesis.lock agree on {} addon(s)",
        lock_ids.len()
      ),
    )
  } else {
    let mut detail = Vec::new();
    if !missing_from_lock.is_empty() {
      detail.push(format!(
        "in anesis.json but not anesis.lock: {}",
        missing_from_lock.join(", ")
      ));
    }
    if !missing_from_manifest.is_empty() {
      detail.push(format!(
        "in anesis.lock but not anesis.json: {}",
        missing_from_manifest.join(", ")
      ));
    }
    Check::warn(
      "Project consistency",
      detail.join("; "),
      "anesis.json and anesis.lock have drifted apart; this usually means one was edited by hand.",
    )
  }
}

fn check_tools_in_path() -> Check {
  let tools = ["git", "node"];
  let missing: Vec<&str> = tools
    .iter()
    .filter(|t| which::which(t).is_err())
    .copied()
    .collect();

  let package_managers = ["npm", "pnpm", "yarn", "bun"];
  let has_pm = package_managers.iter().any(|pm| which::which(pm).is_ok());

  if missing.is_empty() && has_pm {
    Check::ok(
      "Tools in PATH",
      "git, node and a package manager are available",
    )
  } else {
    let mut detail = missing.to_vec();
    if !has_pm {
      detail.push("a package manager (npm/pnpm/yarn/bun)");
    }
    Check::warn(
      "Tools in PATH",
      format!("missing: {}", detail.join(", ")),
      "Addons that run install/build steps need these on PATH.",
    )
  }
}

pub fn print_checks(checks: &[Check]) {
  for check in checks {
    let symbol = match check.status {
      CheckStatus::Ok => ui::good(ui::symbols::ok()),
      CheckStatus::Warn => ui::yellow(ui::symbols::warn()),
      CheckStatus::Fail => ui::red(ui::symbols::err()),
    };
    println!("{} {}  {}", symbol, ui::bold(&check.name), check.detail);
    if let Some(hint) = &check.hint {
      println!("    {} {}", ui::muted("hint:"), hint);
    }
  }
}

#[doc(hidden)]
pub fn check_home_writable_for_tests(ctx: &AppContext) -> Check {
  check_home_writable(ctx)
}

#[doc(hidden)]
pub fn check_project_consistency_for_tests(project_root: &Path) -> Check {
  check_project_consistency(project_root)
}

#[doc(hidden)]
pub fn check_addon_cache_for_tests(ctx: &AppContext) -> Check {
  check_addon_cache(ctx)
}

#[doc(hidden)]
pub fn check_template_cache_for_tests(ctx: &AppContext) -> Check {
  check_template_cache(ctx)
}

pub fn doctor_json(checks: &[Check]) -> serde_json::Value {
  serde_json::json!({ "checks": checks })
}

pub fn has_failure(checks: &[Check]) -> bool {
  checks.iter().any(|c| c.status == CheckStatus::Fail)
}

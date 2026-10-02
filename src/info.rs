use std::{fs, path::Path};

use crate::{auth::token::get_auth_user, context::AppContext, utils::ui};

pub fn print_info(ctx: &AppContext) {
  println!(
    "{} {}",
    ui::accent_bold("anesis"),
    ui::muted(format!("v{}", env!("CARGO_PKG_VERSION")))
  );
  println!();

  match get_auth_user(&ctx.paths.auth) {
    Ok(user) => println!(
      "{}  {} {}",
      ui::bold("Account:"),
      ui::good(ui::symbols::ok()),
      ui::good(format!("logged in as {}", user.name))
    ),
    Err(_) => println!(
      "{}  {} {}",
      ui::bold("Account:"),
      ui::yellow(ui::symbols::err()),
      ui::yellow("not logged in (run: anesis login)")
    ),
  }
  println!("{}  {}", ui::bold("Backend:"), ui::muted(&ctx.backend_url));
  println!();

  println!("{}", ui::bold("Paths:"));
  print_path("home", &ctx.paths.home);
  print_path("templates cache", &ctx.paths.templates);
  print_path("addons cache", &ctx.paths.addons);
  print_path("auth", &ctx.paths.auth);
  println!();

  let templates = count_array(
    &ctx.paths.templates.join("anesis-templates.json"),
    "templates",
  );
  let addons = count_array(&ctx.paths.addons_index, "addons");
  println!(
    "{} {} template(s), {} addon(s)",
    ui::bold("Installed:"),
    ui::accent(templates.to_string()),
    ui::accent(addons.to_string())
  );
}

pub fn info_json(ctx: &AppContext) -> serde_json::Value {
  let user = get_auth_user(&ctx.paths.auth).ok();
  serde_json::json!({
    "version": env!("CARGO_PKG_VERSION"),
    "backend": ctx.backend_url,
    "logged_in": user.is_some(),
    "account": user.map(|u| u.name),
    "paths": {
      "home": ctx.paths.home,
      "templates_cache": ctx.paths.templates,
      "addons_cache": ctx.paths.addons,
      "auth": ctx.paths.auth,
    },
    "installed": {
      "templates": count_array(&ctx.paths.templates.join("anesis-templates.json"), "templates"),
      "addons": count_array(&ctx.paths.addons_index, "addons"),
    },
  })
}

fn print_path(label: &str, path: &Path) {
  ui::kv_padded(label, path.display().to_string(), 16);
}

fn count_array(path: &Path, field: &str) -> usize {
  fs::read_to_string(path)
    .ok()
    .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
    .and_then(|value| value.get(field)?.as_array().map(Vec::len))
    .unwrap_or(0)
}

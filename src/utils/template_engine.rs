use std::collections::BTreeMap;

use minijinja::{AutoEscape, Environment, UndefinedBehavior, Value};
use serde::Serialize;

const ONE_OFF_TEMPLATE_NAME: &str = "__anesis_one_off__";

#[derive(Default)]
pub struct TemplateContext(BTreeMap<String, Value>);

impl TemplateContext {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn insert<K: Into<String>, V: Serialize + ?Sized>(&mut self, key: K, value: &V) {
    self.0.insert(key.into(), Value::from_serialize(value));
  }
}

pub fn hardened_env() -> Environment<'static> {
  let mut env = Environment::new();
  env.set_undefined_behavior(UndefinedBehavior::SemiStrict);
  env.set_keep_trailing_newline(true);
  env.set_auto_escape_callback(|_| AutoEscape::None);
  env
}

pub fn render_string(s: &str, ctx: &TemplateContext) -> anyhow::Result<String> {
  let mut env = hardened_env();
  env.add_template_owned(ONE_OFF_TEMPLATE_NAME, s.to_owned())?;
  Ok(env.get_template(ONE_OFF_TEMPLATE_NAME)?.render(&ctx.0)?)
}

fn shell_quote(value: &str) -> anyhow::Result<String> {
  if value.chars().any(|c| c.is_control() && c != '\t') {
    anyhow::bail!("input value contains control characters and cannot be used in a shell command");
  }
  if !value.is_empty()
    && value
      .chars()
      .all(|c| c.is_ascii_alphanumeric() || "_@%+=:,./-".contains(c))
  {
    return Ok(value.to_string());
  }
  if cfg!(windows) {
    if value.contains(['%', '!']) {
      anyhow::bail!("input value contains '%' or '!' and cannot be used safely in a cmd command");
    }
    Ok(format!("\"{}\"", value.replace('"', "\"\"")))
  } else {
    Ok(format!("'{}'", value.replace('\'', "'\\''")))
  }
}

pub fn render_shell_string(s: &str, ctx: &TemplateContext) -> anyhow::Result<String> {
  let mut env = hardened_env();
  env.set_formatter(|out, state, value| {
    if value.is_undefined() {
      return minijinja::escape_formatter(out, state, value);
    }
    let text = value
      .as_str()
      .map(str::to_string)
      .unwrap_or_else(|| value.to_string());
    let quoted = shell_quote(&text)
      .map_err(|e| minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, e.to_string()))?;
    std::fmt::Write::write_str(out, &quoted)
      .map_err(|_| minijinja::Error::from(minijinja::ErrorKind::WriteFailure))
  });
  env.add_template_owned(ONE_OFF_TEMPLATE_NAME, s.to_owned())?;
  Ok(env.get_template(ONE_OFF_TEMPLATE_NAME)?.render(&ctx.0)?)
}

pub fn render_named(
  env: &mut Environment<'static>,
  name: &str,
  source: &str,
  ctx: &TemplateContext,
) -> anyhow::Result<String> {
  env.add_template_owned(name.to_owned(), source.to_owned())?;
  Ok(env.get_template(name)?.render(&ctx.0)?)
}

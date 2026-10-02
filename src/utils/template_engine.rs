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

pub fn render_named(
  env: &mut Environment<'static>,
  name: &str,
  source: &str,
  ctx: &TemplateContext,
) -> anyhow::Result<String> {
  env.add_template_owned(name.to_owned(), source.to_owned())?;
  Ok(env.get_template(name)?.render(&ctx.0)?)
}

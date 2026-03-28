use std::path::Path;

use anyhow::Result;

use crate::{
  AppContext,
  cache::is_template_installed,
  templates::{TemplateFile, install::install_template},
  utils::fs::read_dir_to_files,
};

pub async fn get_files(ctx: &AppContext, template_name: &str) -> Result<Vec<TemplateFile>> {
  let path = Path::new(template_name);
  if !is_template_installed(ctx, template_name)? {
    install_template(ctx, template_name).await?;
  }

  let files = read_dir_to_files(&ctx.paths.templates.join(path))?;

  Ok(files)
}

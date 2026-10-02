use std::{
  fs,
  path::{Path, PathBuf},
};

use anyhow::Result;

pub struct AnesisPaths {
  pub home: PathBuf,
  pub version_check: PathBuf,
  pub cache: PathBuf,
  pub templates: PathBuf,
  pub auth: PathBuf,
  pub addons: PathBuf,
  pub addons_index: PathBuf,
  pub stacks: PathBuf,
}

impl AnesisPaths {
  pub fn new() -> Result<Self> {
    let home_dir = match std::env::var_os("ANESIS_HOME") {
      Some(dir) => PathBuf::from(dir),
      None => dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?,
    };

    Ok(Self::under(&home_dir))
  }

  pub fn under(home_dir: &Path) -> Self {
    let anesis_home = home_dir.join(".anesis");

    Self {
      home: anesis_home.clone(),
      version_check: anesis_home.join("version_check.json"),
      cache: anesis_home.join("cache"),
      templates: anesis_home.join("cache").join("templates"),
      auth: anesis_home.join("auth.json"),
      addons: anesis_home.join("cache").join("addons"),
      addons_index: anesis_home
        .join("cache")
        .join("addons")
        .join("anesis-addons.json"),
      stacks: anesis_home.join("cache").join("stacks"),
    }
  }

  pub fn addon_dir(&self, addon_id: &str) -> Result<PathBuf> {
    crate::utils::validate::validate_registry_id("addon", addon_id)?;
    Ok(self.addons.join(addon_id))
  }

  pub fn ensure_directories(&self) -> Result<()> {
    fs::create_dir_all(&self.home)?;
    fs::create_dir_all(&self.cache)?;
    fs::create_dir_all(&self.templates)?;
    fs::create_dir_all(&self.addons)?;
    fs::create_dir_all(&self.stacks)?;
    Ok(())
  }
}

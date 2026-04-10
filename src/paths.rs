use std::{fs, path::PathBuf};

use anyhow::Result;

pub struct OxidePaths {
  pub home: PathBuf,
  pub config: PathBuf,
  pub cache: PathBuf,
  pub templates: PathBuf,
  pub auth: PathBuf,
  pub addons: PathBuf,
  pub addons_index: PathBuf,
}

impl OxidePaths {
  pub fn new() -> Result<Self> {
    let home_dir =
      dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

    let oxide_home = home_dir.join(".oxide");

    Ok(Self {
      home: oxide_home.clone(),
      config: oxide_home.join("config.json"),
      cache: oxide_home.join("cache"),
      templates: oxide_home.join("cache").join("templates"),
      auth: oxide_home.join("auth.json"),
      addons: oxide_home.join("cache").join("addons"),
      addons_index: oxide_home.join("cache").join("addons").join("oxide-addons.json"),
    })
  }

  pub fn ensure_directories(&self) -> Result<()> {
    fs::create_dir_all(&self.home)?;
    fs::create_dir_all(&self.cache)?;
    fs::create_dir_all(&self.templates)?;
    fs::create_dir_all(&self.addons)?;
    Ok(())
  }
}

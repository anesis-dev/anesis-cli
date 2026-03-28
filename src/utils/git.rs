use std::{
  fs,
  path::{Path, PathBuf},
};

use anyhow::Result;
use reqwest::{Client, header::USER_AGENT};
use serde::Deserialize;

use crate::BACKEND_URL;

#[derive(Deserialize)]
struct GithubEntry {
  name: String,

  #[serde(rename = "type")]
  entry_type: String,

  download_url: Option<String>,
  url: String,
}

pub async fn download_dir(client: &Client, api_url: &str, path: &Path) -> Result<()> {
  fs::create_dir_all(path)?;

  let entries: Vec<GithubEntry> = client
    .get(format!("{}/github/proxy", BACKEND_URL))
    .query(&[("url", api_url)])
    .header(USER_AGENT, "oxide")
    .send()
    .await?
    .error_for_status()?
    .json()
    .await?;

  for entry in entries {
    let local_path: PathBuf = path.join(&entry.name);

    match entry.entry_type.as_str() {
      "file" => {
        if let Some(download_url) = entry.download_url {
          let bytes = client
            .get(download_url)
            .header(USER_AGENT, "oxide")
            .send()
            .await?
            .bytes()
            .await?;

          fs::write(&local_path, bytes)?;
          println!("✓ {}", local_path.display());
        }
      }
      "dir" => {
        Box::pin(download_dir(client, &entry.url, &local_path)).await?;
      }
      _ => {}
    }
  }

  Ok(())
}

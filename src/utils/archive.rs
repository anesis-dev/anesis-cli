use std::{io::Cursor, path::Path};

use anyhow::Result;
use flate2::read::GzDecoder;
use reqwest::Client;
use tar::Archive;

/// Download a GitHub tarball directly and extract it to `dest`.
///
/// `subdir` — optional path within the repo to extract (e.g. `"templates/react"`).
/// Pass `None` to extract the full repo root.
pub async fn download_and_extract(
  client: &Client,
  archive_url: &str,
  dest: &Path,
  subdir: Option<&str>,
) -> Result<()> {
  let bytes = client
    .get(archive_url)
    .header("User-Agent", "oxide")
    .send()
    .await?
    .error_for_status()?
    .bytes()
    .await?;

  std::fs::create_dir_all(dest)?;

  let gz = GzDecoder::new(Cursor::new(bytes));
  let mut archive = Archive::new(gz);

  for entry in archive.entries()? {
    let mut entry = entry?;
    let raw_path = entry.path()?.into_owned();

    // GitHub tarballs always have a single root dir: {owner}-{repo}-{short_sha}/
    // Strip it so all paths are relative to the repo root.
    let mut components = raw_path.components();
    components.next(); // discard the archive root component
    let stripped = components.as_path();

    // If the template lives in a subdirectory, skip everything outside it
    // and strip that prefix so files land directly in `dest`.
    let rel = if let Some(dir) = subdir {
      match stripped.strip_prefix(dir) {
        Ok(r) => r.to_owned(),
        Err(_) => continue,
      }
    } else {
      stripped.to_owned()
    };

    if rel.as_os_str().is_empty() {
      continue; // the directory entry itself — nothing to write
    }

    let out_path = dest.join(&rel);
    if let Some(parent) = out_path.parent() {
      std::fs::create_dir_all(parent)?;
    }
    entry.unpack(&out_path)?;
  }

  Ok(())
}

/// Strips the archive root component and optional subdir prefix from a raw
/// entry path, mirroring the extraction logic in `download_and_extract`.
/// Returns `None` if the entry should be skipped (outside subdir, or empty).
#[doc(hidden)]
pub fn strip_archive_path_for_tests(
  raw_path: &std::path::Path,
  subdir: Option<&str>,
) -> Option<std::path::PathBuf> {
  let mut components = raw_path.components();
  components.next(); // discard archive root (e.g. owner-repo-sha/)
  let stripped = components.as_path();

  let rel: std::path::PathBuf = if let Some(dir) = subdir {
    match stripped.strip_prefix(dir) {
      Ok(r) => r.to_owned(),
      Err(_) => return None,
    }
  } else {
    stripped.to_owned()
  };

  if rel.as_os_str().is_empty() {
    return None;
  }
  Some(rel)
}

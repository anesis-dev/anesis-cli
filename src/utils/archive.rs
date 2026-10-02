use std::{
  io::{Cursor, Read, Write},
  path::{Component, Path, PathBuf},
};

use anyhow::{Result, anyhow};
use flate2::read::GzDecoder;
use futures::StreamExt;
use indicatif::ProgressBar;
use reqwest::Client;
use tar::Archive;

use crate::utils::{ui, validate::require_https_url};

fn is_safe_relative(rel: &Path) -> bool {
  rel
    .components()
    .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

const MAX_RESPONSE_BYTES: u64 = 100 * 1024 * 1024;
const MAX_ENTRIES: usize = 20_000;
const MAX_TOTAL_UNCOMPRESSED_BYTES: u64 = 512 * 1024 * 1024;

#[doc(hidden)]
pub const MAX_ENTRIES_FOR_TESTS: usize = MAX_ENTRIES;

pub(crate) async fn download_capped(
  client: &Client,
  url: &str,
  token: Option<&str>,
  progress: Option<&ProgressBar>,
) -> Result<Vec<u8>> {
  let mut request = client.get(url).header("User-Agent", "anesis");
  if let Some(token) = token {
    request = request.bearer_auth(token);
  }

  let response = request.send().await?.error_for_status()?;

  if let Some(len) = response.content_length() {
    if len > MAX_RESPONSE_BYTES {
      return Err(anyhow!(
        "archive download exceeds the {MAX_RESPONSE_BYTES} byte limit (Content-Length: {len})"
      ));
    }
    if let Some(pb) = progress {
      pb.set_length(len);
      ui::progress::apply_download_style(pb);
    }
  }

  let mut buf = Vec::new();
  let mut stream = response.bytes_stream();
  while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    if buf.len() as u64 + chunk.len() as u64 > MAX_RESPONSE_BYTES {
      return Err(anyhow!(
        "archive download exceeds the {MAX_RESPONSE_BYTES} byte limit"
      ));
    }
    buf.extend_from_slice(&chunk);
    if let Some(pb) = progress {
      pb.inc(chunk.len() as u64);
    }
  }

  Ok(buf)
}

#[doc(hidden)]
pub async fn download_capped_for_tests(
  client: &Client,
  url: &str,
  token: Option<&str>,
) -> Result<Vec<u8>> {
  download_capped(client, url, token, None).await
}

#[doc(hidden)]
pub async fn download_capped_with_progress_for_tests(
  client: &Client,
  url: &str,
  token: Option<&str>,
  progress: &ProgressBar,
) -> Result<Vec<u8>> {
  download_capped(client, url, token, Some(progress)).await
}

fn copy_with_cap(
  mut reader: impl Read,
  writer: &mut impl Write,
  remaining_budget: &mut u64,
) -> Result<()> {
  let mut buf = [0u8; 64 * 1024];
  loop {
    let n = reader.read(&mut buf)?;
    if n == 0 {
      break;
    }
    let n = n as u64;
    if n > *remaining_budget {
      return Err(anyhow!(
        "archive exceeds the {MAX_TOTAL_UNCOMPRESSED_BYTES} byte uncompressed size limit"
      ));
    }
    *remaining_budget -= n;
    writer.write_all(&buf[..n as usize])?;
  }
  Ok(())
}

#[doc(hidden)]
pub fn copy_with_cap_for_tests(
  reader: impl Read,
  writer: &mut impl Write,
  remaining_budget: &mut u64,
) -> Result<()> {
  copy_with_cap(reader, writer, remaining_budget)
}

pub(crate) fn extract_tar_gz(bytes: Vec<u8>, dest: &Path, subdir: Option<&str>) -> Result<()> {
  std::fs::create_dir_all(dest)?;

  let gz = GzDecoder::new(Cursor::new(bytes));
  let mut archive = Archive::new(gz);

  let mut entry_count: usize = 0;
  let mut remaining_budget = MAX_TOTAL_UNCOMPRESSED_BYTES;

  for entry in archive.entries()? {
    entry_count += 1;
    if entry_count > MAX_ENTRIES {
      return Err(anyhow!("archive has more than {MAX_ENTRIES} entries"));
    }

    let mut entry = entry?;
    let raw_path = entry.path()?.into_owned();

    let Some(rel) = strip_archive_path(&raw_path, subdir) else {
      continue;
    };

    if !is_safe_relative(&rel) {
      return Err(anyhow!(
        "refusing to extract entry with unsafe path: {}",
        rel.display()
      ));
    }

    let entry_type = entry.header().entry_type();
    if entry_type.is_symlink() || entry_type.is_hard_link() {
      continue;
    }

    let out_path = dest.join(&rel);
    if let Some(parent) = out_path.parent() {
      std::fs::create_dir_all(parent)?;
    }

    if entry_type.is_dir() {
      std::fs::create_dir_all(&out_path)?;
      continue;
    }

    if !entry_type.is_file() {
      continue;
    }

    let mut out_file = std::fs::File::create(&out_path)?;
    copy_with_cap(&mut entry, &mut out_file, &mut remaining_budget)?;

    #[cfg(unix)]
    if let Ok(mode) = entry.header().mode() {
      use std::os::unix::fs::PermissionsExt;
      std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(mode & 0o777))?;
    }
  }

  Ok(())
}

#[doc(hidden)]
pub fn extract_tar_gz_for_tests(bytes: Vec<u8>, dest: &Path, subdir: Option<&str>) -> Result<()> {
  extract_tar_gz(bytes, dest, subdir)
}

pub async fn download_and_extract(
  client: &Client,
  archive_url: &str,
  dest: &Path,
  subdir: Option<&str>,
  token: Option<&str>,
) -> Result<()> {
  require_https_url(archive_url, "archive_url")?;
  let pb = ui::progress::download_bar(None);
  let bytes = download_capped(client, archive_url, token, Some(&pb)).await;
  pb.finish_and_clear();
  extract_tar_gz(bytes?, dest, subdir)
}

fn strip_archive_path(raw_path: &Path, subdir: Option<&str>) -> Option<PathBuf> {
  let mut components = raw_path.components();
  while matches!(components.clone().next(), Some(Component::CurDir)) {
    components.next();
  }
  components.next();
  let stripped = components.as_path();

  let rel: PathBuf = if let Some(dir) = subdir {
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

#[doc(hidden)]
pub fn strip_archive_path_for_tests(
  raw_path: &std::path::Path,
  subdir: Option<&str>,
) -> Option<std::path::PathBuf> {
  strip_archive_path(raw_path, subdir)
}

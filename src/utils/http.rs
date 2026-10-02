use anyhow::{Context, Result};
use futures::{StreamExt, TryStreamExt, stream};
use serde::de::DeserializeOwned;

use crate::{auth::token::get_auth_user, context::AppContext};

#[derive(serde::Deserialize)]
struct Paginated<T> {
  data: Vec<T>,
  total_pages: i64,
}

const MAX_TOTAL_PAGES: i64 = 1000;
const MAX_CONCURRENT_PAGE_FETCHES: usize = 8;

fn check_total_pages(total_pages: i64, label: &str) -> Result<()> {
  anyhow::ensure!(
    total_pages <= MAX_TOTAL_PAGES,
    "the {label} reports {total_pages} pages, more than the {MAX_TOTAL_PAGES}-page limit this client will fetch"
  );
  Ok(())
}

#[doc(hidden)]
pub fn check_total_pages_for_tests(total_pages: i64, label: &str) -> Result<()> {
  check_total_pages(total_pages, label)
}

#[doc(hidden)]
pub const MAX_TOTAL_PAGES_FOR_TESTS: i64 = MAX_TOTAL_PAGES;

pub async fn fetch_paginated<T: DeserializeOwned>(
  ctx: &AppContext,
  path: &str,
  label: &str,
) -> Result<Vec<T>> {
  let token = get_auth_user(&ctx.paths.auth).ok().map(|u| u.token);

  let fetch_page = |page: i64| {
    let mut req = ctx
      .client
      .get(format!("{}{path}", ctx.backend_url))
      .query(&[("page", page), ("page_size", 100)]);
    if let Some(token) = &token {
      req = req.bearer_auth(token);
    }
    async move {
      let res: Paginated<T> = req
        .send()
        .await
        .with_context(|| format!("Failed to connect to server for the {label}"))?
        .error_for_status()
        .with_context(|| format!("Server returned an error for the {label}"))?
        .json()
        .await
        .with_context(|| format!("Failed to parse the {label}"))?;
      Ok::<_, anyhow::Error>(res)
    }
  };

  let first = fetch_page(1).await?;
  let total_pages = first.total_pages;
  let mut all = first.data;
  if total_pages > 1 {
    check_total_pages(total_pages, label)?;
    let rest: Vec<Paginated<T>> = stream::iter(2..=total_pages)
      .map(fetch_page)
      .buffer_unordered(MAX_CONCURRENT_PAGE_FETCHES)
      .try_collect()
      .await?;
    for page in rest {
      all.extend(page.data);
    }
  }
  Ok(all)
}

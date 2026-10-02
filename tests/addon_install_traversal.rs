mod common;

use anesis::addons::install::install_addon;
use common::fixture::Fixture;

#[tokio::test]
async fn addon_id_with_dotdot_is_rejected_before_any_fs_write() {
  let fx = Fixture::new();
  let ctx = fx.offline_ctx();

  let err = install_addon(&ctx, "../../../../tmp/pwned")
    .await
    .expect_err("a traversing addon id must be rejected");

  assert!(
    format!("{err:#}").to_lowercase().contains("invalid") || format!("{err:#}").contains(".."),
    "unexpected error: {err:#}"
  );
  assert!(
    ctx
      .paths
      .home
      .parent()
      .map(|p| !p.join("tmp").join("pwned").exists())
      .unwrap_or(true),
    "install must not have written outside the addons cache"
  );
}

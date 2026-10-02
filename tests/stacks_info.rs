mod common;

use anesis::stacks::info::{print_installed_stacks, stack_info};
use common::fixture::{Fixture, build};

#[test]
fn print_installed_stacks_reports_none_when_the_cache_is_empty() {
  let fx = Fixture::new();
  let ctx = fx.offline_ctx();
  print_installed_stacks(&ctx, false).unwrap();
  print_installed_stacks(&ctx, true).unwrap();
}

#[test]
fn print_installed_stacks_json_mode_serializes_every_cached_stack() {
  let fx = Fixture::new();
  let ctx = fx.offline_ctx();
  std::fs::create_dir_all(&ctx.paths.stacks).unwrap();
  std::fs::write(
    ctx.paths.stacks.join("one.json"),
    build::stack_manifest("one", "One", "react-vite-ts"),
  )
  .unwrap();
  std::fs::write(
    ctx.paths.stacks.join("two.json"),
    build::stack_manifest("two", "Two", "express-api"),
  )
  .unwrap();

  print_installed_stacks(&ctx, true).unwrap();
  print_installed_stacks(&ctx, false).unwrap();
}

#[tokio::test]
async fn stack_info_falls_back_to_the_cache_when_the_registry_is_unreachable() {
  let fx = Fixture::new();
  let ctx = fx.offline_ctx();
  std::fs::create_dir_all(&ctx.paths.stacks).unwrap();
  std::fs::write(
    ctx.paths.stacks.join("cached-stack.json"),
    build::stack_manifest("cached-stack", "Cached Stack", "react-vite-ts"),
  )
  .unwrap();

  stack_info(&ctx, "cached-stack", false).await.unwrap();
  stack_info(&ctx, "cached-stack", true).await.unwrap();
}

#[tokio::test]
async fn stack_info_propagates_the_original_network_error_when_nothing_is_cached() {
  let fx = Fixture::new();
  let ctx = fx.offline_ctx();

  let err = stack_info(&ctx, "never-installed", false)
    .await
    .expect_err("with no cache to fall back to, the connection error must surface");
  assert!(!err.to_string().is_empty());
}

mod common;

use anesis::addons::test::resolve_fixture_for_tests as resolve_fixture;
use common::fixture::Fixture;

#[test]
fn an_explicit_project_path_wins_when_it_is_a_directory() {
  let fx = Fixture::new();
  let ctx = fx.offline_ctx();

  let result = resolve_fixture(
    &ctx,
    "any-addon",
    Some(fx.project.path().display().to_string()),
  )
  .unwrap();

  assert_eq!(result, fx.project.path());
}

#[test]
fn an_explicit_project_path_that_is_not_a_directory_is_an_error() {
  let fx = Fixture::new();
  let ctx = fx.offline_ctx();
  let not_a_dir = fx.project.path().join("does-not-exist");

  let err = resolve_fixture(&ctx, "any-addon", Some(not_a_dir.display().to_string()))
    .expect_err("a missing --project path must be rejected");
  assert!(err.to_string().contains("is not a directory"));
}

#[test]
fn falls_back_to_the_addons_bundled_test_fixture_directory() {
  let fx = Fixture::new();
  let ctx = fx.offline_ctx();
  let bundled = ctx.paths.addons.join("my-addon").join("test-fixture");
  std::fs::create_dir_all(&bundled).unwrap();

  let result = resolve_fixture(&ctx, "my-addon", None).unwrap();
  assert_eq!(result, bundled);
}

#[test]
fn errors_when_neither_a_project_flag_nor_a_bundled_fixture_exists() {
  let fx = Fixture::new();
  let ctx = fx.offline_ctx();

  let err = resolve_fixture(&ctx, "no-fixture-addon", None)
    .expect_err("must fail when there is nothing to test against");
  assert!(err.to_string().contains("No fixture project"));
}

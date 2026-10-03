mod common;

use anesis::addons::runner::run_addon_command;
use anesis::utils::cleanup::set_interrupted_for_tests;
use anesis::utils::errors::{exit_code, exit_code_for};
use common::fixture::{Fixture, build};
use std::collections::HashMap;

#[tokio::test]
async fn an_interrupt_stops_the_command_before_any_step_runs_and_exits_with_130() {
  let fx = Fixture::new();
  fx.seed_project();
  fx.install_addon(
    "interrupt-addon",
    &build::addon_manifest("interrupt-addon", "1.0.0"),
  );
  let ctx = fx.offline_ctx();

  set_interrupted_for_tests(true);
  let result = run_addon_command(
    &ctx,
    "interrupt-addon",
    "install",
    fx.project.path(),
    &HashMap::new(),
    true,
    false,
  )
  .await;
  set_interrupted_for_tests(false);

  let err = result.expect_err("an interrupted run must fail");
  assert_eq!(exit_code_for(&err), exit_code::INTERRUPTED);
  assert!(!fx.project.path().join("generated.txt").exists());
  assert!(!fx.project.path().join("anesis.lock").exists());
}

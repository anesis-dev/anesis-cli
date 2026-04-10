use assert_cmd::Command;
use predicates::str::contains;

fn cmd() -> Command {
  assert_cmd::cargo::cargo_bin_cmd!("oxide")
}

// ── Help / top-level ─────────────────────────────────────────────────────────

#[test]
fn help_flag() {
  cmd().arg("--help").assert().success().stdout(contains("Usage"));
}

#[test]
fn no_args_shows_help() {
  // clap prints help and exits with error when no subcommand is given
  cmd().assert().failure();
}

// ── template subcommand ───────────────────────────────────────────────────────

#[test]
fn template_help() {
  cmd()
    .args(["template", "--help"])
    .assert()
    .success()
    .stdout(contains("install"))
    .stdout(contains("list"))
    .stdout(contains("remove"))
    .stdout(contains("publish"));
}

#[test]
fn template_install_missing_arg() {
  cmd()
    .args(["template", "install"])
    .assert()
    .failure()
    .stderr(contains("TEMPLATE_NAME"));
}

#[test]
fn template_remove_missing_arg() {
  cmd()
    .args(["template", "remove"])
    .assert()
    .failure()
    .stderr(contains("TEMPLATE_NAME"));
}

#[test]
fn template_publish_missing_arg() {
  cmd()
    .args(["template", "publish"])
    .assert()
    .failure()
    .stderr(contains("TEMPLATE_URL"));
}

#[test]
fn template_unknown_subcommand() {
  cmd()
    .args(["template", "frobnicate"])
    .assert()
    .failure()
    .stderr(contains("unrecognized subcommand"));
}

// ── addon subcommand ──────────────────────────────────────────────────────────

#[test]
fn addon_help() {
  cmd()
    .args(["addon", "--help"])
    .assert()
    .success()
    .stdout(contains("install"))
    .stdout(contains("list"))
    .stdout(contains("remove"));
}

#[test]
fn addon_install_missing_arg() {
  cmd()
    .args(["addon", "install"])
    .assert()
    .failure()
    .stderr(contains("ADDON_ID"));
}

#[test]
fn addon_remove_missing_arg() {
  cmd()
    .args(["addon", "remove"])
    .assert()
    .failure()
    .stderr(contains("ADDON_ID"));
}

#[test]
fn addon_unknown_subcommand() {
  cmd()
    .args(["addon", "frobnicate"])
    .assert()
    .failure()
    .stderr(contains("unrecognized subcommand"));
}

// ── new subcommand ────────────────────────────────────────────────────────────

#[test]
fn new_help() {
  cmd()
    .args(["new", "--help"])
    .assert()
    .success()
    .stdout(contains("template"));
}

#[test]
fn new_missing_both_args() {
  cmd()
    .arg("new")
    .assert()
    .failure()
    .stderr(contains("NAME"));
}

#[test]
fn new_missing_template_arg() {
  cmd()
    .args(["new", "my-project"])
    .assert()
    .failure()
    .stderr(contains("TEMPLATE_NAME"));
}

// ── auth subcommands ──────────────────────────────────────────────────────────

#[test]
fn login_help() {
  cmd()
    .args(["login", "--help"])
    .assert()
    .success();
}

#[test]
fn logout_help() {
  cmd()
    .args(["logout", "--help"])
    .assert()
    .success();
}

#[test]
fn account_help() {
  cmd()
    .args(["account", "--help"])
    .assert()
    .success();
}

// ── aliases ───────────────────────────────────────────────────────────────────

#[test]
fn alias_t_for_template() {
  cmd()
    .args(["t", "--help"])
    .assert()
    .success()
    .stdout(contains("install"));
}

#[test]
fn alias_n_for_new() {
  cmd()
    .args(["n", "--help"])
    .assert()
    .success();
}

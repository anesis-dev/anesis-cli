use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

fn cmd() -> Command {
  assert_cmd::cargo::cargo_bin_cmd!("anesis")
}

#[test]
fn help_flag() {
  cmd()
    .arg("--help")
    .assert()
    .success()
    .stdout(contains("Usage"));
}

#[test]
fn ascii_flag_swaps_unicode_glyphs_for_ascii() {
  let home = assert_fs::TempDir::new().unwrap();
  cmd()
    .env("ANESIS_HOME", home.path())
    .env_remove("ANESIS_TOKEN")
    .args(["--ascii", "info"])
    .assert()
    .success()
    .stdout(contains("[x] not logged in"))
    .stdout(contains("✗").not());
}

#[test]
fn no_args_shows_help() {
  cmd()
    .assert()
    .failure()
    .code(2)
    .stderr(contains("Commands:"))
    .stderr(contains("Scaffold projects from remote templates"));
}

#[test]
fn top_level_help_lists_visible_aliases() {
  cmd()
    .arg("--help")
    .assert()
    .success()
    .stdout(contains("[aliases: n]"))
    .stdout(contains("[aliases: t]"))
    .stdout(contains("[aliases: a]"))
    .stdout(contains("[aliases: s]"))
    .stdout(contains("[aliases: in]"))
    .stdout(contains("[aliases: out]"));
}

#[test]
fn doctor_is_a_real_command_not_an_info_alias() {
  cmd()
    .arg("--help")
    .assert()
    .success()
    .stdout(contains("doctor"))
    .stdout(contains("Diagnose"));
}

#[test]
fn top_level_help_has_an_about_line() {
  cmd()
    .arg("-h")
    .assert()
    .success()
    .stdout(contains("Scaffold projects from remote templates"));

  cmd()
    .arg("--help")
    .assert()
    .success()
    .stdout(contains("Anesis scaffolds a project from a template"))
    .stdout(contains("anesis new"));
}

#[test]
fn short_aliases_resolve_to_their_command() {
  for (alias, canonical) in [("t", "template"), ("a", "addon"), ("s", "stack")] {
    let aliased = cmd().args([alias, "--help"]).assert().success();
    let direct = cmd().args([canonical, "--help"]).assert().success();
    assert_eq!(
      String::from_utf8_lossy(&aliased.get_output().stdout),
      String::from_utf8_lossy(&direct.get_output().stdout),
      "`anesis {alias}` should be `anesis {canonical}`"
    );
  }
}

#[test]
fn template_help() {
  cmd()
    .args(["template", "--help"])
    .assert()
    .success()
    .stdout(contains("Manage templates"))
    .stdout(contains("install"))
    .stdout(contains("list"))
    .stdout(contains("remove"))
    .stdout(contains("publish"));
}

#[test]
fn template_install_arg_is_optional() {
  cmd()
    .args(["template", "install", "--help"])
    .assert()
    .success()
    .stdout(contains("[TEMPLATE_NAME]"))
    .stdout(contains("pick interactively"));
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
fn template_publish_rejects_a_non_github_url() {
  cmd()
    .args(["template", "publish", "https://gitlab.com/owner/repo"])
    .assert()
    .failure()
    .stderr(contains("GitHub"));
}

#[test]
fn template_republish_rejects_a_non_github_url() {
  cmd()
    .args(["template", "republish", "https://gitlab.com/owner/repo"])
    .assert()
    .failure()
    .stderr(contains("GitHub"));
}

#[test]
fn template_unknown_subcommand() {
  cmd()
    .args(["template", "frobnicate"])
    .assert()
    .failure()
    .stderr(contains("unrecognized subcommand"));
}

#[test]
fn addon_help() {
  cmd()
    .args(["addon", "--help"])
    .assert()
    .success()
    .stdout(contains("Manage addons"))
    .stdout(contains("install"))
    .stdout(contains("list"))
    .stdout(contains("remove"))
    .stdout(contains("lint"));
}

#[test]
fn addon_install_arg_is_optional() {
  cmd()
    .args(["addon", "install", "--help"])
    .assert()
    .success()
    .stdout(contains("[ADDON_ID]"))
    .stdout(contains("pick interactively"));
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

#[test]
fn new_help() {
  cmd()
    .args(["new", "--help"])
    .assert()
    .success()
    .stdout(contains("Create a new project from a template"))
    .stdout(contains("template"));
}

#[test]
fn new_missing_both_args() {
  cmd().arg("new").assert().failure().stderr(contains("NAME"));
}

#[test]
fn new_template_arg_is_optional() {
  cmd()
    .args(["new", "--help"])
    .assert()
    .success()
    .stdout(contains("[TEMPLATE_NAME]"))
    .stdout(contains("pick interactively"));
}

#[test]
fn login_help() {
  cmd()
    .args(["login", "--help"])
    .assert()
    .success()
    .stdout(contains("Log in to your Anesis account"));
}

#[test]
fn logout_help() {
  cmd()
    .args(["logout", "--help"])
    .assert()
    .success()
    .stdout(contains("Log out of your Anesis account"));
}

#[test]
fn account_help() {
  cmd().args(["account", "--help"]).assert().success();
}

#[test]
fn upgrade_help() {
  cmd()
    .args(["upgrade", "--help"])
    .assert()
    .success()
    .stdout(contains("latest Anesis release"));
}

#[test]
fn use_help() {
  cmd()
    .args(["use", "--help"])
    .assert()
    .success()
    .stdout(contains("Usage: anesis use [OPTIONS] [ADDON_ID] [COMMAND]"));
}

#[test]
fn use_addon_id_is_optional() {
  cmd()
    .args(["use", "--help"])
    .assert()
    .success()
    .stdout(contains("[ADDON_ID]"))
    .stdout(contains("pick interactively"));
}

#[test]
fn top_level_addon_execution_is_not_available_anymore() {
  cmd()
    .args(["drizzle", "install"])
    .assert()
    .failure()
    .stderr(contains("unrecognized subcommand"));
}

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
  cmd().args(["n", "--help"]).assert().success();
}

#[test]
fn alias_in_for_login() {
  cmd().args(["in", "--help"]).assert().success();
}

#[test]
fn alias_out_for_logout() {
  cmd().args(["out", "--help"]).assert().success();
}

#[test]
fn alias_a_for_addon() {
  cmd()
    .args(["a", "--help"])
    .assert()
    .success()
    .stdout(contains("install"));
}

#[test]
fn template_update_missing_arg() {
  cmd()
    .args(["template", "update"])
    .assert()
    .failure()
    .stderr(contains("TEMPLATE_URL"));
}

#[test]
fn addon_publish_missing_arg() {
  cmd()
    .args(["addon", "publish"])
    .assert()
    .failure()
    .stderr(contains("ADDON_URL"));
}

#[test]
fn addon_update_missing_arg() {
  cmd()
    .args(["addon", "update"])
    .assert()
    .failure()
    .stderr(contains("ADDON_URL"));
}

#[test]
fn version_flag() {
  cmd()
    .arg("--version")
    .assert()
    .success()
    .stdout(contains("anesis"));
}

#[test]
fn republish_replaces_the_publish_side_update_verb() {
  for group in ["template", "addon", "stack"] {
    cmd()
      .args([group, "--help"])
      .assert()
      .success()
      .stdout(contains("republish"))
      .stdout(contains("Refresh"));

    let output = cmd().args([group, "update", "--help"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(
      stdout.contains("Refresh"),
      "`{group} update` should still reach republish, got: {stdout}"
    );
  }
}

#[test]
fn upgrade_advertises_the_self_update_alias() {
  cmd()
    .arg("--help")
    .assert()
    .success()
    .stdout(contains("self-update"));

  cmd()
    .args(["self-update", "--help"])
    .assert()
    .success()
    .stdout(contains("latest Anesis release"));
}

#[test]
fn update_help_disambiguates_the_three_verbs() {
  cmd()
    .args(["update", "--help"])
    .assert()
    .success()
    .stdout(contains("anesis upgrade"))
    .stdout(contains("anesis addon republish"));
}

#[test]
fn update_without_an_addon_id_or_all_is_an_error() {
  cmd()
    .args(["update"])
    .assert()
    .failure()
    .stderr(contains("--all"));
}

#[test]
fn update_with_both_an_addon_id_and_all_is_an_error() {
  cmd()
    .args(["update", "some-addon", "--all"])
    .assert()
    .failure()
    .stderr(contains("both"));
}

#[test]
fn stack_link_exists_for_symmetry_with_template_and_addon() {
  cmd()
    .args(["stack", "--help"])
    .assert()
    .success()
    .stdout(contains("link"));

  cmd()
    .args(["stack", "link", "--help"])
    .assert()
    .success()
    .stdout(contains("anesis.stack.json"))
    .stdout(contains("--force"));
}

#[test]
fn outdated_accepts_json() {
  cmd()
    .args(["outdated", "--help"])
    .assert()
    .success()
    .stdout(contains("--json"));
}

#[test]
fn global_presentation_flags_are_available_on_every_subcommand() {
  for args in [
    vec!["--help"],
    vec!["new", "--help"],
    vec!["addon", "install", "--help"],
  ] {
    let output = cmd().args(&args).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    for flag in ["--verbose", "--quiet", "--no-color"] {
      assert!(
        stdout.contains(flag),
        "{args:?} help is missing the global {flag}: {stdout}"
      );
    }
  }
}

#[test]
fn verbose_and_quiet_conflict() {
  cmd()
    .args(["--verbose", "--quiet", "info"])
    .assert()
    .failure()
    .code(2)
    .stderr(contains("cannot be used with"));
}

#[test]
fn quiet_keeps_command_output_and_exit_code() {
  let home = assert_fs::TempDir::new().unwrap();
  cmd()
    .env("HOME", home.path())
    .env("USERPROFILE", home.path())
    .env("ANESIS_HOME", home.path())
    .args(["--quiet", "info"])
    .assert()
    .success()
    .stdout(contains("anesis"));
}

#[test]
fn man_writes_a_page_per_command() {
  let dir = assert_fs::TempDir::new().unwrap();
  cmd()
    .args(["man", dir.path().to_str().unwrap()])
    .assert()
    .success()
    .stdout(contains("man pages"));

  for page in [
    "anesis.1",
    "anesis-new.1",
    "anesis-addon.1",
    "anesis-addon-install.1",
    "anesis-stack-link.1",
    "anesis-template-republish.1",
  ] {
    assert!(
      dir.path().join(page).exists(),
      "{page} was not generated in {}",
      dir.path().display()
    );
  }

  let root = std::fs::read_to_string(dir.path().join("anesis.1")).unwrap();
  assert!(root.contains(".TH"), "not a roff page: {root}");
  assert!(root.contains("anesis"), "{root}");

  assert!(
    !dir.path().join("anesis-man.1").exists(),
    "the hidden `man` command should not get its own page"
  );
}

#[test]
fn man_is_hidden_from_help() {
  let output = cmd().arg("--help").assert().success();
  let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
  assert!(
    !stdout.contains("\n  man"),
    "the man command should be hidden from --help: {stdout}"
  );
}

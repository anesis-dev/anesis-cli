use assert_cmd::Command;
use assert_fs::TempDir;

fn command(home: &TempDir) -> Command {
  let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("anesis");
  cmd
    .env("HOME", home.path())
    .env("USERPROFILE", home.path())
    .env("ANESIS_HOME", home.path())
    .env_remove("XDG_CONFIG_HOME")
    .env_remove("ZDOTDIR");
  cmd
}

#[test]
fn print_mode_writes_a_real_completion_script_to_stdout_for_every_shell() {
  let home = TempDir::new().unwrap();
  for shell in ["bash", "zsh", "fish", "powershell"] {
    let output = command(&home)
      .args(["completions", shell, "--print"])
      .assert()
      .success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout).into_owned();
    assert!(
      stdout.contains("anesis"),
      "{shell} completion script should mention the binary name: {stdout}"
    );
  }
}

#[test]
fn print_mode_does_not_write_any_files() {
  let home = TempDir::new().unwrap();
  command(&home)
    .args(["completions", "bash", "--print"])
    .assert()
    .success();
  assert!(
    !home.path().join(".local").exists(),
    "--print must not install anything"
  );
}

#[test]
fn install_bash_writes_to_the_xdg_bash_completion_dir() {
  let home = TempDir::new().unwrap();
  command(&home)
    .args(["completions", "bash"])
    .assert()
    .success();

  let dest = home
    .path()
    .join(".local/share/bash-completion/completions/anesis");
  assert!(dest.exists(), "expected a completion script at {dest:?}");
  let content = std::fs::read_to_string(&dest).unwrap();
  assert!(content.contains("anesis"));
}

#[test]
fn install_fish_writes_to_the_xdg_config_fish_completions_dir() {
  let home = TempDir::new().unwrap();
  let xdg_config = home.path().join("custom-config");
  command(&home)
    .env("XDG_CONFIG_HOME", &xdg_config)
    .args(["completions", "fish"])
    .assert()
    .success();

  let dest = xdg_config.join("fish/completions/anesis.fish");
  assert!(dest.exists(), "expected a completion script at {dest:?}");
}

#[test]
fn install_fish_falls_back_to_home_dot_config_without_xdg_config_home() {
  let home = TempDir::new().unwrap();
  command(&home)
    .args(["completions", "fish"])
    .assert()
    .success();

  let dest = home.path().join(".config/fish/completions/anesis.fish");
  assert!(dest.exists(), "expected a completion script at {dest:?}");
}

#[test]
fn install_zsh_without_zdotdir_writes_to_zfunc_and_updates_zshrc() {
  let home = TempDir::new().unwrap();
  command(&home)
    .args(["completions", "zsh"])
    .assert()
    .success();

  let dest = home.path().join(".zfunc/_anesis");
  assert!(dest.exists(), "expected a completion script at {dest:?}");

  let zshrc = std::fs::read_to_string(home.path().join(".zshrc")).unwrap();
  assert!(zshrc.contains("anesis completions start"));
  assert!(zshrc.contains(".zfunc"));
}

#[test]
fn installing_twice_is_idempotent_for_the_zshrc_managed_block() {
  let home = TempDir::new().unwrap();
  command(&home)
    .args(["completions", "zsh"])
    .assert()
    .success();
  let first = std::fs::read_to_string(home.path().join(".zshrc")).unwrap();

  command(&home)
    .args(["completions", "zsh"])
    .assert()
    .success();
  let second = std::fs::read_to_string(home.path().join(".zshrc")).unwrap();

  assert_eq!(
    first, second,
    "re-installing must not duplicate the managed block"
  );
}

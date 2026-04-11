use std::{
  collections::HashSet,
  fs,
  path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use colored::Colorize;

use crate::addons::{cache::AddonsCache, manifest::AddonManifest};

/// Install the shell completion script to the appropriate location automatically.
pub fn install_completions(shell: &str) -> Result<()> {
  match shell {
    "bash" => install_bash(),
    "zsh" => install_zsh(),
    "fish" => install_fish(),
    other => {
      eprintln!("Unsupported shell '{other}'. Supported: bash, zsh, fish");
      Ok(())
    }
  }
}

fn install_bash() -> Result<()> {
  // ~/.local/share/bash-completion/completions/ is the standard user-level location
  // that bash-completion picks up automatically — no .bashrc changes needed.
  let dir = bash_completions_dir()?;
  fs::create_dir_all(&dir)
    .with_context(|| format!("Failed to create directory {}", dir.display()))?;
  let dest = dir.join("oxide");
  fs::write(&dest, BASH_SCRIPT)
    .with_context(|| format!("Failed to write {}", dest.display()))?;

  println!("{} Bash completions installed to {}", "✔".green(), dest.display());
  println!(
    "  Open a new terminal, or run: {}",
    format!("source {}", dest.display()).cyan()
  );
  println!("  Note: requires the {} package to be installed.", "bash-completion".cyan());
  Ok(())
}

fn bash_completions_dir() -> Result<PathBuf> {
  let home = dirs::home_dir().context("Could not determine home directory")?;
  Ok(home.join(".local/share/bash-completion/completions"))
}

fn install_zsh() -> Result<()> {
  // If the user has a $ZDOTDIR/completions/ directory (e.g. HyDE), write a
  // directly-sourceable script there — no fpath changes needed.
  // Otherwise fall back to the standard ~/.zfunc/_oxide fpath approach.
  if let Some(dir) = zdotdir_completions_dir() {
    fs::create_dir_all(&dir)
      .with_context(|| format!("Failed to create directory {}", dir.display()))?;
    let dest = dir.join("oxide.zsh");
    fs::write(&dest, ZSH_SOURCED_SCRIPT)
      .with_context(|| format!("Failed to write {}", dest.display()))?;
    println!("{} Zsh completions installed to {}", "✔".green(), dest.display());
    println!("  Restart your terminal or open a new tab — completions are active immediately.");
  } else {
    let dir = home_zfunc_dir()?;
    fs::create_dir_all(&dir)
      .with_context(|| format!("Failed to create directory {}", dir.display()))?;
    let dest = dir.join("_oxide");
    fs::write(&dest, ZSH_FPATH_SCRIPT)
      .with_context(|| format!("Failed to write {}", dest.display()))?;
    println!("{} Zsh completions installed to {}", "✔".green(), dest.display());
    println!("  Ensure your ~/.zshrc contains:");
    println!("    {}", "fpath=(~/.zfunc $fpath)".cyan());
    println!("    {}", "autoload -Uz compinit && compinit".cyan());
  }
  Ok(())
}

/// Returns `$ZDOTDIR/completions/` only when it looks like a HyDE setup:
/// the directory exists AND `$ZDOTDIR/.hyde.zshrc` or `hyde-cli` is present.
fn zdotdir_completions_dir() -> Option<PathBuf> {
  // Only check an explicit $ZDOTDIR — do not fall back to ~/.config/zsh,
  // because that directory may exist on non-HyDE systems without any
  // auto-sourcing of its contents.
  let zdotdir = std::env::var("ZDOTDIR").map(PathBuf::from).ok()?;
  let dir = zdotdir.join("completions");
  if !dir.is_dir() {
    return None;
  }
  // Confirm this is a HyDE environment before using the sourced-script path.
  let is_hyde = zdotdir.join(".hyde.zshrc").exists()
    || which::which("hyde-cli").is_ok();
  if is_hyde { Some(dir) } else { None }
}

fn home_zfunc_dir() -> Result<PathBuf> {
  let home = dirs::home_dir().context("Could not determine home directory")?;
  Ok(home.join(".zfunc"))
}

fn install_fish() -> Result<()> {
  // ~/.config/fish/completions/ is the standard location for fish completions.
  let dir = fish_completions_dir()?;
  fs::create_dir_all(&dir)
    .with_context(|| format!("Failed to create directory {}", dir.display()))?;
  let dest = dir.join("oxide.fish");
  fs::write(&dest, FISH_SCRIPT)
    .with_context(|| format!("Failed to write {}", dest.display()))?;

  println!("{} Fish completions installed to {}", "✔".green(), dest.display());
  println!("  Completions are active immediately in new fish sessions.");
  Ok(())
}

fn fish_completions_dir() -> Result<PathBuf> {
  // Respect $XDG_CONFIG_HOME if set, otherwise fall back to ~/.config
  let config_dir = std::env::var("XDG_CONFIG_HOME")
    .map(PathBuf::from)
    .unwrap_or_else(|_| {
      dirs::home_dir()
        .expect("Could not determine home directory")
        .join(".config")
    });
  Ok(config_dir.join("fish/completions"))
}

/// Print dynamic completions to stdout — called by the generated completion scripts.
///
/// - No `addon_id`: prints installed addon IDs, one per line.
/// - With `addon_id`: prints that addon's command names, one per line.
///
/// Errors are silently ignored so a broken cache never crashes the shell.
pub fn print_dynamic_completions(addons_dir: &Path, addon_id: Option<&str>) {
  match addon_id {
    None => print_addon_ids(addons_dir),
    Some(id) => print_addon_commands(addons_dir, id),
  }
}

fn print_addon_ids(addons_dir: &Path) {
  let index = addons_dir.join("oxide-addons.json");
  let Ok(content) = std::fs::read_to_string(&index) else {
    return;
  };
  let Ok(cache) = serde_json::from_str::<AddonsCache>(&content) else {
    return;
  };
  for addon in &cache.addons {
    println!("{}", addon.id);
  }
}

fn print_addon_commands(addons_dir: &Path, addon_id: &str) {
  let manifest_path = addons_dir.join(addon_id).join("oxide.addon.json");
  let Ok(content) = std::fs::read_to_string(&manifest_path) else {
    return;
  };
  let Ok(manifest) = serde_json::from_str::<AddonManifest>(&content) else {
    return;
  };
  let mut seen: HashSet<String> = HashSet::new();
  for variant in &manifest.variants {
    for cmd in &variant.commands {
      if seen.insert(cmd.name.clone()) {
        println!("{}", cmd.name);
      }
    }
  }
}

// ── Shell scripts ─────────────────────────────────────────────────────────────

const BASH_SCRIPT: &str = r#"# oxide shell completions for bash
# Source this file or append it to ~/.bashrc:
#   oxide completions bash >> ~/.bashrc

_oxide_completions() {
  local cur prev
  cur="${COMP_WORDS[COMP_CWORD]}"
  prev="${COMP_WORDS[COMP_CWORD-1]}"

  local static_top="new template addon login logout account completions"
  local template_subs="install list remove publish update"
  local addon_subs="install list remove publish update"

  case $COMP_CWORD in
    1)
      local addon_ids
      addon_ids=$(oxide _complete 2>/dev/null)
      COMPREPLY=($(compgen -W "$static_top $addon_ids" -- "$cur"))
      ;;
    2)
      case $prev in
        template|t) COMPREPLY=($(compgen -W "$template_subs" -- "$cur")) ;;
        addon|a)     COMPREPLY=($(compgen -W "$addon_subs" -- "$cur")) ;;
        *)
          local addon_cmds
          addon_cmds=$(oxide _complete "$prev" 2>/dev/null)
          if [ -n "$addon_cmds" ]; then
            COMPREPLY=($(compgen -W "$addon_cmds" -- "$cur"))
          fi
          ;;
      esac
      ;;
  esac
}

complete -F _oxide_completions oxide
"#;

// Used when saving to $ZDOTDIR/completions/ (e.g. HyDE) — sourced directly,
// so no #compdef header; registers via `compdef` instead.
const ZSH_SOURCED_SCRIPT: &str = r#"# oxide shell completions for zsh
# Auto-installed by: oxide completions zsh
# Loaded automatically by your shell config via _load_completions().

if command -v oxide &>/dev/null; then
  _oxide() {
    local state

    _arguments \
      '1: :->cmd' \
      '2: :->subcmd' && return 0

    case $state in
      cmd)
        local static_cmds=(
          'new:Create a new project from a template (oxide n)'
          'template:Manage templates (oxide t)'
          'addon:Manage addons (oxide a)'
          'login:Log in to your Oxide account (oxide in)'
          'logout:Log out of your Oxide account (oxide out)'
          'account:Show account information'
          'completions:Install shell completion script'
        )
        local addon_ids
        addon_ids=(${(f)"$(oxide _complete 2>/dev/null)"})
        _describe 'command' static_cmds
        [[ ${#addon_ids[@]} -gt 0 ]] && compadd -- "${addon_ids[@]}"
        ;;
      subcmd)
        case ${words[2]} in
          template|t)
            local subs=(
              'install:Download and cache a template (i)'
              'list:List installed templates (l)'
              'remove:Remove a template from cache (r)'
              'publish:Publish a GitHub repository as a template (p)'
              'update:Update a published template (u)'
            )
            _describe 'subcommand' subs
            ;;
          addon|a)
            local subs=(
              'install:Install and cache an addon (i)'
              'list:List installed addons (l)'
              'remove:Remove a cached addon (r)'
              'publish:Publish a GitHub repository as an addon (p)'
              'update:Update a published addon (u)'
            )
            _describe 'subcommand' subs
            ;;
          *)
            local addon_cmds
            addon_cmds=(${(f)"$(oxide _complete "${words[2]}" 2>/dev/null)"})
            [[ ${#addon_cmds[@]} -gt 0 ]] && compadd -- "${addon_cmds[@]}"
            ;;
        esac
        ;;
    esac
  }

  compdef _oxide oxide
fi
"#;

// Used when saving to ~/.zfunc/_oxide — loaded via fpath, needs #compdef header.
const ZSH_FPATH_SCRIPT: &str = r#"#compdef oxide
# oxide shell completions for zsh
# Auto-installed by: oxide completions zsh
# Requires ~/.zfunc in your fpath and compinit called in ~/.zshrc.

_oxide() {
  local state

  _arguments \
    '1: :->cmd' \
    '2: :->subcmd' && return 0

  case $state in
    cmd)
      local static_cmds=(
        'new:Create a new project from a template (oxide n)'
        'template:Manage templates (oxide t)'
        'addon:Manage addons (oxide a)'
        'login:Log in to your Oxide account (oxide in)'
        'logout:Log out of your Oxide account (oxide out)'
        'account:Show account information'
        'completions:Install shell completion script'
      )
      local addon_ids
      addon_ids=(${(f)"$(oxide _complete 2>/dev/null)"})
      _describe 'command' static_cmds
      [[ ${#addon_ids[@]} -gt 0 ]] && compadd -- "${addon_ids[@]}"
      ;;
    subcmd)
      case ${words[2]} in
        template|t)
          local subs=(
            'install:Download and cache a template (i)'
            'list:List installed templates (l)'
            'remove:Remove a template from cache (r)'
            'publish:Publish a GitHub repository as a template (p)'
            'update:Update a published template (u)'
          )
          _describe 'subcommand' subs
          ;;
        addon|a)
          local subs=(
            'install:Install and cache an addon (i)'
            'list:List installed addons (l)'
            'remove:Remove a cached addon (r)'
            'publish:Publish a GitHub repository as an addon (p)'
            'update:Update a published addon (u)'
          )
          _describe 'subcommand' subs
          ;;
        *)
          local addon_cmds
          addon_cmds=(${(f)"$(oxide _complete "${words[2]}" 2>/dev/null)"})
          [[ ${#addon_cmds[@]} -gt 0 ]] && compadd -- "${addon_cmds[@]}"
          ;;
      esac
      ;;
  esac
}

_oxide
"#;

const FISH_SCRIPT: &str = r#"# oxide shell completions for fish
# Save to your fish completions directory:
#   oxide completions fish > ~/.config/fish/completions/oxide.fish

# Disable file completions for oxide globally
complete -c oxide -f

# ── Top-level subcommands ──────────────────────────────────────────────────────
complete -c oxide -n '__fish_use_subcommand' -a 'new'         -d 'Create a new project from a template (oxide n)'
complete -c oxide -n '__fish_use_subcommand' -a 'template'    -d 'Manage templates (oxide t)'
complete -c oxide -n '__fish_use_subcommand' -a 'addon'       -d 'Manage addons (oxide a)'
complete -c oxide -n '__fish_use_subcommand' -a 'login'       -d 'Log in to your Oxide account (oxide in)'
complete -c oxide -n '__fish_use_subcommand' -a 'logout'      -d 'Log out of your Oxide account (oxide out)'
complete -c oxide -n '__fish_use_subcommand' -a 'account'     -d 'Show account information'
complete -c oxide -n '__fish_use_subcommand' -a 'completions' -d 'Install shell completion script'

# Dynamic addon IDs from cache (automatically updated as addons are installed)
complete -c oxide -n '__fish_use_subcommand' -a '(oxide _complete 2>/dev/null)'

# ── template subcommands ───────────────────────────────────────────────────────
complete -c oxide -n '__fish_seen_subcommand_from template t' -a 'install' -d 'Download and cache a template (i)'
complete -c oxide -n '__fish_seen_subcommand_from template t' -a 'list'    -d 'List installed templates (l)'
complete -c oxide -n '__fish_seen_subcommand_from template t' -a 'remove'  -d 'Remove a template from cache (r)'
complete -c oxide -n '__fish_seen_subcommand_from template t' -a 'publish' -d 'Publish a GitHub repository as a template (p)'
complete -c oxide -n '__fish_seen_subcommand_from template t' -a 'update'  -d 'Update a published template (u)'

# ── addon subcommands ──────────────────────────────────────────────────────────
complete -c oxide -n '__fish_seen_subcommand_from addon a' -a 'install' -d 'Install and cache an addon (i)'
complete -c oxide -n '__fish_seen_subcommand_from addon a' -a 'list'    -d 'List installed addons (l)'
complete -c oxide -n '__fish_seen_subcommand_from addon a' -a 'remove'  -d 'Remove a cached addon (r)'
complete -c oxide -n '__fish_seen_subcommand_from addon a' -a 'publish' -d 'Publish a GitHub repository as an addon (p)'
complete -c oxide -n '__fish_seen_subcommand_from addon a' -a 'update'  -d 'Update a published addon (u)'

# ── Dynamic addon commands ─────────────────────────────────────────────────────
# When the second token is an installed addon ID, complete with its commands.
# This fires automatically after every `oxide addon install <id>` — no extra steps needed.
complete -c oxide \
  -n 'test (count (commandline -opc)) -eq 2; and not contains -- (commandline -opc)[2] new template addon login logout account completions' \
  -a '(oxide _complete (commandline -opc)[2] 2>/dev/null)'
"#;

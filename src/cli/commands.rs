use clap::Subcommand;

use crate::completions::CompletionShell;

#[derive(Subcommand)]
pub enum AddonCommands {
  #[command(visible_alias = "i", about = "Install and cache an addon")]
  Install {
    #[arg(help = "Addon id to install (omit to pick interactively)")]
    addon_id: Option<String>,
  },

  #[command(about = "Validate a local directory and cache it as an addon for local testing")]
  Link {
    #[arg(help = "Path to the addon directory (defaults to the current directory)")]
    path: Option<String>,

    #[arg(
      short,
      long,
      help = "Overwrite an existing cached addon without asking"
    )]
    force: bool,
  },

  #[command(visible_alias = "l", about = "List installed addons")]
  List {
    #[arg(long, help = "Output as JSON")]
    json: bool,
  },

  #[command(
    about = "Show an addon's manifest: description, version, variants, commands, inputs and steps"
  )]
  Info {
    #[arg(help = "Addon id to inspect")]
    addon_id: String,

    #[arg(long, help = "Output as JSON")]
    json: bool,
  },

  #[command(about = "Dry-run an addon command on a throwaway copy of a project and show the diff")]
  Test {
    #[arg(help = "Addon id to test")]
    addon_id: String,

    #[arg(help = "Command to run")]
    command: String,

    #[arg(
      long,
      value_name = "PATH",
      help = "Fixture project to test on (defaults to the addon's bundled test-fixture/)"
    )]
    project: Option<String>,
  },

  #[command(visible_alias = "r", about = "Remove a cached addon")]
  Remove { addon_id: String },

  #[command(
    about = "Check a local addon directory for common manifest mistakes (missing copy sources, unknown requires_commands, test-fixture anchors that won't match)"
  )]
  Lint {
    #[arg(help = "Path to the addon directory (defaults to the current directory)")]
    path: Option<String>,
  },

  #[command(
    visible_alias = "p",
    about = "Publish a GitHub repository as an Anesis addon"
  )]
  Publish {
    #[arg(help = "GitHub repository URL (e.g. https://github.com/owner/repo)")]
    addon_url: String,

    #[arg(
      long,
      value_name = "VISIBILITY",
      help = "Visibility: public, private, org-private (default: public)"
    )]
    visibility: Option<String>,

    #[arg(
      long,
      value_name = "UUID",
      help = "Credential ID for private GitHub repositories"
    )]
    credential_id: Option<String>,

    #[arg(long, value_name = "UUID", help = "Organization ID to publish under")]
    org_id: Option<String>,
  },

  #[command(
    visible_alias = "rp",
    aliases = ["u", "update"],
    about = "Refresh this addon's registry entry from its GitHub repository"
  )]
  Republish {
    #[arg(help = "GitHub repository URL (e.g. https://github.com/owner/repo)")]
    addon_url: String,

    #[arg(
      long,
      value_name = "VISIBILITY",
      help = "Visibility: public, private, org-private"
    )]
    visibility: Option<String>,

    #[arg(
      long,
      value_name = "UUID",
      help = "Credential ID for private GitHub repositories"
    )]
    credential_id: Option<String>,

    #[arg(long, value_name = "UUID", help = "Organization ID")]
    org_id: Option<String>,
  },
}

#[derive(Subcommand)]
pub enum TemplateCommands {
  #[command(visible_alias = "i", about = "Download and cache a template locally")]
  Install {
    #[arg(help = "Name of the template to install (omit to pick interactively)")]
    template_name: Option<String>,
  },

  #[command(about = "Validate a local directory and cache it as a template for local testing")]
  Link {
    #[arg(help = "Path to the template directory (defaults to the current directory)")]
    path: Option<String>,

    #[arg(
      short,
      long,
      help = "Overwrite an existing cached template without asking"
    )]
    force: bool,
  },

  #[command(visible_alias = "l", about = "List all locally installed templates")]
  List {
    #[arg(long, help = "Output as JSON")]
    json: bool,
  },

  #[command(about = "Show a template's manifest: description, version and repository")]
  Info {
    #[arg(help = "Name of the template to inspect")]
    template_name: String,

    #[arg(long, help = "Output as JSON")]
    json: bool,
  },

  #[command(
    visible_alias = "r",
    about = "Remove an installed template from the local cache"
  )]
  Remove {
    #[arg(help = "Name of the template to remove")]
    template_name: String,
  },

  #[command(
    visible_alias = "p",
    about = "Publish a GitHub repository as an Anesis template"
  )]
  Publish {
    #[arg(help = "GitHub repository URL (e.g. https://github.com/owner/repo)")]
    template_url: String,

    #[arg(
      long,
      value_name = "VISIBILITY",
      help = "Visibility: public, private, org-private (default: public)"
    )]
    visibility: Option<String>,

    #[arg(
      long,
      value_name = "UUID",
      help = "Credential ID for private GitHub repositories"
    )]
    credential_id: Option<String>,

    #[arg(long, value_name = "UUID", help = "Organization ID to publish under")]
    org_id: Option<String>,
  },

  #[command(
    visible_alias = "rp",
    aliases = ["u", "update"],
    about = "Refresh this template's registry entry from its GitHub repository"
  )]
  Republish {
    #[arg(help = "GitHub repository URL (e.g. https://github.com/owner/repo)")]
    template_url: String,

    #[arg(
      long,
      value_name = "VISIBILITY",
      help = "Visibility: public, private, org-private"
    )]
    visibility: Option<String>,

    #[arg(
      long,
      value_name = "UUID",
      help = "Credential ID for private GitHub repositories"
    )]
    credential_id: Option<String>,

    #[arg(long, value_name = "UUID", help = "Organization ID")]
    org_id: Option<String>,
  },
}

#[derive(Subcommand)]
pub enum StackCommands {
  #[command(
    visible_alias = "i",
    about = "Download and cache a stack from the registry"
  )]
  Install {
    #[arg(help = "Stack id to install")]
    stack_id: String,
  },

  #[command(about = "Validate a local anesis.stack.json and cache it for local testing")]
  Link {
    #[arg(help = "Path to the stack directory or manifest (defaults to the current directory)")]
    path: Option<String>,

    #[arg(
      short,
      long,
      help = "Overwrite an existing cached stack without asking"
    )]
    force: bool,
  },

  #[command(visible_alias = "l", about = "List locally installed stacks")]
  List {
    #[arg(long, help = "Output as JSON")]
    json: bool,
  },

  #[command(about = "Show a stack's composition: template and addons")]
  Info {
    #[arg(help = "Stack id to inspect")]
    stack_id: String,

    #[arg(long, help = "Output as JSON")]
    json: bool,
  },

  #[command(visible_alias = "r", about = "Remove a locally cached stack")]
  Remove {
    #[arg(help = "Stack id to remove")]
    stack_id: String,
  },

  #[command(
    visible_alias = "p",
    about = "Publish a GitHub repository as an Anesis stack"
  )]
  Publish {
    #[arg(help = "GitHub repository URL (e.g. https://github.com/owner/repo)")]
    stack_url: String,

    #[arg(
      long,
      value_name = "VISIBILITY",
      help = "Visibility: public, private (default: public)"
    )]
    visibility: Option<String>,

    #[arg(
      long,
      value_name = "UUID",
      help = "Credential ID for private GitHub repositories"
    )]
    credential_id: Option<String>,

    #[arg(long, value_name = "UUID", help = "Organization ID to publish under")]
    org_id: Option<String>,
  },

  #[command(
    visible_alias = "rp",
    aliases = ["u", "update"],
    about = "Refresh this stack's registry entry from its GitHub repository"
  )]
  Republish {
    #[arg(help = "GitHub repository URL (e.g. https://github.com/owner/repo)")]
    stack_url: String,

    #[arg(long, value_name = "VISIBILITY", help = "Visibility: public, private")]
    visibility: Option<String>,

    #[arg(
      long,
      value_name = "UUID",
      help = "Credential ID for private GitHub repositories"
    )]
    credential_id: Option<String>,

    #[arg(long, value_name = "UUID", help = "Organization ID")]
    org_id: Option<String>,
  },
}

#[derive(Subcommand)]
pub enum Commands {
  #[command(visible_alias = "n", about = "Create a new project from a template")]
  New {
    #[arg(help = "Name of the project directory to create")]
    name: String,

    #[arg(help = "Name of the template to use (omit to pick interactively)")]
    template_name: Option<String>,

    #[arg(
      long,
      value_name = "PATH",
      help = "Scaffold from a stack manifest (anesis.stack.json): template + ordered addons"
    )]
    stack: Option<String>,

    #[arg(short, long, help = "Pick only from already-downloaded templates")]
    installed: bool,

    #[arg(short, long, help = "Accept all defaults, skip confirmation prompts")]
    yes: bool,

    #[arg(
      long,
      help = "Allow overwriting existing files in the destination directory"
    )]
    overwrite: bool,

    #[arg(
      long,
      value_name = "NAME=VALUE",
      help = "Provide an input value non-interactively (repeatable)"
    )]
    input: Vec<String>,

    #[arg(long, help = "Show what would be generated without writing any files")]
    dry_run: bool,
  },

  #[command(visible_alias = "t", about = "Manage templates")]
  Template {
    #[command(subcommand)]
    command: TemplateCommands,
  },

  #[command(visible_alias = "in", about = "Log in to your Anesis account")]
  Login,

  #[command(visible_alias = "out", about = "Log out of your Anesis account")]
  Logout,

  #[command(about = "Show information about the currently logged-in account")]
  Account {
    #[arg(long, help = "Output as JSON")]
    json: bool,
  },

  #[command(visible_alias = "a", about = "Manage addons")]
  Addon {
    #[command(subcommand)]
    command: AddonCommands,
  },

  #[command(
    visible_alias = "s",
    about = "Manage stacks: template + ordered addons"
  )]
  Stack {
    #[command(subcommand)]
    command: StackCommands,
  },

  #[command(about = "Run an addon command in the current project")]
  Use {
    #[arg(help = "Addon id (omit to pick interactively)")]
    addon_id: Option<String>,

    #[arg(help = "Command to run (omit to list the addon's commands)")]
    command: Option<String>,

    #[arg(short, long, help = "Pick only from already-downloaded addons")]
    installed: bool,

    #[arg(short, long, help = "Accept all defaults, skip confirmation prompts")]
    yes: bool,

    #[arg(
      long,
      value_name = "NAME=VALUE",
      help = "Provide an input value non-interactively (repeatable)"
    )]
    input: Vec<String>,

    #[arg(
      long,
      help = "Show the plan (variant, inputs, steps) without changing any files"
    )]
    dry_run: bool,

    #[arg(
      long,
      conflicts_with = "dry_run",
      help = "Run the command against a scratch copy of the project and show a diff, leaving the original untouched"
    )]
    diff: bool,
  },

  #[command(about = "Revert an applied addon's changes in the current project")]
  Undo {
    #[arg(help = "Addon id to revert")]
    addon_id: String,

    #[arg(short = 'y', long, help = "Skip the confirmation prompt")]
    yes: bool,
  },

  #[command(about = "List applied addons that have a newer version in the registry")]
  Outdated {
    #[arg(long, help = "Output as JSON")]
    json: bool,
  },

  #[command(
    about = "Upgrade an applied addon in this project to the registry's latest version",
    long_about = "Upgrade an addon already applied to this project to the registry's \
                  latest version.\n\n\
                  Not to be confused with:\n  \
                  anesis upgrade                 — replace the anesis binary itself\n  \
                  anesis addon republish <URL>   — refresh an addon's registry entry"
  )]
  Update {
    #[arg(help = "Addon id to update (omit with --all to update every outdated addon)")]
    addon_id: Option<String>,

    #[arg(
      short = 'y',
      long,
      help = "Accept all defaults, skip confirmation prompts"
    )]
    yes: bool,

    #[arg(long, help = "Update every outdated addon in this project")]
    all: bool,
  },

  #[command(
    visible_alias = "self-update",
    about = "Download and install the latest Anesis release (updates the CLI itself)"
  )]
  Upgrade,

  #[command(
    about = "Run an MCP (Model Context Protocol) stdio server so AI agents can drive Anesis"
  )]
  Mcp,

  #[command(
    hide = true,
    about = "Write roff man pages for anesis into a directory"
  )]
  Man {
    #[arg(help = "Directory to write the man pages into")]
    dir: String,
  },

  #[command(about = "Install shell tab completion for anesis")]
  Completions {
    #[arg(value_enum, help = "Shell to install completions for")]
    shell: CompletionShell,

    #[arg(
      long,
      help = "Write the script to stdout instead of installing it",
      long_help = "Write the script to stdout instead of installing it.\n\n\
                   For packagers: a Homebrew formula or distro package installs \
                   completions into its own prefix and must not write into the \
                   building user's dotfiles."
    )]
    print: bool,
  },

  #[command(about = "Interactively search the registry for templates and addons")]
  Search {
    #[arg(help = "Pre-fill the filter (optional; you can also type in the picker)")]
    query: Option<String>,

    #[arg(
      long,
      help = "Output matching results as JSON instead of opening the picker"
    )]
    json: bool,
  },

  #[command(about = "Show CLI version, data paths and login status")]
  Info {
    #[arg(long, help = "Output as JSON")]
    json: bool,
  },

  #[command(about = "Show this project's template and applied addons")]
  Status {
    #[arg(long, help = "Output as JSON")]
    json: bool,
  },

  #[command(about = "Diagnose common environment/project problems")]
  Doctor {
    #[arg(long, help = "Output as JSON")]
    json: bool,
  },

  #[command(about = "Show which addon command created or modified a file")]
  Why {
    #[arg(help = "File path to look up (relative to the project root); omit to list all")]
    path: Option<String>,

    #[arg(long, help = "Output as JSON")]
    json: bool,
  },
}

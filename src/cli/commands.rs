use clap::{Subcommand, arg};

#[derive(Subcommand)]
pub enum AddonCommands {
  #[command(about = "Install and cache an addon")]
  Install { addon_id: String },

  #[command(about = "List installed addons")]
  List,

  #[command(about = "Remove a cached addon")]
  Remove { addon_id: String },
}

#[derive(Subcommand)]
pub enum TemplateCommands {
  #[command(about = "Download and cache a template locally")]
  Install {
    #[arg(help = "Name of the template to install")]
    template_name: String,
  },

  #[command(about = "List all locally installed templates")]
  List,

  #[command(about = "Remove an installed template from the local cache")]
  Remove {
    #[arg(help = "Name of the template to remove")]
    template_name: String,
  },

  #[command(about = "Publish a GitHub repository as an Oxide template")]
  Publish {
    #[arg(help = "GitHub repository URL (e.g. https://github.com/owner/repo)")]
    template_url: String,
  },
}

#[derive(Subcommand)]
pub enum Commands {
  #[command(alias = "n", about = "Create a new project from a template")]
  New {
    #[arg(help = "Name of the project directory to create")]
    name: String,

    #[arg(help = "Name of the template to use (e.g. react-vite-ts)")]
    template_name: String,
  },

  #[command(alias = "t", about = "Manage templates")]
  Template {
    #[command(subcommand)]
    command: TemplateCommands,
  },

  #[command(alias = "in", about = "Log in to your Oxide account")]
  Login,

  #[command(alias = "out", about = "Log out of your Oxide account")]
  Logout,

  #[command(
    alias = "a",
    about = "Show information about the currently logged-in account"
  )]
  Account,

  #[command(about = "Manage addons")]
  Addon {
    #[command(subcommand)]
    command: AddonCommands,
  },

  #[command(external_subcommand)]
  External(Vec<String>),
}

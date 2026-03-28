use clap::{Subcommand, arg};

#[derive(Subcommand)]
pub enum Commands {
  #[command(alias = "n", about = "Create a new project from a template")]
  New {
    #[arg(help = "Name of the project directory to create")]
    name: String,

    #[arg(help = "Name of the template to use (e.g. react-vite-ts)")]
    template_name: String,
  },

  #[command(alias = "it", about = "Download and cache a template locally")]
  InstallTemplate {
    #[arg(help = "Name of the template to install")]
    template_name: String,
  },

  #[command(
    alias = "d",
    about = "Remove an installed template from the local cache"
  )]
  Delete {
    #[arg(help = "Name of the template to remove")]
    template_name: String,
  },

  #[command(alias = "ied", about = "List all locally installed templates")]
  Installed,

  #[command(alias = "in", about = "Log in to your Oxide account")]
  Login,

  #[command(alias = "out", about = "Log out of your Oxide account")]
  Logout,

  #[command(
    alias = "a",
    about = "Show information about the currently logged-in account"
  )]
  Account,

  #[command(
    alias = "pt",
    about = "Publish a GitHub repository as an Oxide template"
  )]
  PublishTemplate {
    #[arg(help = "GitHub repository URL (e.g. https://github.com/owner/repo)")]
    template_url: String,
  },
}

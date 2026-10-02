use clap::{CommandFactory, Parser};

pub mod commands;
pub mod dispatch;
use commands::Commands;

#[derive(Parser)]
#[command(
  name = "anesis",
  bin_name = "anesis",
  version,
  about = "Scaffold projects from remote templates and extend them with addons",
  long_about = "Anesis scaffolds a project from a template, then layers reusable addons \
                (auth, ORM, Docker, CI, …) on top of it — each one applied as a reversible \
                set of steps.\n\n\
                Start with `anesis new <name>` to pick a template interactively, `anesis use` \
                to apply an addon, and `anesis status` to see what a project already has.\n\n\
                Registry: https://anesis-dev.vercel.app",
  arg_required_else_help = true
)]
pub struct Cli {
  #[command(subcommand)]
  pub command: Commands,

  #[arg(
    long,
    global = true,
    help = "Do not report install counts to the registry",
    long_help = "Do not report install counts to the registry.\n\n\
                 By default `anesis new` and every addon command send a single \
                 `POST /{resource}/use` so the registry can show download counts. \
                 It carries your account token, the resource name and a timestamp — \
                 no project contents and no file paths. Setting ANESIS_NO_TELEMETRY \
                 has the same effect."
  )]
  pub no_telemetry: bool,

  #[arg(
    long,
    global = true,
    help = "Allow addon `run` steps to execute without confirmation",
    long_help = "Allow addon `run` steps to execute without confirmation.\n\n\
                 A `run` step is an arbitrary shell command that comes from a \
                 remote registry. Interactively you are asked before each one. \
                 With --yes, --stack, or under `anesis mcp` there is nobody to \
                 ask, so run steps are refused unless you pass this flag (or set \
                 ANESIS_ALLOW_RUN). See SECURITY.md."
  )]
  pub allow_run: bool,

  #[arg(
    short,
    long,
    global = true,
    conflicts_with = "quiet",
    help = "Print debug detail about what anesis is doing",
    long_help = "Print debug detail about what anesis is doing.\n\n\
                 Equivalent to RUST_LOG=debug for anesis' own logging. Repeat \
                 (-vv) for trace level. Diagnostics go to stderr, so `--json` \
                 output on stdout stays a single clean document.",
    action = clap::ArgAction::Count
  )]
  pub verbose: u8,

  #[arg(
    short,
    long,
    global = true,
    help = "Suppress progress spinners and the upgrade notice",
    long_help = "Suppress progress spinners and the upgrade notice.\n\n\
                 Command results and errors are still printed and the exit code \
                 is unchanged, so this is safe in scripts."
  )]
  pub quiet: bool,

  #[arg(
    long,
    global = true,
    help = "Disable coloured output",
    long_help = "Disable coloured output.\n\n\
                 Colour is also disabled automatically when stdout is not a \
                 terminal, or when NO_COLOR is set."
  )]
  pub no_color: bool,

  #[arg(
    long,
    global = true,
    help = "Use ASCII glyphs instead of Unicode symbols",
    long_help = "Use ASCII glyphs instead of Unicode symbols.\n\n\
                 Useful for cmd.exe, non-UTF-8 locales, and CI logs. \
                 ANESIS_ASCII has the same effect."
  )]
  pub ascii: bool,
}

pub fn parse() -> Cli {
  Cli::parse()
}

pub fn command() -> clap::Command {
  <Cli as CommandFactory>::command()
}

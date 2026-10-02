pub mod caps;
pub mod progress;
pub mod symbols;
pub mod tree;

use std::sync::atomic::{AtomicBool, Ordering};

use colored::Colorize;
use comfy_table::{ContentArrangement, Table, presets::UTF8_FULL_CONDENSED};

pub use progress::{StepProgress, download_bar, spinner};

pub fn catalog_table() -> Table {
  let mut table = Table::new();
  table
    .load_preset(UTF8_FULL_CONDENSED)
    .set_content_arrangement(ContentArrangement::Dynamic);
  table
}

pub fn truncate(s: &str, max: usize) -> String {
  if s.chars().count() <= max {
    return s.to_string();
  }
  let short: String = s.chars().take(max.saturating_sub(1)).collect();
  format!("{}{}", short.trim_end(), symbols::ellipsis())
}

static QUIET: AtomicBool = AtomicBool::new(false);

pub fn set_quiet(quiet: bool) {
  QUIET.store(quiet, Ordering::Relaxed);
}

pub fn is_quiet() -> bool {
  QUIET.load(Ordering::Relaxed)
}

pub fn init(ascii: bool) {
  caps::init(ascii);
}

pub fn success(msg: impl AsRef<str>) {
  println!("{} {}", symbols::ok().green(), msg.as_ref());
}

pub fn warn(msg: impl AsRef<str>) {
  println!("{} {}", symbols::warn().yellow(), msg.as_ref());
}

pub fn warn_err(msg: impl AsRef<str>) {
  eprintln!("{} {}", symbols::warn().yellow().bold(), msg.as_ref());
}

pub fn failure(msg: impl AsRef<str>) {
  eprintln!("{} {}", symbols::err().red().bold(), msg.as_ref());
}

pub fn error_header(msg: impl std::fmt::Display) {
  eprintln!("{} {}", "error:".red().bold(), msg);
}

pub fn hint_line(msg: impl AsRef<str>) {
  eprintln!("  {} {}", "hint:".cyan().bold(), msg.as_ref());
}

pub fn labeled_line(label: &str, text: impl AsRef<str>) {
  eprintln!("  {} {}", format!("{label}:").cyan().bold(), text.as_ref());
}

pub fn note(msg: impl AsRef<str>) {
  println!("{}", msg.as_ref().dimmed());
}

pub fn kv(label: &str, value: impl AsRef<str>) {
  println!("{} {}", format!("{label}:").dimmed(), value.as_ref().cyan());
}

pub fn kv_padded(label: &str, value: impl AsRef<str>, width: usize) {
  let padded = format!("{:<width$}", format!("{label}:"));
  println!("  {} {}", padded.dimmed(), value.as_ref());
}

pub fn section(title: impl AsRef<str>) {
  println!("{}", format!("{}:", title.as_ref()).bold());
}

pub fn hint(label: &str, cmd: impl AsRef<str>) {
  let padded = format!("{:<10}", format!("{label}:"));
  println!("  {}{}", padded.dimmed(), cmd.as_ref().cyan());
}

pub fn kind_tag(kind: &str) -> String {
  match kind {
    "template" => kind.green().bold().to_string(),
    "addon" => kind.magenta().bold().to_string(),
    "stack" => kind.blue().bold().to_string(),
    _ => kind.bold().to_string(),
  }
}

pub fn accent(text: impl AsRef<str>) -> String {
  text.as_ref().cyan().to_string()
}

pub fn good(text: impl AsRef<str>) -> String {
  text.as_ref().green().to_string()
}

pub fn muted(text: impl AsRef<str>) -> String {
  text.as_ref().dimmed().to_string()
}

pub fn bold(text: impl AsRef<str>) -> String {
  text.as_ref().bold().to_string()
}

pub fn magenta(text: impl AsRef<str>) -> String {
  text.as_ref().magenta().to_string()
}

pub fn magenta_bold(text: impl AsRef<str>) -> String {
  text.as_ref().magenta().bold().to_string()
}

pub fn yellow(text: impl AsRef<str>) -> String {
  text.as_ref().yellow().to_string()
}

pub fn red(text: impl AsRef<str>) -> String {
  text.as_ref().red().to_string()
}

pub fn blue(text: impl AsRef<str>) -> String {
  text.as_ref().blue().to_string()
}

pub fn accent_bold(text: impl AsRef<str>) -> String {
  text.as_ref().cyan().bold().to_string()
}

pub fn step(idx: usize, total: usize, label: impl AsRef<str>) {
  println!(
    "{} {}",
    format!("[{}/{total}]", idx + 1).dimmed(),
    label.as_ref()
  );
}

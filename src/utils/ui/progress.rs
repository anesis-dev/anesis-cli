use std::time::{Duration, Instant};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use super::is_quiet;
use super::symbols;

pub fn spinner(msg: impl Into<String>) -> ProgressBar {
  if is_quiet() {
    return ProgressBar::hidden();
  }

  let pb = ProgressBar::new_spinner();
  pb.set_style(
    ProgressStyle::with_template("{spinner:.cyan} {msg}")
      .unwrap()
      .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
  );
  pb.set_message(msg.into());
  pb.enable_steady_tick(Duration::from_millis(80));
  pb
}

pub fn download_bar(total: Option<u64>) -> ProgressBar {
  if is_quiet() {
    return ProgressBar::hidden();
  }

  match total {
    Some(len) => {
      let pb = ProgressBar::new(len);
      apply_download_style(&pb);
      pb
    }
    None => spinner("Downloading..."),
  }
}

pub fn apply_download_style(pb: &ProgressBar) {
  pb.set_style(
    ProgressStyle::with_template("{bar:30.cyan/blue} {bytes}/{total_bytes} {bytes_per_sec} {eta}")
      .unwrap()
      .progress_chars("=> "),
  );
}

/// Animated per-step spinners in a TTY (`✓ label 0.4s` on completion); under
/// `--quiet` or a non-TTY (piped output, CI logs) it degrades to the plain
/// `[i/n] label` line printed once up front, matching the pre-stage-3 output
/// exactly so scripts scraping that format don't see a change.
pub struct StepProgress {
  multi: Option<MultiProgress>,
}

impl StepProgress {
  pub fn new() -> Self {
    Self {
      multi: if is_quiet() || !std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        None
      } else {
        Some(MultiProgress::new())
      },
    }
  }

  pub fn start_step(&self, idx: usize, total: usize, label: &str) -> StepHandle {
    let prefix = format!("[{}/{total}]", idx + 1);
    match &self.multi {
      Some(multi) => {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
          ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.set_message(format!("{prefix} {label}"));
        pb.enable_steady_tick(Duration::from_millis(80));
        let pb = multi.add(pb);
        StepHandle {
          pb: Some(pb),
          prefix,
          label: label.to_string(),
          start: Instant::now(),
        }
      }
      None => {
        println!("{prefix} {label}");
        StepHandle {
          pb: None,
          prefix,
          label: label.to_string(),
          start: Instant::now(),
        }
      }
    }
  }
}

impl Default for StepProgress {
  fn default() -> Self {
    Self::new()
  }
}

pub struct StepHandle {
  pb: Option<ProgressBar>,
  prefix: String,
  label: String,
  start: Instant,
}

impl StepHandle {
  pub fn success(self) {
    let Some(pb) = self.pb else {
      return;
    };
    let elapsed = self.start.elapsed();
    pb.finish_with_message(format!(
      "{} {} {} {:.1}s",
      symbols::ok(),
      self.prefix,
      self.label,
      elapsed.as_secs_f64()
    ));
  }

  pub fn failure(self) {
    let Some(pb) = self.pb else {
      return;
    };
    pb.finish_with_message(format!("{} {} {}", symbols::err(), self.prefix, self.label));
  }
}

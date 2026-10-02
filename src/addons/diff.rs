use std::{path::Path, process::Command};

pub fn show_diff(baseline: &Path, work: &Path) {
  let Ok(diff) = which::which("diff") else {
    eprintln!(
      "('diff' was not found on PATH; cannot show changes. \
       On Windows, install Git for Windows or run this inside WSL.)"
    );
    eprintln!("  baseline: {}", baseline.display());
    eprintln!("  after:    {}", work.display());
    return;
  };

  match Command::new(diff)
    .arg("-ruN")
    .arg("-x")
    .arg("anesis.lock")
    .arg(baseline)
    .arg(work)
    .output()
  {
    Ok(out) => {
      let text = String::from_utf8_lossy(&out.stdout);
      if text.trim().is_empty() {
        println!("(the addon made no changes)");
      } else {
        print!("{text}");
      }
    }
    Err(_) => eprintln!("(system 'diff' not available; cannot show changes)"),
  }
}

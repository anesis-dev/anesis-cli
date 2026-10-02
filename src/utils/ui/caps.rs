use std::sync::OnceLock;

#[derive(Clone, Copy)]
pub struct Caps {
  pub color: bool,
  pub unicode: bool,
}

static CAPS: OnceLock<Caps> = OnceLock::new();

pub fn init(ascii: bool) {
  let _ = CAPS.get_or_init(|| detect(ascii));
}

pub fn caps() -> Caps {
  *CAPS.get_or_init(|| detect(false))
}

fn detect(ascii: bool) -> Caps {
  Caps {
    color: colored::control::SHOULD_COLORIZE.should_colorize(),
    unicode: detect_unicode(ascii),
  }
}

fn detect_unicode(ascii: bool) -> bool {
  if ascii || std::env::var_os("ANESIS_ASCII").is_some() {
    return false;
  }

  if cfg!(windows) {
    return std::env::var_os("WT_SESSION").is_some() || std::env::var_os("TERM_PROGRAM").is_some();
  }

  for var in ["LC_ALL", "LC_CTYPE", "LANG"] {
    if let Some(value) = std::env::var_os(var) {
      return value.to_string_lossy().to_uppercase().contains("UTF-8");
    }
  }
  true
}

pub fn detect_unicode_for_tests(ascii: bool) -> bool {
  detect_unicode(ascii)
}

pub fn caps_for_tests(ascii: bool) -> Caps {
  detect(ascii)
}

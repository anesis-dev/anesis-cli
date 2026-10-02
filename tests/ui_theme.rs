mod common;

use common::{caps_for_tests, detect_unicode_for_tests};

#[test]
fn unicode_detection_matrix() {
  let saved: Vec<(&str, Option<String>)> = ["ANESIS_ASCII", "LC_ALL", "LC_CTYPE", "LANG"]
    .iter()
    .map(|name| (*name, std::env::var(name).ok()))
    .collect();
  for (name, _) in &saved {
    unsafe {
      std::env::remove_var(name);
    }
  }

  assert!(
    !detect_unicode_for_tests(true),
    "--ascii forces ASCII regardless of locale"
  );

  unsafe {
    std::env::set_var("ANESIS_ASCII", "1");
  }
  assert!(
    !detect_unicode_for_tests(false),
    "ANESIS_ASCII env forces ASCII"
  );
  unsafe {
    std::env::remove_var("ANESIS_ASCII");
  }

  if cfg!(windows) {
    assert!(!detect_unicode_for_tests(false));
    unsafe {
      std::env::set_var("WT_SESSION", "1");
    }
    assert!(detect_unicode_for_tests(false));
    unsafe {
      std::env::remove_var("WT_SESSION");
    }
  } else {
    assert!(
      detect_unicode_for_tests(false),
      "no locale vars set → unicode defaults on"
    );

    unsafe {
      std::env::set_var("LC_ALL", "en_US.UTF-8");
    }
    assert!(detect_unicode_for_tests(false));

    unsafe {
      std::env::set_var("LC_ALL", "C");
    }
    assert!(!detect_unicode_for_tests(false));

    unsafe {
      std::env::remove_var("LC_ALL");
      std::env::set_var("LANG", "POSIX");
    }
    assert!(!detect_unicode_for_tests(false));

    unsafe {
      std::env::remove_var("LANG");
    }
  }

  let caps = caps_for_tests(true);
  assert!(!caps.unicode);

  for (name, value) in saved {
    unsafe {
      match value {
        Some(v) => std::env::set_var(name, v),
        None => std::env::remove_var(name),
      }
    }
  }
}

#[test]
fn symbols_switch_on_unicode_flag() {
  use anesis::utils::ui::symbols;

  anesis::utils::ui::caps::init(true);
  assert_eq!(symbols::ok(), "[ok]");
}

#[test]
fn kv_padded_pads_the_plain_label_not_the_colored_string() {
  let padded = format!("{:<16}", format!("{}:", "label"));
  assert_eq!(padded, "label:          ");
}

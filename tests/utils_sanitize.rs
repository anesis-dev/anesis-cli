use anesis::utils::sanitize::sanitize_for_display;

#[test]
fn passes_through_plain_text_unchanged() {
  assert_eq!(
    sanitize_for_display("npm install left-pad"),
    "npm install left-pad"
  );
}

#[test]
fn strips_carriage_return_and_escape_bytes() {
  let input = "safe text\r\x1b[2Kmalicious redraw";
  let out = sanitize_for_display(input);
  assert!(!out.contains('\r'));
  assert!(!out.contains('\u{1b}'));
  assert_eq!(out, "safe text[2Kmalicious redraw");
}

#[test]
fn strips_c1_control_characters() {
  let input = format!("before{}after", '\u{85}');
  let out = sanitize_for_display(&input);
  assert_eq!(out, "beforeafter");
}

#[test]
fn strips_del() {
  let input = format!("before{}after", '\u{7f}');
  assert_eq!(sanitize_for_display(&input), "beforeafter");
}

#[test]
fn truncates_overly_long_input() {
  let input = "a".repeat(500);
  let out = sanitize_for_display(&input);
  assert!(out.chars().count() <= 201);
  assert!(out.ends_with('…'));
}

#[test]
fn keeps_short_input_untouched_in_length() {
  let input = "a".repeat(50);
  let out = sanitize_for_display(&input);
  assert_eq!(out, input);
}

const MAX_DISPLAY_LEN: usize = 200;

pub fn sanitize_for_display(input: &str) -> String {
  let mut out: String = input.chars().filter(|c| !is_control_or_del(*c)).collect();

  if out.chars().count() > MAX_DISPLAY_LEN {
    out = out.chars().take(MAX_DISPLAY_LEN).collect();
    out.push('…');
  }

  out
}

fn is_control_or_del(c: char) -> bool {
  let cp = c as u32;
  cp < 0x20 || cp == 0x7f || (0x80..=0x9f).contains(&cp)
}

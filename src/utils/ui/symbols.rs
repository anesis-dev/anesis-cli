use super::caps::caps;

pub fn ok() -> &'static str {
  if caps().unicode { "✓" } else { "[ok]" }
}

pub fn warn() -> &'static str {
  if caps().unicode { "⚠" } else { "[!]" }
}

pub fn err() -> &'static str {
  if caps().unicode { "✗" } else { "[x]" }
}

pub fn info() -> &'static str {
  if caps().unicode { "ℹ" } else { "[i]" }
}

pub fn arrow() -> &'static str {
  if caps().unicode { "→" } else { "->" }
}

pub fn bullet() -> &'static str {
  if caps().unicode { "•" } else { "*" }
}

pub fn chevron() -> &'static str {
  if caps().unicode { "❯" } else { ">" }
}

pub fn ellipsis() -> &'static str {
  if caps().unicode { "…" } else { "..." }
}

pub fn tree_branch() -> &'static str {
  if caps().unicode { "├─ " } else { "|- " }
}

pub fn tree_last() -> &'static str {
  if caps().unicode { "└─ " } else { "`- " }
}

pub fn tree_vert() -> &'static str {
  if caps().unicode { "│  " } else { "|  " }
}

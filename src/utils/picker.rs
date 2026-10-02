use std::io::{IsTerminal, Write, stderr};

use anyhow::{Context, Result};
use crossterm::{
  cursor::{Hide, MoveTo, Show},
  event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, read},
  execute, queue,
  terminal::{
    Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
    enable_raw_mode, size,
  },
};

use crate::utils::errors::AnesisError;
use crate::utils::ui::{self, symbols, truncate};

#[derive(Clone, Copy, PartialEq)]
pub enum ItemKind {
  Template,
  Addon,
  Stack,
}

impl ItemKind {
  fn tag(self) -> &'static str {
    match self {
      ItemKind::Template => "template",
      ItemKind::Addon => "addon",
      ItemKind::Stack => "stack",
    }
  }
}

pub struct PickItem {
  pub kind: ItemKind,
  pub id: String,
  pub name: String,
  pub meta: String,
  pub description: String,
  pub haystack: String,
}

pub fn search_results_json(items: &[PickItem], query: Option<&str>) -> Vec<serde_json::Value> {
  let needle = query.unwrap_or_default().to_lowercase();
  items
    .iter()
    .filter(|i| needle.is_empty() || i.haystack.contains(&needle))
    .map(|i| {
      serde_json::json!({
        "kind": i.kind.tag(),
        "id": i.id,
        "name": i.name,
        "description": i.description,
      })
    })
    .collect()
}

fn tokenize(query: &str) -> Vec<&str> {
  query
    .split(|c: char| !c.is_alphanumeric())
    .filter(|tok| !tok.is_empty())
    .collect()
}

fn smart_match(haystack: &str, tokens: &[&str]) -> bool {
  tokens.iter().all(|tok| haystack.contains(tok))
}

fn match_score(name: &str, query: &str, tokens: &[&str]) -> i32 {
  let mut score = 0;
  if name == query {
    score += 1000;
  } else if name.starts_with(query) {
    score += 500;
  } else if name.contains(query) {
    score += 200;
  }
  score += tokens.iter().filter(|tok| name.contains(*tok)).count() as i32 * 20;
  score
}

struct TerminalGuard;

impl TerminalGuard {
  fn enter() -> Result<Self> {
    enable_raw_mode().context("Failed to enter raw mode")?;
    let _ = execute!(stderr(), EnterAlternateScreen, Hide);
    Ok(Self)
  }
}

impl Drop for TerminalGuard {
  fn drop(&mut self) {
    let _ = execute!(stderr(), Show, LeaveAlternateScreen);
    let _ = disable_raw_mode();
  }
}

pub fn restore_terminal() {
  let _ = execute!(stderr(), Show, LeaveAlternateScreen);
  let _ = disable_raw_mode();
}

pub fn pick(
  items: &[PickItem],
  title: &str,
  show_kind: bool,
  initial_filter: &str,
) -> Result<Option<usize>> {
  if !stderr().is_terminal() {
    return Err(AnesisError::NotATerminal("Choosing from a list".to_string()).into());
  }

  let _guard = TerminalGuard::enter()?;
  let mut out = stderr();
  picker_loop(&mut out, items, title, show_kind, initial_filter)
}

pub async fn pick_one(
  items: Vec<PickItem>,
  title: impl Into<String>,
  show_kind: bool,
  seed: String,
) -> Result<Option<(ItemKind, String, String)>> {
  let title = title.into();
  tokio::task::spawn_blocking(move || {
    pick(&items, &title, show_kind, &seed)
      .map(|sel| sel.map(|i| (items[i].kind, items[i].id.clone(), items[i].name.clone())))
  })
  .await
  .context("Picker task failed")?
}

fn picker_loop(
  out: &mut impl Write,
  items: &[PickItem],
  title: &str,
  show_kind: bool,
  initial_filter: &str,
) -> Result<Option<usize>> {
  let mut filter = initial_filter.to_string();
  let mut selected = 0usize;

  loop {
    let (cols, rows) = size().unwrap_or((80, 24));
    let width = cols as usize;

    let needle = filter.to_lowercase();
    let tokens = tokenize(&needle);
    let mut matches: Vec<usize> = items
      .iter()
      .enumerate()
      .filter(|(_, it)| smart_match(&it.haystack, &tokens))
      .map(|(i, _)| i)
      .collect();
    matches.sort_by_key(|&i| -match_score(&items[i].name.to_lowercase(), &needle, &tokens));
    selected = selected.min(matches.len().saturating_sub(1));

    let per_page = ((rows as usize).saturating_sub(4) / 3).max(1);
    let offset = if selected >= per_page {
      selected - per_page + 1
    } else {
      0
    };

    let mut lines: Vec<String> = Vec::with_capacity(per_page * 3 + 3);
    lines.push(ui::bold(title));
    lines.push(format!("{} {}", ui::muted(symbols::chevron()), filter));
    lines.push(String::new());
    for (row_idx, &item_idx) in matches.iter().enumerate().skip(offset).take(per_page) {
      let it = &items[item_idx];
      let tag = if show_kind {
        format!("[{}] ", it.kind.tag())
      } else {
        String::new()
      };
      let body = truncate(
        if it.description.is_empty() {
          &it.name
        } else {
          &it.description
        },
        width.saturating_sub(4),
      );
      if row_idx == selected {
        lines.push(format!(
          "{} {}{}{}",
          ui::accent_bold(symbols::chevron()),
          ui::accent(&tag),
          ui::accent_bold(&it.name),
          ui::accent(&it.meta)
        ));
        lines.push(format!("    {}", ui::accent(&body)));
      } else {
        let tag = match it.kind {
          ItemKind::Template => ui::good(&tag),
          ItemKind::Addon => ui::magenta(&tag),
          ItemKind::Stack => ui::blue(&tag),
        };
        lines.push(format!(
          "  {}{}{}",
          tag,
          ui::bold(&it.name),
          ui::muted(&it.meta)
        ));
        lines.push(format!("    {}", ui::muted(&body)));
      }
      lines.push(String::new());
    }
    if matches.is_empty() {
      lines.push(ui::muted("  no matches"));
    }
    lines.push(ui::muted(
      "↑↓ move · type to filter · enter select · esc cancel",
    ));

    queue!(out, Clear(ClearType::All))?;
    for (row, line) in lines.iter().enumerate() {
      if row as u16 >= rows {
        break;
      }
      queue!(out, MoveTo(0, row as u16))?;
      out.write_all(line.as_bytes())?;
    }
    out.flush()?;

    let Event::Key(KeyEvent {
      code,
      modifiers,
      kind: KeyEventKind::Press,
      ..
    }) = read()?
    else {
      continue;
    };

    match code {
      KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return Ok(None),
      KeyCode::Esc => return Ok(None),
      KeyCode::Enter => match matches.get(selected) {
        Some(&i) => return Ok(Some(i)),
        None => continue,
      },
      KeyCode::Up => selected = selected.saturating_sub(1),
      KeyCode::Down => {
        if selected + 1 < matches.len() {
          selected += 1;
        }
      }
      KeyCode::Backspace => {
        filter.pop();
        selected = 0;
      }
      KeyCode::Char(c) => {
        filter.push(c);
        selected = 0;
      }
      _ => {}
    }
  }
}

pub fn tokenize_for_tests(query: &str) -> Vec<&str> {
  tokenize(query)
}

pub fn smart_match_for_tests(haystack: &str, tokens: &[&str]) -> bool {
  smart_match(haystack, tokens)
}

pub fn match_score_for_tests(name: &str, query: &str, tokens: &[&str]) -> i32 {
  match_score(name, query, tokens)
}

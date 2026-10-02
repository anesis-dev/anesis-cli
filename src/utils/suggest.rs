const MAX_DISTANCE: usize = 2;

pub fn suggest<'a>(input: &str, candidates: &[&'a str]) -> Option<&'a str> {
  candidates
    .iter()
    .map(|&c| (c, levenshtein(input, c)))
    .filter(|(_, distance)| *distance <= MAX_DISTANCE)
    .min_by_key(|(_, distance)| *distance)
    .map(|(c, _)| c)
}

fn levenshtein(a: &str, b: &str) -> usize {
  let a: Vec<char> = a.chars().collect();
  let b: Vec<char> = b.chars().collect();

  let mut row: Vec<usize> = (0..=b.len()).collect();

  for i in 1..=a.len() {
    let mut prev_diag = row[0];
    row[0] = i;
    for j in 1..=b.len() {
      let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
      let above = row[j];
      row[j] = (row[j] + 1).min(row[j - 1] + 1).min(prev_diag + cost);
      prev_diag = above;
    }
  }

  row[b.len()]
}

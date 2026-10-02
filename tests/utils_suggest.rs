use anesis::utils::suggest::suggest;

#[test]
fn suggests_the_closest_match_within_distance() {
  let candidates = ["install", "update", "outdated"];
  assert_eq!(suggest("instal", &candidates), Some("install"));
}

#[test]
fn suggests_none_when_too_far_from_every_candidate() {
  let candidates = ["install", "update", "outdated"];
  assert_eq!(suggest("zzzzzzzzzz", &candidates), None);
}

#[test]
fn suggests_none_for_an_empty_candidate_list() {
  let candidates: [&str; 0] = [];
  assert_eq!(suggest("install", &candidates), None);
}

#[test]
fn picks_the_nearest_of_multiple_close_candidates() {
  let candidates = ["use", "used", "user"];
  assert_eq!(suggest("use", &candidates), Some("use"));
}

#[test]
fn exact_match_has_zero_distance() {
  let candidates = ["outdated"];
  assert_eq!(suggest("outdated", &candidates), Some("outdated"));
}

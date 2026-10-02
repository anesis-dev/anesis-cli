mod common;

use common::{MAX_TOTAL_PAGES_FOR_TESTS, check_total_pages_for_tests};

#[test]
fn total_pages_within_limit_is_accepted() {
  assert!(check_total_pages_for_tests(1, "catalog").is_ok());
  assert!(check_total_pages_for_tests(MAX_TOTAL_PAGES_FOR_TESTS, "catalog").is_ok());
}

#[test]
fn total_pages_over_limit_is_rejected() {
  let err = check_total_pages_for_tests(MAX_TOTAL_PAGES_FOR_TESTS + 1, "catalog")
    .expect_err("a page count above the limit must be rejected");
  assert!(err.to_string().contains("catalog"));
}

#[test]
fn a_bogus_huge_total_pages_is_rejected() {
  assert!(check_total_pages_for_tests(i64::MAX, "addon catalog").is_err());
}

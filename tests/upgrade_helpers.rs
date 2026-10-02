mod common;

use anesis::upgrade::render_upgrade_notice;
use chrono::{TimeZone, Utc};
use common::{
  asset_filename_for_tests, is_cache_fresh_for_tests, is_newer_version_for_tests,
  normalize_version_tag_for_tests, parse_version_for_tests, parse_version_prerelease_for_tests,
  release_asset_url_for_tests, release_checksums_url_for_tests, upgrade_success_message_for_tests,
  verify_asset_checksum_for_tests,
};

#[test]
fn normalize_version_tag_strips_leading_v() {
  let version = normalize_version_tag_for_tests("v1.2.3").unwrap();
  assert_eq!(version, "1.2.3");
}

#[test]
fn parse_version_rejects_invalid_formats() {
  let error = parse_version_for_tests("1.2").unwrap_err();
  assert!(
    error.to_string().contains("1.2"),
    "the error should name the offending version: {error}"
  );
}

#[test]
fn parse_version_accepts_prerelease_and_build_metadata() {
  assert_eq!(parse_version_for_tests("1.0.0-rc1").unwrap(), (1, 0, 0));
  assert_eq!(parse_version_for_tests("1.2.3-beta.2").unwrap(), (1, 2, 3));
  assert_eq!(parse_version_for_tests("1.2.3+build.5").unwrap(), (1, 2, 3));
  assert_eq!(
    parse_version_prerelease_for_tests("1.0.0-rc1").unwrap(),
    "rc1"
  );
}

#[test]
fn prerelease_versions_order_correctly_against_releases() {
  assert!(is_newer_version_for_tests("1.0.0-rc1", "1.0.0").unwrap());
  assert!(is_newer_version_for_tests("1.0.0-rc1", "1.0.0-rc2").unwrap());
  assert!(!is_newer_version_for_tests("1.0.0", "1.0.0-rc2").unwrap());
  assert!(!is_newer_version_for_tests("1.0.0-rc2", "1.0.0-rc1").unwrap());
}

#[test]
fn normalize_version_tag_accepts_a_prerelease_tag() {
  assert_eq!(
    normalize_version_tag_for_tests("v1.0.0-rc1").unwrap(),
    "1.0.0-rc1"
  );
}

#[test]
fn is_newer_version_compares_numeric_components() {
  assert!(is_newer_version_for_tests("0.7.4", "0.8.0").unwrap());
  assert!(!is_newer_version_for_tests("0.8.0", "0.7.4").unwrap());
}

#[test]
fn asset_filename_uses_zip_for_windows_builds() {
  assert_eq!(
    asset_filename_for_tests("windows-x86_64"),
    "anesis-windows-x86_64.zip"
  );
  assert_eq!(
    asset_filename_for_tests("linux-x86_64"),
    "anesis-linux-x86_64.tar.gz"
  );
}

const DOWNLOAD_BASE_URL: &str = "https://github.com/anesis-dev/anesis-cli/releases/download";

#[test]
fn release_asset_url_uses_expected_github_pattern() {
  let asset_url = release_asset_url_for_tests(DOWNLOAD_BASE_URL, "1.2.3", "linux-x86_64").unwrap();
  assert_eq!(
    asset_url,
    "https://github.com/anesis-dev/anesis-cli/releases/download/v1.2.3/anesis-linux-x86_64.tar.gz"
  );
}

#[test]
fn cache_is_fresh_for_recent_version_checks() {
  let now = Utc.with_ymd_and_hms(2026, 4, 11, 10, 30, 0).unwrap();
  assert!(is_cache_fresh_for_tests(
    "2026-04-11T10:00:00Z",
    "0.8.0",
    now
  ));
}

#[test]
fn cache_is_stale_after_one_hour() {
  let now = Utc.with_ymd_and_hms(2026, 4, 11, 11, 0, 1).unwrap();
  assert!(!is_cache_fresh_for_tests(
    "2026-04-11T10:00:00Z",
    "0.8.0",
    now
  ));
}

#[test]
fn render_upgrade_notice_mentions_upgrade_command() {
  let mut parts = env!("CARGO_PKG_VERSION").split('.');
  let major = parts.next().unwrap();
  let minor = parts.next().unwrap();
  let patch: u64 = parts.next().unwrap().parse().unwrap();
  let latest = format!("{major}.{minor}.{}", patch + 1);
  let notice = render_upgrade_notice(&latest);
  assert!(notice.contains("Run `anesis upgrade` to update."));
  assert!(notice.contains(&format!("v{} → v{}", env!("CARGO_PKG_VERSION"), latest)));
}

#[test]
fn parse_version_succeeds_for_valid_semver() {
  let result = parse_version_for_tests("1.2.3").unwrap();
  assert_eq!(result, (1, 2, 3));
}

#[test]
fn parse_version_succeeds_for_zero_components() {
  let result = parse_version_for_tests("0.0.0").unwrap();
  assert_eq!(result, (0, 0, 0));
}

#[test]
fn parse_version_rejects_too_many_components() {
  assert!(parse_version_for_tests("1.2.3.4").is_err());
}

#[test]
fn parse_version_rejects_non_numeric() {
  assert!(parse_version_for_tests("1.2.beta").is_err());
}

#[test]
fn normalize_version_tag_without_v_prefix_is_unchanged() {
  let version = normalize_version_tag_for_tests("2.5.1").unwrap();
  assert_eq!(version, "2.5.1");
}

#[test]
fn normalize_version_tag_rejects_invalid_semver() {
  assert!(normalize_version_tag_for_tests("v1.2").is_err());
}

#[test]
fn is_newer_version_false_when_equal() {
  assert!(!is_newer_version_for_tests("1.0.0", "1.0.0").unwrap());
}

#[test]
fn is_newer_version_patch_bump() {
  assert!(is_newer_version_for_tests("1.0.0", "1.0.1").unwrap());
}

#[test]
fn is_newer_version_major_bump() {
  assert!(is_newer_version_for_tests("0.9.9", "1.0.0").unwrap());
}

#[test]
fn asset_filename_for_macos_uses_tar_gz() {
  assert_eq!(
    asset_filename_for_tests("macos-aarch64"),
    "anesis-macos-aarch64.tar.gz"
  );
}

#[test]
fn cache_is_stale_for_invalid_date_format() {
  assert!(!is_cache_fresh_for_tests(
    "not-a-date",
    "0.8.0",
    chrono::Utc::now()
  ));
}

#[test]
fn release_checksums_url_points_at_the_tag_assets() {
  assert_eq!(
    release_checksums_url_for_tests(DOWNLOAD_BASE_URL, "1.2.3").unwrap(),
    "https://github.com/anesis-dev/anesis-cli/releases/download/v1.2.3/SHA256SUMS"
  );
}

#[test]
fn release_asset_url_rejects_a_plaintext_override() {
  let asset_err =
    release_asset_url_for_tests("http://evil.example/dl", "1.2.3", "linux-x86_64").unwrap_err();
  let checksums_err =
    release_checksums_url_for_tests("http://evil.example/dl", "1.2.3").unwrap_err();

  assert!(
    asset_err.to_string().contains("https"),
    "unexpected error: {asset_err}"
  );
  assert!(
    checksums_err.to_string().contains("https"),
    "unexpected error: {checksums_err}"
  );
}

const ANESIS_SHA256: &str = "187492ca0dd98db10435ba0a271b88367e3f3f5f0af4715630266837397086e4";

#[test]
fn verify_asset_checksum_accepts_a_matching_sum() {
  let bytes = b"anesis";
  let sums = format!("{ANESIS_SHA256}  anesis-linux-x86_64.tar.gz\n");
  assert!(
    verify_asset_checksum_for_tests(bytes, &sums, "anesis-linux-x86_64.tar.gz").is_ok(),
    "expected the published sum to verify"
  );
}

#[test]
fn verify_asset_checksum_rejects_a_tampered_archive() {
  let sums = format!("{ANESIS_SHA256}  anesis-linux-x86_64.tar.gz\n");
  let error =
    verify_asset_checksum_for_tests(b"tampered", &sums, "anesis-linux-x86_64.tar.gz").unwrap_err();
  assert!(error.to_string().contains("Checksum mismatch"));
}

#[test]
fn verify_asset_checksum_rejects_a_missing_entry() {
  let sums = format!("{ANESIS_SHA256}  anesis-linux-x86_64.tar.gz\n");
  let error =
    verify_asset_checksum_for_tests(b"anesis", &sums, "anesis-windows-x86_64.zip").unwrap_err();
  assert!(error.to_string().contains("No checksum for"));
}

#[test]
fn verify_asset_checksum_picks_the_right_line_from_a_full_sums_file() {
  let sums = format!(
    "0000000000000000000000000000000000000000000000000000000000000000  anesis-macos-aarch64.tar.gz\n\
     {ANESIS_SHA256}  anesis-linux-x86_64.tar.gz\n\
     1111111111111111111111111111111111111111111111111111111111111111  anesis-windows-x86_64.zip\n"
  );
  assert!(verify_asset_checksum_for_tests(b"anesis", &sums, "anesis-linux-x86_64.tar.gz").is_ok());
}

#[test]
fn upgrade_success_message_claims_completion_when_the_swap_already_happened() {
  let message = upgrade_success_message_for_tests("1.2.3", false);
  assert!(message.contains("updated to v1.2.3"));
  assert!(!message.contains("background"));
}

#[test]
fn upgrade_success_message_does_not_overclaim_when_the_swap_is_deferred() {
  let message = upgrade_success_message_for_tests("1.2.3", true);
  assert!(
    message.contains("background"),
    "a deferred Windows swap must not claim the update is already done: {message}"
  );
  assert!(message.contains("v1.2.3"));
}

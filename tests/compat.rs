use anesis::compat::{check_anesis_version, check_schema_version, running_version_for_tests};

#[test]
fn the_supported_schema_version_is_accepted() {
  assert!(check_schema_version("addon", "acme/x", "1").is_ok());
}

#[test]
fn an_older_schema_version_is_accepted() {
  assert!(check_schema_version("stack", "acme/s", "0").is_ok());
}

#[test]
fn a_newer_schema_version_is_refused_with_an_upgrade_hint() {
  let err = check_schema_version("addon", "acme/x", "2").unwrap_err();
  let message = err.to_string();
  assert!(message.contains("acme/x"), "{message}");
  assert!(message.contains("schema version 2"), "{message}");
  assert!(message.contains("anesis upgrade"), "{message}");
}

#[test]
fn a_non_numeric_schema_version_is_refused() {
  let err = check_schema_version("addon", "acme/x", "one").unwrap_err();
  assert!(err.to_string().contains("not a version number"));
}

#[test]
fn whitespace_around_the_schema_version_is_tolerated() {
  assert!(check_schema_version("addon", "acme/x", " 1 ").is_ok());
}

#[test]
fn a_satisfied_anesis_version_range_passes() {
  assert!(check_anesis_version("acme/t", ">=0.9.0").is_ok());
}

#[test]
fn an_unsatisfiable_anesis_version_range_is_refused() {
  let err = check_anesis_version("acme/t", ">=99.0.0").unwrap_err();
  let message = err.to_string();
  assert!(message.contains("acme/t"), "{message}");
  assert!(message.contains(">=99.0.0"), "{message}");
  assert!(message.contains("anesis upgrade"), "{message}");
}

#[test]
fn an_empty_anesis_version_is_ignored() {
  assert!(check_anesis_version("acme/t", "   ").is_ok());
}

#[test]
fn an_unparseable_anesis_version_warns_instead_of_failing() {
  assert!(check_anesis_version("acme/t", "not a range").is_ok());
}

#[test]
fn the_running_version_ignores_prerelease_metadata() {
  let version = running_version_for_tests().unwrap();
  assert!(version.pre.is_empty());
  assert!(version.build.is_empty());
}

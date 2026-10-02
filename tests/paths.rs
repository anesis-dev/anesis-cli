use anesis::paths::AnesisPaths;

#[test]
fn new_returns_ok() {
  assert!(AnesisPaths::new().is_ok());
}

#[test]
fn new_matches_under_when_anesis_home_is_set() {
  let dir = assert_fs::TempDir::new().unwrap();
  unsafe { std::env::set_var("ANESIS_HOME", dir.path()) };

  let from_new = AnesisPaths::new().unwrap();
  let from_under = AnesisPaths::under(dir.path());

  unsafe { std::env::remove_var("ANESIS_HOME") };

  assert_eq!(from_new.home, from_under.home);
  assert_eq!(from_new.version_check, from_under.version_check);
  assert_eq!(from_new.cache, from_under.cache);
  assert_eq!(from_new.templates, from_under.templates);
  assert_eq!(from_new.auth, from_under.auth);
  assert_eq!(from_new.addons, from_under.addons);
  assert_eq!(from_new.addons_index, from_under.addons_index);
  assert_eq!(from_new.stacks, from_under.stacks);
}

#[test]
fn home_contains_anesis_suffix() {
  let paths = AnesisPaths::new().unwrap();
  assert!(
    paths.home.to_string_lossy().ends_with(".anesis"),
    "home should end with .anesis, got: {}",
    paths.home.display()
  );
}

#[test]
fn auth_json_is_under_home() {
  let paths = AnesisPaths::new().unwrap();
  assert!(paths.auth.starts_with(&paths.home));
  assert_eq!(paths.auth.file_name().unwrap(), "auth.json");
}

#[test]
fn version_check_is_under_home() {
  let paths = AnesisPaths::new().unwrap();
  assert!(paths.version_check.starts_with(&paths.home));
  assert_eq!(
    paths.version_check.file_name().unwrap(),
    "version_check.json"
  );
}

#[test]
fn templates_is_under_cache() {
  let paths = AnesisPaths::new().unwrap();
  assert!(paths.templates.starts_with(&paths.cache));
}

#[test]
fn addons_is_under_cache() {
  let paths = AnesisPaths::new().unwrap();
  assert!(paths.addons.starts_with(&paths.cache));
}

#[test]
fn addons_index_is_under_addons() {
  let paths = AnesisPaths::new().unwrap();
  assert!(paths.addons_index.starts_with(&paths.addons));
  assert_eq!(
    paths.addons_index.file_name().unwrap(),
    "anesis-addons.json"
  );
}

#[test]
fn ensure_directories_creates_cache_dir() {
  let dir = assert_fs::TempDir::new().unwrap();
  let paths = AnesisPaths::under(dir.path());

  paths.ensure_directories().unwrap();

  assert!(dir.path().join(".anesis").join("cache").is_dir());
}

#[test]
fn ensure_directories_creates_templates_dir() {
  let dir = assert_fs::TempDir::new().unwrap();
  let paths = AnesisPaths::under(dir.path());

  paths.ensure_directories().unwrap();

  assert!(
    dir
      .path()
      .join(".anesis")
      .join("cache")
      .join("templates")
      .is_dir()
  );
}

#[test]
fn ensure_directories_creates_addons_dir() {
  let dir = assert_fs::TempDir::new().unwrap();
  let paths = AnesisPaths::under(dir.path());

  paths.ensure_directories().unwrap();

  assert!(
    dir
      .path()
      .join(".anesis")
      .join("cache")
      .join("addons")
      .is_dir()
  );
}

#[test]
fn addon_dir_joins_the_addons_directory() {
  let paths = AnesisPaths::new().unwrap();
  assert_eq!(
    paths.addon_dir("drizzle").unwrap(),
    paths.addons.join("drizzle")
  );
}

#[test]
fn addon_dir_rejects_a_traversing_id() {
  let paths = AnesisPaths::new().unwrap();
  assert!(paths.addon_dir("../../etc/passwd").is_err());
}

#[test]
fn addon_dir_rejects_an_absolute_id() {
  let paths = AnesisPaths::new().unwrap();
  assert!(paths.addon_dir("/etc/passwd").is_err());
}

#[test]
fn addon_dir_rejects_an_empty_id() {
  let paths = AnesisPaths::new().unwrap();
  assert!(paths.addon_dir("").is_err());
}

#[test]
fn ensure_directories_is_idempotent() {
  let dir = assert_fs::TempDir::new().unwrap();
  let paths = AnesisPaths::under(dir.path());

  paths.ensure_directories().unwrap();
  paths.ensure_directories().unwrap();

  assert!(dir.path().join(".anesis").join("cache").is_dir());
}

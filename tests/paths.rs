use oxide_cli::paths::OxidePaths;

// ── OxidePaths::new ───────────────────────────────────────────────────────────

#[test]
fn new_returns_ok() {
  assert!(OxidePaths::new().is_ok());
}

#[test]
fn home_contains_oxide_suffix() {
  let paths = OxidePaths::new().unwrap();
  assert!(
    paths.home.to_string_lossy().ends_with(".oxide"),
    "home should end with .oxide, got: {}",
    paths.home.display()
  );
}

#[test]
fn auth_json_is_under_home() {
  let paths = OxidePaths::new().unwrap();
  assert!(paths.auth.starts_with(&paths.home));
  assert_eq!(paths.auth.file_name().unwrap(), "auth.json");
}

#[test]
fn templates_is_under_cache() {
  let paths = OxidePaths::new().unwrap();
  assert!(paths.templates.starts_with(&paths.cache));
}

#[test]
fn addons_is_under_cache() {
  let paths = OxidePaths::new().unwrap();
  assert!(paths.addons.starts_with(&paths.cache));
}

#[test]
fn addons_index_is_under_addons() {
  let paths = OxidePaths::new().unwrap();
  assert!(paths.addons_index.starts_with(&paths.addons));
  assert_eq!(paths.addons_index.file_name().unwrap(), "oxide-addons.json");
}

// ── OxidePaths::ensure_directories ───────────────────────────────────────────

#[test]
fn ensure_directories_creates_cache_dir() {
  let dir = assert_fs::TempDir::new().unwrap();
  let paths = OxidePaths {
    home: dir.path().to_path_buf(),
    config: dir.path().join("config.json"),
    cache: dir.path().join("cache"),
    templates: dir.path().join("cache").join("templates"),
    auth: dir.path().join("auth.json"),
    addons: dir.path().join("cache").join("addons"),
    addons_index: dir.path().join("cache").join("addons").join("oxide-addons.json"),
  };

  paths.ensure_directories().unwrap();

  assert!(dir.path().join("cache").is_dir());
}

#[test]
fn ensure_directories_creates_templates_dir() {
  let dir = assert_fs::TempDir::new().unwrap();
  let paths = OxidePaths {
    home: dir.path().to_path_buf(),
    config: dir.path().join("config.json"),
    cache: dir.path().join("cache"),
    templates: dir.path().join("cache").join("templates"),
    auth: dir.path().join("auth.json"),
    addons: dir.path().join("cache").join("addons"),
    addons_index: dir.path().join("cache").join("addons").join("oxide-addons.json"),
  };

  paths.ensure_directories().unwrap();

  assert!(dir.path().join("cache").join("templates").is_dir());
}

#[test]
fn ensure_directories_creates_addons_dir() {
  let dir = assert_fs::TempDir::new().unwrap();
  let paths = OxidePaths {
    home: dir.path().to_path_buf(),
    config: dir.path().join("config.json"),
    cache: dir.path().join("cache"),
    templates: dir.path().join("cache").join("templates"),
    auth: dir.path().join("auth.json"),
    addons: dir.path().join("cache").join("addons"),
    addons_index: dir.path().join("cache").join("addons").join("oxide-addons.json"),
  };

  paths.ensure_directories().unwrap();

  assert!(dir.path().join("cache").join("addons").is_dir());
}

#[test]
fn ensure_directories_is_idempotent() {
  let dir = assert_fs::TempDir::new().unwrap();
  let paths = OxidePaths {
    home: dir.path().to_path_buf(),
    config: dir.path().join("config.json"),
    cache: dir.path().join("cache"),
    templates: dir.path().join("cache").join("templates"),
    auth: dir.path().join("auth.json"),
    addons: dir.path().join("cache").join("addons"),
    addons_index: dir.path().join("cache").join("addons").join("oxide-addons.json"),
  };

  // Running twice should not fail
  paths.ensure_directories().unwrap();
  paths.ensure_directories().unwrap();

  assert!(dir.path().join("cache").is_dir());
}

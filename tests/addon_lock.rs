use anesis::addons::lock::{CommandRun, LockEntry, LockFile};
use anesis::addons::steps::Rollback;

fn sample_entry(id: &str) -> LockEntry {
  LockEntry::new(id, "1.0.0", "universal")
}

fn entry_with_journal(id: &str, journal: Vec<Rollback>) -> LockEntry {
  let mut entry = sample_entry(id);
  entry.commands.push(CommandRun {
    name: "install".to_string(),
    inputs: Default::default(),
    journal,
  });
  entry
}

#[test]
fn load_returns_default_when_no_file() {
  let dir = assert_fs::TempDir::new().unwrap();
  let lock = LockFile::load(dir.path()).unwrap();
  assert!(lock.addons.is_empty());
}

#[test]
fn save_and_load_roundtrip() {
  let dir = assert_fs::TempDir::new().unwrap();
  let mut lock = LockFile::default();
  let mut entry = sample_entry("drizzle");
  entry.upsert_command("install", Default::default(), vec![]);
  lock.upsert_entry(entry);
  lock.save(dir.path()).unwrap();

  let loaded = LockFile::load(dir.path()).unwrap();
  assert_eq!(loaded.addons.len(), 1);
  assert_eq!(loaded.addons[0].id, "drizzle");
  assert!(loaded.is_command_executed("drizzle", "install"));
}

#[test]
fn is_command_executed_false_when_no_entry() {
  let lock = LockFile::default();
  assert!(!lock.is_command_executed("drizzle", "install"));
}

#[test]
fn addon_version_returns_current_version_when_entry_exists() {
  let mut lock = LockFile::default();
  lock.upsert_entry(sample_entry("drizzle"));
  assert_eq!(lock.addon_version("drizzle"), Some("1.0.0"));
}

#[test]
fn upsert_command_adds_once() {
  let mut entry = sample_entry("drizzle");
  entry.upsert_command("install", Default::default(), vec![]);
  entry.upsert_command("install", Default::default(), vec![]);
  assert_eq!(entry.commands.len(), 1);
  assert_eq!(entry.commands_executed().len(), 1);
}

#[test]
fn upsert_command_replaces_inputs_and_journal_on_rerun_without_touching_other_commands() {
  let mut entry = sample_entry("crud");
  entry.upsert_command(
    "add-entity",
    [("name".to_string(), "User".to_string())].into(),
    vec![Rollback::DeleteCreatedFile {
      path: "User.txt".into(),
    }],
  );
  entry.upsert_command(
    "seed",
    [("name".to_string(), "unrelated".to_string())].into(),
    vec![],
  );
  entry.upsert_command(
    "add-entity",
    [("name".to_string(), "Post".to_string())].into(),
    vec![Rollback::DeleteCreatedFile {
      path: "Post.txt".into(),
    }],
  );

  assert_eq!(entry.commands.len(), 2, "still one entry per command name");
  let add_entity = entry
    .commands
    .iter()
    .find(|c| c.name == "add-entity")
    .unwrap();
  assert_eq!(add_entity.inputs.get("name"), Some(&"Post".to_string()));
  assert_eq!(add_entity.journal.len(), 1);

  let seed = entry.commands.iter().find(|c| c.name == "seed").unwrap();
  assert_eq!(
    seed.inputs.get("name"),
    Some(&"unrelated".to_string()),
    "a same-named input on a different command must not be corrupted by add-entity's rerun"
  );
}

#[test]
fn upsert_entry_adds_new() {
  let mut lock = LockFile::default();
  lock.upsert_entry(sample_entry("drizzle"));
  assert_eq!(lock.addons.len(), 1);
}

#[test]
fn upsert_entry_replaces_existing() {
  let mut lock = LockFile::default();
  lock.upsert_entry(sample_entry("drizzle"));
  let mut replacement = LockEntry::new("drizzle", "2.0.0", "nestjs");
  replacement.upsert_command("install", Default::default(), vec![]);
  lock.upsert_entry(replacement);
  assert_eq!(lock.addons.len(), 1);
  assert_eq!(lock.addons[0].version, "2.0.0");
  assert_eq!(lock.addons[0].variant, "nestjs");
}

#[test]
fn load_drops_delete_created_file_path_outside_root() {
  let dir = assert_fs::TempDir::new().unwrap();
  let outside = assert_fs::TempDir::new().unwrap();
  let mut lock = LockFile::default();
  lock.upsert_entry(entry_with_journal(
    "evil",
    vec![Rollback::DeleteCreatedFile {
      path: outside.path().join("victim-file"),
    }],
  ));
  lock.save(dir.path()).unwrap();

  let loaded = LockFile::load(dir.path()).unwrap();
  assert!(
    loaded.addons[0].commands[0].journal.is_empty(),
    "a journal path outside the project root must be dropped, not fatal"
  );
}

#[test]
fn load_drops_restore_file_path_outside_root() {
  let dir = assert_fs::TempDir::new().unwrap();
  let outside = assert_fs::TempDir::new().unwrap();
  let mut lock = LockFile::default();
  lock.upsert_entry(entry_with_journal(
    "evil",
    vec![Rollback::RestoreFile {
      path: outside.path().join("authorized_keys"),
      original: b"attacker-controlled content".to_vec(),
      mode: None,
      is_symlink: false,
    }],
  ));
  lock.save(dir.path()).unwrap();

  let loaded = LockFile::load(dir.path()).unwrap();
  assert!(loaded.addons[0].commands[0].journal.is_empty());
}

#[test]
fn load_drops_rename_file_when_either_side_is_outside_root() {
  let dir = assert_fs::TempDir::new().unwrap();
  let outside = assert_fs::TempDir::new().unwrap();

  let mut lock_from = LockFile::default();
  lock_from.upsert_entry(entry_with_journal(
    "evil",
    vec![Rollback::RenameFile {
      from: outside.path().join("a"),
      to: dir.path().join("b"),
    }],
  ));
  lock_from.save(dir.path()).unwrap();
  assert!(
    LockFile::load(dir.path()).unwrap().addons[0].commands[0]
      .journal
      .is_empty()
  );

  let mut lock_to = LockFile::default();
  lock_to.upsert_entry(entry_with_journal(
    "evil",
    vec![Rollback::RenameFile {
      from: dir.path().join("a"),
      to: outside.path().join("b"),
    }],
  ));
  lock_to.save(dir.path()).unwrap();
  assert!(
    LockFile::load(dir.path()).unwrap().addons[0].commands[0]
      .journal
      .is_empty()
  );
}

#[test]
fn load_drops_absolute_path_unrelated_to_project() {
  let dir = assert_fs::TempDir::new().unwrap();
  let mut lock = LockFile::default();
  lock.upsert_entry(entry_with_journal(
    "evil",
    vec![Rollback::DeleteCreatedFile {
      path: std::path::PathBuf::from("/etc/passwd"),
    }],
  ));
  lock.save(dir.path()).unwrap();

  let loaded = LockFile::load(dir.path()).unwrap();
  assert!(loaded.addons[0].commands[0].journal.is_empty());
}

#[test]
fn load_survives_a_journal_path_outside_the_root() {
  let dir = assert_fs::TempDir::new().unwrap();
  let outside = assert_fs::TempDir::new().unwrap();
  let mut lock = LockFile::default();
  lock.upsert_entry(entry_with_journal(
    "mixed",
    vec![
      Rollback::DeleteCreatedFile {
        path: outside.path().join("victim"),
      },
      Rollback::IrreversibleRun {
        command: "echo hi".to_string(),
      },
    ],
  ));
  lock.save(dir.path()).unwrap();

  let loaded = LockFile::load(dir.path()).unwrap();
  assert_eq!(
    loaded.addons[0].commands[0].journal.len(),
    1,
    "the outside-root DeleteCreatedFile entry must be dropped, the IrreversibleRun entry kept"
  );
  assert!(matches!(
    loaded.addons[0].commands[0].journal[0],
    Rollback::IrreversibleRun { .. }
  ));
}

#[cfg(unix)]
#[test]
fn load_matches_paths_through_a_symlinked_root() {
  let real_dir = assert_fs::TempDir::new().unwrap();
  let parent = assert_fs::TempDir::new().unwrap();
  let symlinked_root = parent.path().join("project-via-symlink");
  std::os::unix::fs::symlink(real_dir.path(), &symlinked_root).unwrap();

  std::fs::write(real_dir.path().join("created.txt"), b"x").unwrap();

  let mut lock = LockFile::default();
  lock.upsert_entry(entry_with_journal(
    "fine",
    vec![Rollback::DeleteCreatedFile {
      path: symlinked_root.join("created.txt"),
    }],
  ));
  lock.save(real_dir.path()).unwrap();

  let loaded = LockFile::load(real_dir.path()).unwrap();
  assert_eq!(
    loaded.addons[0].commands[0].journal.len(),
    1,
    "a journal path reached through a symlinked root component must still match after canonicalization"
  );
}

#[test]
fn load_accepts_journal_paths_inside_root() {
  let dir = assert_fs::TempDir::new().unwrap();
  let canon_root = dir.path().canonicalize().unwrap();
  let mut lock = LockFile::default();
  lock.upsert_entry(entry_with_journal(
    "fine",
    vec![
      Rollback::DeleteCreatedFile {
        path: canon_root.join("created.txt"),
      },
      Rollback::RestoreFile {
        path: canon_root.join("restored.txt"),
        original: b"content".to_vec(),
        mode: None,
        is_symlink: false,
      },
      Rollback::RenameFile {
        from: canon_root.join("a.txt"),
        to: canon_root.join("b.txt"),
      },
      Rollback::IrreversibleRun {
        command: "echo hi".to_string(),
      },
    ],
  ));
  lock.save(dir.path()).unwrap();

  let loaded = LockFile::load(dir.path()).unwrap();
  assert_eq!(loaded.addons[0].commands[0].journal.len(), 4);
}

#[test]
fn save_stores_relative_paths_on_disk() {
  let dir = assert_fs::TempDir::new().unwrap();
  let canon_root = dir.path().canonicalize().unwrap();
  let mut lock = LockFile::default();
  lock.upsert_entry(entry_with_journal(
    "fine",
    vec![Rollback::DeleteCreatedFile {
      path: canon_root.join("nested").join("created.txt"),
    }],
  ));
  lock.save(dir.path()).unwrap();

  let raw = std::fs::read_to_string(dir.path().join("anesis.lock")).unwrap();
  assert!(
    !raw.contains(canon_root.to_str().unwrap()),
    "the project root prefix must not appear in the on-disk lock file: {raw}"
  );
  assert!(raw.contains("nested"));
  assert_eq!(
    serde_json::from_str::<serde_json::Value>(&raw).unwrap()["schema_version"],
    2
  );
}

#[test]
#[cfg(unix)]
fn load_drops_nonexistent_path_beneath_a_symlink_pointing_outside_root() {
  let dir = assert_fs::TempDir::new().unwrap();
  let outside = assert_fs::TempDir::new().unwrap();
  std::os::unix::fs::symlink(outside.path(), dir.path().join("link")).unwrap();

  let mut lock = LockFile::default();
  lock.upsert_entry(entry_with_journal(
    "evil",
    vec![
      Rollback::DeleteCreatedFile {
        path: dir.path().join("link/new-file"),
      },
      Rollback::RestoreFile {
        path: dir.path().join("link/deep/new-file"),
        original: b"x".to_vec(),
        mode: None,
        is_symlink: false,
      },
      Rollback::RenameFile {
        from: dir.path().join("a"),
        to: dir.path().join("link/new-file"),
      },
    ],
  ));
  lock.save(dir.path()).unwrap();

  let loaded = LockFile::load(dir.path()).unwrap();
  assert!(
    loaded.addons[0].commands[0].journal.is_empty(),
    "paths that resolve outside the root through a symlink must be dropped"
  );
}

#[test]
#[cfg(unix)]
fn load_drops_a_dangling_symlink_that_points_outside_root() {
  let dir = assert_fs::TempDir::new().unwrap();
  let outside = assert_fs::TempDir::new().unwrap();
  std::os::unix::fs::symlink(outside.path().join("missing"), dir.path().join("dangling")).unwrap();

  let mut lock = LockFile::default();
  lock.upsert_entry(entry_with_journal(
    "evil",
    vec![Rollback::RestoreFile {
      path: dir.path().join("dangling"),
      original: b"x".to_vec(),
      mode: None,
      is_symlink: false,
    }],
  ));
  lock.save(dir.path()).unwrap();

  let loaded = LockFile::load(dir.path()).unwrap();
  assert!(loaded.addons[0].commands[0].journal.is_empty());
}

#[test]
#[cfg(unix)]
fn load_keeps_nonexistent_path_beneath_a_symlink_that_stays_inside_root() {
  let dir = assert_fs::TempDir::new().unwrap();
  std::fs::create_dir(dir.path().join("real")).unwrap();
  std::os::unix::fs::symlink(dir.path().join("real"), dir.path().join("link")).unwrap();

  let mut lock = LockFile::default();
  lock.upsert_entry(entry_with_journal(
    "ok",
    vec![Rollback::DeleteCreatedFile {
      path: dir.path().join("link/new-file"),
    }],
  ));
  lock.save(dir.path()).unwrap();

  let loaded = LockFile::load(dir.path()).unwrap();
  assert_eq!(loaded.addons[0].commands[0].journal.len(), 1);
}

use anesis::addons::lock::{LockEntry, LockFile};

fn write_v1_lock(dir: &assert_fs::TempDir, contents: &str) {
  std::fs::write(dir.path().join("anesis.lock"), contents).unwrap();
}

#[test]
fn v1_lock_without_schema_version_migrates() {
  let dir = assert_fs::TempDir::new().unwrap();
  let created = dir.path().canonicalize().unwrap().join("created.txt");
  std::fs::write(&created, "x").unwrap();

  write_v1_lock(
    &dir,
    &serde_json::json!({
      "addons": [{
        "id": "fixture-addon",
        "version": "1.0.0",
        "variant": "universal",
        "commands_executed": ["install"],
        "journal": [
          { "DeleteCreatedFile": { "path": created.to_string_lossy() } }
        ],
        "inputs": { "name": "svc" }
      }]
    })
    .to_string(),
  );

  let lock = LockFile::load(dir.path()).unwrap();
  assert_eq!(lock.addons.len(), 1);
  let entry = &lock.addons[0];
  assert_eq!(entry.id, "fixture-addon");
  assert_eq!(entry.version, "1.0.0");
  assert!(lock.is_command_executed("fixture-addon", "install"));
  assert!(
    entry.has_undoable_changes(),
    "the migrated entry must still be undoable"
  );
  assert_eq!(
    entry.commands[0].inputs.get("name"),
    Some(&"svc".to_string()),
    "the flat v1 inputs must carry over onto the migrated command"
  );
}

#[test]
fn v1_absolute_paths_relativize() {
  let dir = assert_fs::TempDir::new().unwrap();
  let canon_root = dir.path().canonicalize().unwrap();
  let created = canon_root.join("nested").join("created.txt");
  std::fs::create_dir_all(created.parent().unwrap()).unwrap();
  std::fs::write(&created, "x").unwrap();

  write_v1_lock(
    &dir,
    &serde_json::json!({
      "addons": [{
        "id": "fixture-addon",
        "version": "1.0.0",
        "variant": "universal",
        "commands_executed": ["install"],
        "journal": [
          { "DeleteCreatedFile": { "path": created.to_string_lossy() } }
        ],
        "inputs": {}
      }]
    })
    .to_string(),
  );

  let lock = LockFile::load(dir.path()).unwrap();
  lock.save(dir.path()).unwrap();

  let raw = std::fs::read_to_string(dir.path().join("anesis.lock")).unwrap();
  assert!(
    !raw.contains(canon_root.to_str().unwrap()),
    "after a load+save roundtrip, the v1 absolute path must have become relative: {raw}"
  );
  assert_eq!(
    serde_json::from_str::<serde_json::Value>(&raw).unwrap()["schema_version"],
    2
  );

  let reloaded = LockFile::load(dir.path()).unwrap();
  assert!(reloaded.addons[0].has_undoable_changes());
}

#[test]
fn v1_from_a_moved_project_drops_journal_with_warning() {
  let dir = assert_fs::TempDir::new().unwrap();
  let elsewhere = assert_fs::TempDir::new().unwrap();
  let moved_away_path = elsewhere.path().join("created.txt");
  std::fs::write(&moved_away_path, "x").unwrap();

  write_v1_lock(
    &dir,
    &serde_json::json!({
      "addons": [{
        "id": "fixture-addon",
        "version": "1.0.0",
        "variant": "universal",
        "commands_executed": ["install"],
        "journal": [
          { "DeleteCreatedFile": { "path": moved_away_path.to_string_lossy() } }
        ],
        "inputs": {}
      }]
    })
    .to_string(),
  );

  let lock = LockFile::load(dir.path()).unwrap();
  assert_eq!(
    lock.addons.len(),
    1,
    "the addon entry itself must survive a moved project -- never hard-fail"
  );
  let entry = &lock.addons[0];
  assert_eq!(entry.id, "fixture-addon");
  assert!(
    lock.is_command_executed("fixture-addon", "install"),
    "commands_executed bookkeeping must survive even though the journal was dropped"
  );
  assert!(
    !entry.has_undoable_changes(),
    "a journal path outside the current project root must be dropped, not kept dangling"
  );
}

#[test]
fn v2_roundtrips() {
  let dir = assert_fs::TempDir::new().unwrap();

  let mut lock = LockFile::default();
  let mut entry = LockEntry::new("fixture-addon", "1.0.0", "universal");
  entry.inputs.insert("lang".to_string(), "rust".to_string());
  entry.upsert_command(
    "install",
    [("name".to_string(), "svc".to_string())].into(),
    vec![anesis::addons::steps::Rollback::DeleteCreatedFile {
      path: dir.path().canonicalize().unwrap().join("out.txt"),
    }],
  );
  lock.addons.push(entry);
  lock.save(dir.path()).unwrap();

  let reloaded = LockFile::load(dir.path()).unwrap();
  assert_eq!(reloaded.addons.len(), 1);
  let entry = &reloaded.addons[0];
  assert_eq!(entry.version, "1.0.0");
  assert_eq!(entry.inputs.get("lang"), Some(&"rust".to_string()));
  assert_eq!(entry.commands.len(), 1);
  assert_eq!(entry.commands[0].name, "install");
  assert_eq!(
    entry.commands[0].inputs.get("name"),
    Some(&"svc".to_string())
  );
  assert_eq!(entry.commands[0].journal.len(), 1);
}

#[test]
fn journal_with_dotdot_is_rejected() {
  let dir = assert_fs::TempDir::new().unwrap();

  std::fs::write(
    dir.path().join("anesis.lock"),
    serde_json::json!({
      "schema_version": 2,
      "addons": [{
        "id": "evil",
        "version": "1.0.0",
        "variant": "universal",
        "inputs": {},
        "commands": [{
          "name": "install",
          "inputs": {},
          "journal": [
            { "DeleteCreatedFile": { "path": "../../../../etc/passwd" } }
          ]
        }]
      }]
    })
    .to_string(),
  )
  .unwrap();

  let lock = LockFile::load(dir.path()).unwrap();
  assert_eq!(lock.addons.len(), 1, "the entry itself must survive");
  assert!(
    !lock.addons[0].has_undoable_changes(),
    "a relative journal path containing '..' must never be allowed to escape the project root"
  );
}

use std::path::PathBuf;

use anesis::addons::lock::{LockEntry, LockFile};
use anesis::addons::steps::Rollback;
use anesis::why::build_index;
use assert_fs::TempDir;

fn path(s: &str) -> PathBuf {
  PathBuf::from(s)
}

#[test]
fn a_created_file_is_indexed_as_created() {
  let mut entry = LockEntry::new("nest-prisma-v7", "2.1.0", "postgres");
  entry.upsert_command(
    "generate",
    Default::default(),
    vec![Rollback::DeleteCreatedFile {
      path: path("/proj/src/prisma/schema.prisma"),
    }],
  );
  let lock = LockFile {
    addons: vec![entry],
  };

  let index = build_index(&lock);
  let entries = index
    .get(&path("/proj/src/prisma/schema.prisma"))
    .expect("indexed");
  assert_eq!(entries.len(), 1);
  assert_eq!(entries[0].addon_id, "nest-prisma-v7");
  assert_eq!(entries[0].version, "2.1.0");
  assert_eq!(entries[0].command, "generate");
  assert_eq!(entries[0].kind, "created");
}

#[test]
fn a_restored_file_is_indexed_as_modified() {
  let mut entry = LockEntry::new("nest-auth", "1.4.0", "universal");
  entry.upsert_command(
    "setup",
    Default::default(),
    vec![Rollback::restore_file_for_tests(
      path("/proj/package.json"),
      b"old".to_vec(),
    )],
  );
  let lock = LockFile {
    addons: vec![entry],
  };

  let index = build_index(&lock);
  let entries = index.get(&path("/proj/package.json")).expect("indexed");
  assert_eq!(entries[0].kind, "modified");
}

#[test]
fn a_renamed_file_is_indexed_at_its_destination() {
  let mut entry = LockEntry::new("nest-x", "1.0.0", "universal");
  entry.upsert_command(
    "setup",
    Default::default(),
    vec![Rollback::RenameFile {
      from: path("/proj/old-name.ts"),
      to: path("/proj/src/app.module.ts"),
    }],
  );
  let lock = LockFile {
    addons: vec![entry],
  };

  let index = build_index(&lock);
  assert!(!index.contains_key(&path("/proj/old-name.ts")));
  let entries = index
    .get(&path("/proj/src/app.module.ts"))
    .expect("indexed at destination");
  assert_eq!(entries[0].kind, "renamed");
}

#[test]
fn irreversible_runs_are_not_indexed_as_files() {
  let mut entry = LockEntry::new("nest-x", "1.0.0", "universal");
  entry.upsert_command(
    "setup",
    Default::default(),
    vec![Rollback::IrreversibleRun {
      command: "npm install".to_string(),
    }],
  );
  let lock = LockFile {
    addons: vec![entry],
  };

  let index = build_index(&lock);
  assert!(index.is_empty());
}

#[test]
fn multiple_addons_touching_the_same_file_all_appear() {
  let mut prisma = LockEntry::new("nest-prisma-v7", "2.1.0", "postgres");
  prisma.upsert_command(
    "generate",
    Default::default(),
    vec![Rollback::restore_file_for_tests(
      path("/proj/package.json"),
      b"a".to_vec(),
    )],
  );
  let mut auth = LockEntry::new("nest-auth", "1.4.0", "universal");
  auth.upsert_command(
    "setup",
    Default::default(),
    vec![Rollback::restore_file_for_tests(
      path("/proj/package.json"),
      b"b".to_vec(),
    )],
  );
  let lock = LockFile {
    addons: vec![prisma, auth],
  };

  let index = build_index(&lock);
  let entries = index.get(&path("/proj/package.json")).expect("indexed");
  assert_eq!(entries.len(), 2);
  assert!(entries.iter().any(|e| e.addon_id == "nest-prisma-v7"));
  assert!(entries.iter().any(|e| e.addon_id == "nest-auth"));
}

#[test]
fn why_json_reports_an_empty_entries_list_for_an_unknown_file() {
  let project = TempDir::new().unwrap();
  let lock = LockFile::load(project.path()).unwrap();
  let index = build_index(&lock);
  assert!(index.is_empty());
}

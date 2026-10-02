use std::path::PathBuf;

use anesis::addons::steps::Rollback;
use anesis::addons::summary::ChangeSummary;

fn path(s: &str) -> PathBuf {
  PathBuf::from(s)
}

#[test]
fn no_rollbacks_renders_no_changes() {
  let summary = ChangeSummary::from_rollbacks(&[]);
  assert!(summary.is_empty());
  assert_eq!(summary.render(), "No changes.");
}

#[test]
fn delete_created_file_counts_as_created() {
  let rollbacks = vec![Rollback::DeleteCreatedFile {
    path: path("new.txt"),
  }];
  let summary = ChangeSummary::from_rollbacks(&rollbacks);
  assert_eq!(summary.created, 1);
  assert_eq!(summary.files_changed(), 1);
  assert_eq!(summary.render(), "1 file changed, 1 created");
}

#[test]
fn restore_file_counts_as_modified() {
  let rollbacks = vec![Rollback::restore_file_for_tests(
    path("existing.txt"),
    b"old".to_vec(),
  )];
  let summary = ChangeSummary::from_rollbacks(&rollbacks);
  assert_eq!(summary.modified, 1);
  assert_eq!(summary.render(), "1 file changed, 1 modified");
}

#[test]
fn rename_file_counts_as_renamed() {
  let rollbacks = vec![Rollback::RenameFile {
    from: path("b.txt"),
    to: path("a.txt"),
  }];
  let summary = ChangeSummary::from_rollbacks(&rollbacks);
  assert_eq!(summary.renamed, 1);
  assert_eq!(summary.render(), "1 file changed, 1 renamed");
}

#[test]
fn irreversible_run_is_reported_separately_from_file_changes() {
  let rollbacks = vec![Rollback::IrreversibleRun {
    command: "npm install".to_string(),
  }];
  let summary = ChangeSummary::from_rollbacks(&rollbacks);
  assert_eq!(summary.files_changed(), 0);
  assert_eq!(summary.irreversible_runs, 1);
  assert_eq!(summary.render(), "1 shell command run (not reversible)");
}

#[test]
fn mixed_rollbacks_render_every_category() {
  let rollbacks = vec![
    Rollback::DeleteCreatedFile {
      path: path("new.txt"),
    },
    Rollback::restore_file_for_tests(path("existing.txt"), b"old".to_vec()),
    Rollback::restore_file_for_tests(path("existing2.txt"), b"old2".to_vec()),
    Rollback::RenameFile {
      from: path("b.txt"),
      to: path("a.txt"),
    },
    Rollback::IrreversibleRun {
      command: "npm install".to_string(),
    },
    Rollback::IrreversibleRun {
      command: "npm run build".to_string(),
    },
  ];
  let summary = ChangeSummary::from_rollbacks(&rollbacks);
  assert_eq!(summary.files_changed(), 4);
  assert_eq!(
    summary.render(),
    "4 files changed, 1 created, 2 modified, 1 renamed, 2 shell commands run (not reversible)"
  );
}

#[test]
fn render_block_of_no_rollbacks_is_no_changes() {
  let summary = ChangeSummary::from_rollbacks(&[]);
  assert_eq!(summary.render_block(&[]), "No changes.");
}

#[test]
fn render_block_lists_each_file_with_a_kind_marker() {
  let rollbacks = vec![
    Rollback::DeleteCreatedFile {
      path: path("new.txt"),
    },
    Rollback::restore_file_for_tests(path("existing.txt"), b"old".to_vec()),
    Rollback::RenameFile {
      from: path("b.txt"),
      to: path("a.txt"),
    },
  ];
  let summary = ChangeSummary::from_rollbacks(&rollbacks);
  let block = summary.render_block(&rollbacks);
  let lines: Vec<&str> = block.lines().collect();

  assert_eq!(lines.len(), 4, "1 header + 3 file lines: {block}");
  assert!(lines[0].contains("3 files changed"));
  assert!(lines[1].contains('+') && lines[1].contains("new.txt"));
  assert!(lines[2].contains('~') && lines[2].contains("existing.txt"));
  assert!(lines[3].contains("a.txt"));
}

#[test]
fn render_block_omits_irreversible_runs_from_the_file_list() {
  let rollbacks = vec![
    Rollback::DeleteCreatedFile {
      path: path("new.txt"),
    },
    Rollback::IrreversibleRun {
      command: "npm install".to_string(),
    },
  ];
  let summary = ChangeSummary::from_rollbacks(&rollbacks);
  let block = summary.render_block(&rollbacks);

  assert!(!block.contains("npm install"));
  assert!(block.contains("new.txt"));
}

#[test]
fn render_block_caps_the_listing_and_reports_the_remainder() {
  let rollbacks: Vec<Rollback> = (0..25)
    .map(|i| Rollback::DeleteCreatedFile {
      path: path(&format!("file{i}.txt")),
    })
    .collect();
  let summary = ChangeSummary::from_rollbacks(&rollbacks);
  let block = summary.render_block(&rollbacks);
  let lines: Vec<&str> = block.lines().collect();

  // 1 header + 20 shown + 1 "and N more" line
  assert_eq!(lines.len(), 22, "{block}");
  assert!(lines.last().unwrap().contains("5 more"));
}

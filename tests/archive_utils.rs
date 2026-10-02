mod common;

use std::path::Path;

use common::strip_archive_path_for_tests;

#[test]
fn strips_single_root_component() {
  let raw = Path::new("owner-repo-abc123/src/main.rs");
  let result = strip_archive_path_for_tests(raw, None).unwrap();
  assert_eq!(result, Path::new("src/main.rs"));
}

#[test]
fn strips_root_for_file_directly_in_archive_root() {
  let raw = Path::new("root-dir/README.md");
  let result = strip_archive_path_for_tests(raw, None).unwrap();
  assert_eq!(result, Path::new("README.md"));
}

#[test]
fn returns_none_for_root_directory_entry() {
  let raw = Path::new("root-dir/");
  let result = strip_archive_path_for_tests(raw, None);
  assert!(
    result.is_none(),
    "the archive root directory entry should be skipped"
  );
}

#[test]
fn returns_none_for_bare_root_component() {
  let raw = Path::new("only-root");
  let result = strip_archive_path_for_tests(raw, None);
  assert!(
    result.is_none(),
    "a path with only the root component produces an empty remainder"
  );
}

#[test]
fn strips_subdir_prefix_from_matching_entry() {
  let raw = Path::new("root/templates/react/src/main.tsx");
  let result = strip_archive_path_for_tests(raw, Some("templates/react")).unwrap();
  assert_eq!(result, Path::new("src/main.tsx"));
}

#[test]
fn returns_none_for_entry_outside_subdir() {
  let raw = Path::new("root/other-dir/file.txt");
  let result = strip_archive_path_for_tests(raw, Some("templates"));
  assert!(
    result.is_none(),
    "entries outside the requested subdir should be skipped"
  );
}

#[test]
fn returns_none_for_subdir_directory_entry_itself() {
  let raw = Path::new("root/templates/");
  let result = strip_archive_path_for_tests(raw, Some("templates"));
  assert!(
    result.is_none(),
    "the subdir directory entry itself should be skipped"
  );
}

#[test]
fn returns_none_when_subdir_matches_partially() {
  let raw = Path::new("root/templates-extra/file.ts");
  let result = strip_archive_path_for_tests(raw, Some("templates"));
  assert!(
    result.is_none(),
    "partial prefix match should not count as inside subdir"
  );
}

#[test]
fn nested_subdir_path_is_preserved() {
  let raw = Path::new("root/pkg/a/b/c/deep.txt");
  let result = strip_archive_path_for_tests(raw, Some("pkg/a")).unwrap();
  assert_eq!(result, Path::new("b/c/deep.txt"));
}

#[test]
fn no_subdir_preserves_deep_nesting() {
  let raw = Path::new("archive-root/a/b/c/d/e.txt");
  let result = strip_archive_path_for_tests(raw, None).unwrap();
  assert_eq!(result, Path::new("a/b/c/d/e.txt"));
}

#[test]
fn strips_a_leading_dot_prefix_before_the_root_component() {
  let raw = Path::new("./archive-root/src/main.rs");
  let result = strip_archive_path_for_tests(raw, None).unwrap();
  assert_eq!(
    result,
    Path::new("src/main.rs"),
    "a tarball built as `tar czf out.tar.gz ./dir` prefixes every entry with \
     './' — that must not be counted as the top-level wrapper directory, or \
     nothing gets extracted"
  );
}

#[test]
fn strips_a_leading_dot_prefix_with_a_matching_subdir() {
  let raw = Path::new("./root/templates/react/src/main.tsx");
  let result = strip_archive_path_for_tests(raw, Some("templates/react")).unwrap();
  assert_eq!(result, Path::new("src/main.tsx"));
}

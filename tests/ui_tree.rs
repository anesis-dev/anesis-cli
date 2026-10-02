use anesis::utils::ui::tree::{TreeNode, render};

#[test]
fn empty_tree_is_just_the_root_label() {
  let root = TreeNode::new("root");
  assert_eq!(render(&root), "root");
}

#[test]
fn single_child_uses_the_last_branch_glyph() {
  let root = TreeNode::new("root").child(TreeNode::new("only"));
  let out = render(&root);
  let lines: Vec<&str> = out.lines().collect();
  assert_eq!(lines[0], "root");
  assert!(lines[1].ends_with("only"));
  assert!(
    lines[1] != "├─ only",
    "single child must use the last-branch glyph"
  );
}

#[test]
fn nested_children_render_with_a_vertical_continuation() {
  let root = TreeNode::new("root").child(
    TreeNode::new("a")
      .child(TreeNode::new("a1"))
      .child(TreeNode::new("a2")),
  );
  let out = render(&root);
  let lines: Vec<&str> = out.lines().collect();
  assert_eq!(lines.len(), 4);
  assert!(lines[2].contains("a1"));
  assert!(lines[3].contains("a2"));
}

#[test]
fn last_sibling_is_marked_differently_from_earlier_siblings() {
  let root = TreeNode::new("root")
    .child(TreeNode::new("first"))
    .child(TreeNode::new("second"));
  let out = render(&root);
  let lines: Vec<&str> = out.lines().collect();
  assert_ne!(
    lines[1].chars().next(),
    lines[2].chars().next(),
    "first and last siblings use different branch glyphs"
  );
}

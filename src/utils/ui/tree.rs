use super::symbols;

pub struct TreeNode {
  pub label: String,
  pub children: Vec<TreeNode>,
}

impl TreeNode {
  pub fn new(label: impl Into<String>) -> Self {
    Self {
      label: label.into(),
      children: Vec::new(),
    }
  }

  pub fn child(mut self, node: TreeNode) -> Self {
    self.children.push(node);
    self
  }
}

pub fn render(root: &TreeNode) -> String {
  let mut out = String::new();
  out.push_str(&root.label);
  out.push('\n');
  render_children(&root.children, "", &mut out);
  out.trim_end().to_string()
}

fn render_children(nodes: &[TreeNode], prefix: &str, out: &mut String) {
  let len = nodes.len();
  for (i, node) in nodes.iter().enumerate() {
    let last = i + 1 == len;
    let branch = if last {
      symbols::tree_last()
    } else {
      symbols::tree_branch()
    };
    out.push_str(prefix);
    out.push_str(branch);
    out.push_str(&node.label);
    out.push('\n');
    let child_prefix = format!(
      "{prefix}{}",
      if last { "   " } else { symbols::tree_vert() }
    );
    render_children(&node.children, &child_prefix, out);
  }
}

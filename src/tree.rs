//! # Tree Printing Module
//!
//! This module provides functionality for pretty-printing a list of file paths
//! as a tree structure, similar to the Unix `tree` command.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A node in the file tree structure.
#[derive(Debug, Default)]
struct TreeNode {
  /// Child nodes (directories or files)
  children: BTreeMap<String, TreeNode>,
  /// Whether this node represents a file (leaf node)
  is_file: bool,
}

impl TreeNode {
  /// Insert a path into the tree.
  fn insert(&mut self, path: &Path) {
    let components: Vec<_> = path
      .components()
      .map(|c| c.as_os_str().to_string_lossy().to_string())
      .collect();
    self.insert_components(&components, 0);
  }

  fn insert_components(&mut self, components: &[String], index: usize) {
    if index >= components.len() {
      return;
    }

    let component = &components[index];
    let child = self.children.entry(component.clone()).or_default();

    if index == components.len() - 1 {
      // This is the last component, mark as file
      child.is_file = true;
    } else {
      // Continue building the tree
      child.insert_components(components, index + 1);
    }
  }

  /// Render the tree to a string.
  fn render(&self, prefix: &str) -> Vec<String> {
    let mut lines = Vec::new();

    let children: Vec<_> = self.children.iter().collect();
    let count = children.len();

    for (i, (name, child)) in children.iter().enumerate() {
      let is_last_child = i == count - 1;

      // Determine the branch character
      let branch = if is_last_child { "└── " } else { "├── " };

      // Build the line
      let line = format!("{}{}{}", prefix, branch, name);
      lines.push(line);

      // Determine the prefix for children
      let child_prefix = if is_last_child {
        format!("{}    ", prefix)
      } else {
        format!("{}│   ", prefix)
      };

      // Recursively render children
      if !child.children.is_empty() {
        let child_lines = child.render(&child_prefix);
        lines.extend(child_lines);
      }
    }

    lines
  }
}

/// Pretty-prints a list of file paths as a tree structure.
///
/// # Parameters
///
/// * `files` - A slice of file paths to display
/// * `base_path` - The base path to display as the root (if provided, paths
///   will be shown relative to it)
///
/// # Returns
///
/// A string containing the tree representation.
pub fn print_tree(files: &[PathBuf], base_path: Option<&Path>) -> String {
  if files.is_empty() {
    return "(no files)\n".to_string();
  }

  let mut root = TreeNode::default();

  // Convert paths to relative if base_path is provided
  for file in files {
    let relative_path = if let Some(base) = base_path {
      file.strip_prefix(base).unwrap_or(file).to_path_buf()
    } else {
      file.clone()
    };
    root.insert(&relative_path);
  }

  // Render the tree
  let mut lines = Vec::new();

  // Add root indicator if base_path is provided
  if let Some(base) = base_path {
    lines.push(format!("{}", base.display()));
  } else {
    lines.push(".".to_string());
  }

  let tree_lines = root.render("");
  lines.extend(tree_lines);

  // Add summary
  let file_count = files.len();
  let dir_count = count_directories(&root);

  lines.push(String::new()); // Empty line before summary
  if dir_count == 1 {
    lines.push(format!("{} directory, {} files", dir_count, file_count));
  } else {
    lines.push(format!("{} directories, {} files", dir_count, file_count));
  }

  lines.join("\n")
}

/// Count the number of directories in the tree.
fn count_directories(node: &TreeNode) -> usize {
  let mut count = 0;
  for child in node.children.values() {
    if !child.is_file || !child.children.is_empty() {
      count += 1;
    }
    count += count_directories(child);
  }
  count
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_empty_tree() {
    let files: Vec<PathBuf> = vec![];
    let result = print_tree(&files, None);
    assert_eq!(result, "(no files)\n");
  }

  #[test]
  fn test_single_file() {
    let files = vec![PathBuf::from("src/main.rs")];
    let result = print_tree(&files, None);
    assert!(result.contains("src"));
    assert!(result.contains("main.rs"));
    assert!(result.contains("1 directory, 1 files"));
  }

  #[test]
  fn test_multiple_files() {
    let files = vec![
      PathBuf::from("src/main.rs"),
      PathBuf::from("src/lib.rs"),
      PathBuf::from("tests/test.rs"),
    ];
    let result = print_tree(&files, None);
    assert!(result.contains("src"));
    assert!(result.contains("main.rs"));
    assert!(result.contains("lib.rs"));
    assert!(result.contains("tests"));
    assert!(result.contains("test.rs"));
  }

  #[test]
  fn test_nested_structure() {
    let files = vec![
      PathBuf::from("src/cli/mod.rs"),
      PathBuf::from("src/cli/check.rs"),
      PathBuf::from("src/main.rs"),
    ];
    let result = print_tree(&files, None);
    assert!(result.contains("src"));
    assert!(result.contains("cli"));
    assert!(result.contains("mod.rs"));
    assert!(result.contains("check.rs"));
    assert!(result.contains("main.rs"));
  }
}

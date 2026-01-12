//! # Workspace Module
//!
//! This module defines the workspace root that edlicense operates on.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::git;

/// Workspace root selection.
pub enum Workspace {
  /// Workspace rooted at a git repository.
  Git { root: PathBuf },
  /// Workspace rooted at a directory.
  Directory { root: PathBuf },
}

impl Workspace {
  pub fn root(&self) -> &Path {
    match self {
      Self::Git { root } | Self::Directory { root } => root.as_path(),
    }
  }

  pub const fn is_git(&self) -> bool {
    matches!(self, Self::Git { .. })
  }
}

/// Resolve the current workspace based on the current directory and patterns.
pub fn resolve_workspace(patterns: &[String]) -> Result<Workspace> {
  let current_dir = std::env::current_dir().with_context(|| "Failed to get current directory")?;

  if let Some(root) = git::discover_repo_root(&current_dir)? {
    return Ok(Workspace::Git { root });
  }

  if let Some(root) = resolve_workspace_from_patterns(patterns, &current_dir) {
    return Ok(Workspace::Directory { root });
  }

  Ok(Workspace::Directory { root: current_dir })
}

fn resolve_workspace_from_patterns(patterns: &[String], current_dir: &Path) -> Option<PathBuf> {
  for pattern in patterns {
    let candidate = PathBuf::from(pattern);
    if candidate.exists() {
      if candidate.is_dir() {
        return Some(abs_path_or_current(&candidate, current_dir));
      }

      if candidate.is_file()
        && let Some(parent) = candidate.parent()
      {
        return Some(abs_path_or_current(parent, current_dir));
      }
    }
  }

  None
}

fn abs_path_or_current(path: &Path, current_dir: &Path) -> PathBuf {
  let abs_path = if path.is_absolute() {
    path.to_path_buf()
  } else {
    current_dir.join(path)
  };

  // Normalize the path to remove `.` and `..` components.
  // This is important for prefix checks like `dir.starts_with(workspace_root)`.
  normalize_path(&abs_path)
}

/// Normalize a path by resolving `.` and `..` components without following
/// symlinks.
///
/// Arguments:
/// - `path`: The path to normalize.
///
/// Returns:
/// - A normalized `PathBuf`.
///
/// Examples:
/// ```
/// let path = PathBuf::from("/home/user/project/./src/../lib");
/// let normalized = normalize_path(&path);
/// assert_eq!(normalized, PathBuf::from("/home/user/project/lib"));
/// ```
fn normalize_path(path: &Path) -> PathBuf {
  use std::path::Component;

  let mut components = Vec::new();
  for component in path.components() {
    match component {
      Component::CurDir => {}
      Component::ParentDir => {
        if let Some(Component::Normal(_)) = components.last() {
          components.pop();
        } else {
          components.push(component);
        }
      }
      _ => components.push(component),
    }
  }

  components.iter().collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_normalize_path_removes_dot_components() {
    let path = PathBuf::from("/home/user/project/.");
    assert_eq!(normalize_path(&path), PathBuf::from("/home/user/project"));

    let path = PathBuf::from("/home/user/./project/./src/.");
    assert_eq!(normalize_path(&path), PathBuf::from("/home/user/project/src"));
  }

  #[test]
  fn test_normalize_path_resolves_parent_dir() {
    let path = PathBuf::from("/home/user/project/../other");
    assert_eq!(normalize_path(&path), PathBuf::from("/home/user/other"));

    let path = PathBuf::from("/home/user/project/src/../lib");
    assert_eq!(normalize_path(&path), PathBuf::from("/home/user/project/lib"));
  }

  #[test]
  fn test_normalize_path_mixed() {
    let path = PathBuf::from("/home/user/./project/../other/./src");
    assert_eq!(normalize_path(&path), PathBuf::from("/home/user/other/src"));
  }

  #[test]
  fn test_normalize_path_preserves_clean_paths() {
    let path = PathBuf::from("/home/user/project");
    assert_eq!(normalize_path(&path), PathBuf::from("/home/user/project"));
  }

  #[test]
  fn test_abs_path_or_current_normalizes_dot() {
    let current = PathBuf::from("/home/user/project");
    let result = abs_path_or_current(Path::new("."), &current);
    assert_eq!(result, PathBuf::from("/home/user/project"));
  }

  #[test]
  fn test_abs_path_or_current_normalizes_subdir() {
    let current = PathBuf::from("/home/user/project");
    let result = abs_path_or_current(Path::new("./subdir"), &current);
    assert_eq!(result, PathBuf::from("/home/user/project/subdir"));
  }
}

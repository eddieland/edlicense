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
  use std::fs;

  use tempfile::TempDir;

  use super::*;

  /// Guard that changes the current working directory and restores it on drop.
  /// With nextest (separate processes per test), restoration isn't strictly
  /// necessary, but it's good practice and allows tests to run correctly
  /// with `cargo test` as well.
  struct CwdGuard {
    original: PathBuf,
  }

  impl CwdGuard {
    fn new(new_cwd: &Path) -> Self {
      let original = std::env::current_dir().expect("test setup: failed to get current directory");
      std::env::set_current_dir(new_cwd).expect("test setup: failed to change directory");
      Self { original }
    }
  }

  impl Drop for CwdGuard {
    fn drop(&mut self) {
      // Best-effort restore; ignore errors during test teardown
      let _ = std::env::set_current_dir(&self.original);
    }
  }

  #[test]
  fn test_workspace_root_returns_path_for_git_variant() {
    let path = PathBuf::from("/some/git/repo");
    let workspace = Workspace::Git { root: path.clone() };
    assert_eq!(workspace.root(), path.as_path());
  }

  #[test]
  fn test_workspace_root_returns_path_for_directory_variant() {
    let path = PathBuf::from("/some/directory");
    let workspace = Workspace::Directory { root: path.clone() };
    assert_eq!(workspace.root(), path.as_path());
  }

  #[test]
  fn test_workspace_is_git_returns_true_for_git_variant() {
    let workspace = Workspace::Git {
      root: PathBuf::from("/repo"),
    };
    assert!(workspace.is_git());
  }

  #[test]
  fn test_workspace_is_git_returns_false_for_directory_variant() {
    let workspace = Workspace::Directory {
      root: PathBuf::from("/dir"),
    };
    assert!(!workspace.is_git());
  }

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
  fn test_normalize_path_handles_relative_paths() {
    let path = PathBuf::from("foo/./bar/../baz");
    assert_eq!(normalize_path(&path), PathBuf::from("foo/baz"));
  }

  #[test]
  fn test_normalize_path_handles_leading_parent_dir() {
    // Leading `..` can't be resolved, so it should be preserved
    let path = PathBuf::from("../foo/bar");
    assert_eq!(normalize_path(&path), PathBuf::from("../foo/bar"));
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

  #[test]
  fn test_abs_path_or_current_preserves_absolute_path() {
    let current = PathBuf::from("/home/user/project");
    let result = abs_path_or_current(Path::new("/absolute/path"), &current);
    assert_eq!(result, PathBuf::from("/absolute/path"));
  }

  #[test]
  fn test_abs_path_or_current_joins_relative_path() {
    let current = PathBuf::from("/home/user/project");
    let result = abs_path_or_current(Path::new("relative/path"), &current);
    assert_eq!(result, PathBuf::from("/home/user/project/relative/path"));
  }

  #[test]
  fn test_resolve_workspace_from_patterns_with_existing_directory() {
    let temp = TempDir::new().expect("failed to create temp dir");
    let subdir = temp.path().join("subdir");
    fs::create_dir(&subdir).expect("failed to create subdir");

    let patterns = vec![subdir.to_string_lossy().to_string()];
    let result = resolve_workspace_from_patterns(&patterns, temp.path());

    assert_eq!(result, Some(subdir));
  }

  #[test]
  fn test_resolve_workspace_from_patterns_with_existing_file() {
    let temp = TempDir::new().expect("failed to create temp dir");
    let file = temp.path().join("file.txt");
    fs::write(&file, "content").expect("failed to create file");

    let patterns = vec![file.to_string_lossy().to_string()];
    let result = resolve_workspace_from_patterns(&patterns, temp.path());

    // Should return the parent directory of the file
    assert_eq!(result, Some(temp.path().to_path_buf()));
  }

  #[test]
  fn test_resolve_workspace_from_patterns_with_nonexistent_pattern() {
    let temp = TempDir::new().expect("failed to create temp dir");

    let patterns = vec!["/nonexistent/path".to_string()];
    let result = resolve_workspace_from_patterns(&patterns, temp.path());

    assert_eq!(result, None);
  }

  #[test]
  fn test_resolve_workspace_from_patterns_with_empty_patterns() {
    let temp = TempDir::new().expect("failed to create temp dir");

    let patterns: Vec<String> = vec![];
    let result = resolve_workspace_from_patterns(&patterns, temp.path());

    assert_eq!(result, None);
  }

  #[test]
  fn test_resolve_workspace_from_patterns_uses_first_matching_pattern() {
    let temp = TempDir::new().expect("failed to create temp dir");
    let dir1 = temp.path().join("dir1");
    let dir2 = temp.path().join("dir2");
    fs::create_dir(&dir1).expect("failed to create dir1");
    fs::create_dir(&dir2).expect("failed to create dir2");

    let patterns = vec![dir1.to_string_lossy().to_string(), dir2.to_string_lossy().to_string()];
    let result = resolve_workspace_from_patterns(&patterns, temp.path());

    // Should return the first matching pattern
    assert_eq!(result, Some(dir1));
  }

  #[test]
  fn test_resolve_workspace_from_patterns_skips_nonexistent_finds_existing() {
    let temp = TempDir::new().expect("failed to create temp dir");
    let existing = temp.path().join("existing");
    fs::create_dir(&existing).expect("failed to create existing dir");

    let patterns = vec!["/nonexistent/first".to_string(), existing.to_string_lossy().to_string()];
    let result = resolve_workspace_from_patterns(&patterns, temp.path());

    assert_eq!(result, Some(existing));
  }

  #[test]
  fn test_resolve_workspace_from_patterns_with_relative_directory() {
    let temp = TempDir::new().expect("failed to create temp dir");
    let subdir = temp.path().join("subdir");
    fs::create_dir(&subdir).expect("failed to create subdir");

    // Change to temp directory so relative path works
    let _guard = CwdGuard::new(temp.path());

    let patterns = vec!["subdir".to_string()];
    let result = resolve_workspace_from_patterns(&patterns, temp.path());

    assert_eq!(result, Some(subdir));
  }

  #[test]
  fn test_resolve_workspace_falls_back_to_current_dir() {
    // Create a temp directory that is NOT a git repo
    let temp = TempDir::new().expect("failed to create temp dir");
    let _guard = CwdGuard::new(temp.path());

    // Empty patterns and no git repo should fall back to current directory
    let result = resolve_workspace(&[]).expect("resolve_workspace failed");

    assert!(!result.is_git());
    // The result should be the temp directory (possibly canonicalized)
    assert_eq!(
      result.root().file_name(),
      temp.path().file_name(),
      "workspace root should be the temp directory"
    );
  }

  #[test]
  fn test_resolve_workspace_uses_pattern_directory_when_no_git() {
    let temp = TempDir::new().expect("failed to create temp dir");
    let subdir = temp.path().join("myproject");
    fs::create_dir(&subdir).expect("failed to create subdir");

    let _guard = CwdGuard::new(temp.path());

    let patterns = vec!["myproject".to_string()];
    let result = resolve_workspace(&patterns).expect("resolve_workspace failed");

    assert!(!result.is_git());
    assert_eq!(result.root(), subdir);
  }

  #[test]
  fn test_resolve_workspace_uses_pattern_file_parent_when_no_git() {
    let temp = TempDir::new().expect("failed to create temp dir");
    let file = temp.path().join("somefile.rs");
    fs::write(&file, "fn main() {}").expect("failed to create file");

    let _guard = CwdGuard::new(temp.path());

    let patterns = vec!["somefile.rs".to_string()];
    let result = resolve_workspace(&patterns).expect("resolve_workspace failed");

    assert!(!result.is_git());
    // Should return parent of the file, which is temp dir
    assert_eq!(
      result.root().file_name(),
      temp.path().file_name(),
      "workspace root should be the file's parent directory"
    );
  }
}

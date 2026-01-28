//! # File Collector Module
//!
//! This module provides utilities for collecting files from directories,
//! pattern matching, and path normalization.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::debug;

/// Pattern matcher for file collection.
pub enum PatternMatcher {
  /// Matches any file
  Any,
  /// Matches a specific file path
  File(PathBuf),
  /// Matches files under a directory
  Dir(PathBuf),
  /// Matches files using a glob pattern
  Glob(glob::Pattern),
}

/// File collector for pattern matching and directory traversal.
///
/// The `FileCollector` handles:
/// - Building pattern matchers from user-provided patterns
/// - Traversing directories to collect files
/// - Path normalization and resolution
pub struct FileCollector {
  /// Root of the workspace
  workspace_root: PathBuf,
}

impl FileCollector {
  /// Creates a new FileCollector with the specified workspace root.
  ///
  /// # Parameters
  ///
  /// * `workspace_root` - The root directory of the workspace
  pub const fn new(workspace_root: PathBuf) -> Self {
    Self { workspace_root }
  }

  /// Builds pattern matchers from user-provided patterns.
  ///
  /// # Parameters
  ///
  /// * `patterns` - A slice of pattern strings (file paths, directory paths, or globs)
  /// * `current_dir` - The current working directory
  ///
  /// # Returns
  ///
  /// A vector of PatternMatcher instances.
  pub fn build_pattern_matchers(&self, patterns: &[String], current_dir: &Path) -> Result<Vec<PatternMatcher>> {
    build_pattern_matchers(patterns, current_dir, &self.workspace_root)
  }

  /// Checks if a path matches any of the given pattern matchers.
  ///
  /// # Parameters
  ///
  /// * `path` - The path to check
  /// * `matchers` - The pattern matchers to check against
  ///
  /// # Returns
  ///
  /// `true` if the path matches any pattern, `false` otherwise.
  pub fn matches_any_pattern(&self, path: &Path, matchers: &[PatternMatcher]) -> bool {
    matches_any_pattern(path, matchers)
  }

  /// Traverses a directory recursively and collects all files.
  ///
  /// # Parameters
  ///
  /// * `dir` - The directory to traverse
  ///
  /// # Returns
  ///
  /// A vector of file paths found in the directory.
  pub fn traverse_directory(&self, dir: &Path) -> Result<Vec<PathBuf>> {
    // Pre-allocate vectors for better performance
    let mut all_files = Vec::with_capacity(1000);

    // Directory traversal with optimized memory usage
    let mut dirs_to_process = std::collections::VecDeque::with_capacity(100);
    dirs_to_process.push_back(dir.to_path_buf());

    // Process directories in batches for better performance
    debug!("Scanning directory: {}", dir.display());
    let start_time = std::time::Instant::now();

    while let Some(current_dir) = dirs_to_process.pop_front() {
      let read_dir_result = std::fs::read_dir(&current_dir);
      if let Err(e) = read_dir_result {
        eprintln!("Error reading directory {}: {}", current_dir.display(), e);
        continue;
      }

      let entries = read_dir_result.expect("Valid read_dir");
      for entry in entries {
        let Ok(entry) = entry else {
          continue;
        };
        let path = entry.path();

        // Prefer cached dirent file type to avoid extra syscalls where possible.
        if let Ok(file_type) = entry.file_type() {
          if file_type.is_dir() {
            dirs_to_process.push_back(path);
          } else if file_type.is_file() {
            all_files.push(path);
          }
        }
      }
    }

    debug!(
      "Found {} files in {}ms",
      all_files.len(),
      start_time.elapsed().as_millis()
    );

    Ok(all_files)
  }
}

/// Builds pattern matchers from user-provided patterns.
pub fn build_pattern_matchers(
  patterns: &[String],
  current_dir: &Path,
  workspace_root: &Path,
) -> Result<Vec<PatternMatcher>> {
  if patterns.is_empty() {
    return Ok(Vec::new());
  }

  let mut matchers = Vec::with_capacity(patterns.len());
  for pattern in patterns {
    let raw_path = PathBuf::from(pattern);
    if raw_path.exists() {
      let abs_path = if raw_path.is_absolute() {
        Cow::Borrowed(raw_path.as_path())
      } else {
        Cow::Owned(current_dir.join(&raw_path))
      };
      let normalized = normalize_relative_path(&abs_path, workspace_root);
      // Collapse any remaining .. segments so paths like src/nested/../other become
      // src/other
      let normalized = PathBuf::from(normalize_path_string(&normalized.to_string_lossy().replace('\\', "/")));
      if raw_path.is_dir() {
        if normalized.as_os_str() == "." {
          matchers.push(PatternMatcher::Any);
        } else {
          matchers.push(PatternMatcher::Dir(normalized));
        }
      } else if raw_path.is_file() {
        matchers.push(PatternMatcher::File(normalized));
      }
    } else {
      let mut glob_source = pattern.as_str().to_string();
      if raw_path.is_absolute() {
        if let Ok(rel_path) = raw_path.strip_prefix(workspace_root) {
          glob_source = rel_path.to_string_lossy().replace("\\", "/");
        }
      } else {
        if let Ok(workspace_relative_cwd) = current_dir.strip_prefix(workspace_root)
          && !workspace_relative_cwd.as_os_str().is_empty()
          && workspace_relative_cwd.as_os_str() != "."
        {
          let cwd_prefix = workspace_relative_cwd.to_string_lossy().replace("\\", "/");
          glob_source = normalize_path_string(&format!("{}/{}", cwd_prefix, glob_source));
        }
      }
      let glob_pattern =
        glob::Pattern::new(&glob_source).with_context(|| format!("Invalid glob pattern: {}", pattern))?;
      matchers.push(PatternMatcher::Glob(glob_pattern));
    }
  }

  Ok(matchers)
}

/// Converts a potentially relative path to an absolute path.
///
/// # Parameters
///
/// * `path` - The path to absolutize
///
/// # Returns
///
/// The absolute path.
pub fn absolutize_path(path: &Path) -> Result<PathBuf> {
  if path.is_absolute() {
    Ok(path.to_path_buf())
  } else {
    let current_dir = std::env::current_dir().with_context(|| "Failed to get current directory")?;
    Ok(current_dir.join(path))
  }
}

/// Checks if a path matches any of the given pattern matchers.
pub fn matches_any_pattern(path: &Path, matchers: &[PatternMatcher]) -> bool {
  if matchers.is_empty() {
    return true;
  }

  matchers.iter().any(|matcher| match matcher {
    PatternMatcher::Any => true,
    PatternMatcher::File(file_path) => path == file_path,
    PatternMatcher::Dir(dir_path) => path.starts_with(dir_path),
    PatternMatcher::Glob(pattern) => pattern.matches_path(path),
  })
}

/// Normalizes a path to be relative to a given directory.
///
/// # Parameters
///
/// * `path` - The path to normalize
/// * `current_dir` - The directory to make the path relative to
///
/// # Returns
///
/// The normalized relative path.
pub fn normalize_relative_path(path: &Path, current_dir: &Path) -> PathBuf {
  if path.is_absolute() {
    if let Ok(stripped) = path.strip_prefix(current_dir) {
      return stripped.to_path_buf();
    }

    if let Some(rel_path) = pathdiff::diff_paths(path, current_dir) {
      return rel_path;
    }
  }

  let mut normalized = PathBuf::new();
  for component in path.components() {
    if matches!(component, std::path::Component::CurDir) {
      continue;
    }
    normalized.push(component.as_os_str());
  }

  if normalized.as_os_str().is_empty() {
    PathBuf::from(".")
  } else {
    normalized
  }
}

/// Normalizes a path string by resolving `..` segments.
///
/// This is useful for glob patterns where we can't use PathBuf directly
/// since they may contain wildcards. The function resolves `..` by removing
/// the preceding path component when possible.
///
/// # Examples
/// - `subdir/../other/**/*.rs` -> `other/**/*.rs`
/// - `a/b/../../c/*.rs` -> `c/*.rs`
/// - `../other/*.rs` -> `../other/*.rs` (can't resolve, keeps as-is)
pub fn normalize_path_string(path: &str) -> String {
  let mut components: Vec<&str> = Vec::new();

  for segment in path.split('/') {
    if segment == ".." {
      // Pop the last component if it exists and isn't ".."
      if let Some(last) = components.last()
        && *last != ".."
        && !last.is_empty()
      {
        components.pop();
        continue;
      }
      components.push(segment);
    } else if segment == "." {
      // Skip current directory markers
      continue;
    } else {
      components.push(segment);
    }
  }

  components.join("/")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_normalize_path_string_simple() {
    assert_eq!(normalize_path_string("a/b/c"), "a/b/c");
  }

  #[test]
  fn test_normalize_path_string_with_parent() {
    assert_eq!(normalize_path_string("a/b/../c"), "a/c");
  }

  #[test]
  fn test_normalize_path_string_with_current() {
    assert_eq!(normalize_path_string("a/./b/c"), "a/b/c");
  }

  #[test]
  fn test_normalize_path_string_unresolvable() {
    assert_eq!(normalize_path_string("../a/b"), "../a/b");
  }

  #[test]
  fn test_absolutize_path_already_absolute() {
    let path = PathBuf::from("/absolute/path");
    let result = absolutize_path(&path).unwrap();
    assert_eq!(result, path);
  }

  #[test]
  fn test_matches_any_pattern_empty() {
    let matchers: Vec<PatternMatcher> = vec![];
    assert!(matches_any_pattern(Path::new("any/path"), &matchers));
  }

  #[test]
  fn test_matches_any_pattern_any() {
    let matchers = vec![PatternMatcher::Any];
    assert!(matches_any_pattern(Path::new("any/path"), &matchers));
  }

  #[test]
  fn test_matches_any_pattern_file() {
    let matchers = vec![PatternMatcher::File(PathBuf::from("src/main.rs"))];
    assert!(matches_any_pattern(Path::new("src/main.rs"), &matchers));
    assert!(!matches_any_pattern(Path::new("src/lib.rs"), &matchers));
  }

  #[test]
  fn test_matches_any_pattern_dir() {
    let matchers = vec![PatternMatcher::Dir(PathBuf::from("src"))];
    assert!(matches_any_pattern(Path::new("src/main.rs"), &matchers));
    assert!(matches_any_pattern(Path::new("src/lib.rs"), &matchers));
    assert!(!matches_any_pattern(Path::new("tests/test.rs"), &matchers));
  }
}

//! # Ignore Module
//!
//! This module contains functionality for handling .licenseignore files and
//! determining which files should be ignored during license processing.
//!
//! It supports:
//! - .licenseignore files in directories (using gitignore-style pattern matching)
//! - A global ignore file specified by the GLOBAL_LICENSE_IGNORE environment variable
//! - Command-line ignore patterns

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::{env, fs};

use anyhow::{Context, Result};
use glob::Pattern;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use tracing::debug;

/// Manager for handling ignore patterns from various sources.
///
/// This struct combines ignore patterns from:
/// - Command-line arguments
/// - .licenseignore files in directories
/// - A global ignore file specified by the GLOBAL_LICENSE_IGNORE environment variable
///
/// # Examples
///
/// ```rust,no_run
/// use std::path::Path;
///
/// use edlicense::ignore::IgnoreManager;
///
/// # fn main() -> anyhow::Result<()> {
/// // Create a new ignore manager with command-line ignore patterns
/// let mut manager = IgnoreManager::new(vec!["**/*.json".to_string()])?;
///
/// // Load .licenseignore files from the current directory up to the workspace root
/// manager.load_licenseignore_files(Path::new("."), Path::new("."))?;
///
/// // Check if a file should be ignored
/// let should_ignore = manager.is_ignored(Path::new("src/config.json"));
/// assert!(should_ignore); // JSON files are ignored
/// //
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct IgnoreManager {
  /// Patterns from command-line arguments
  cli_patterns: Vec<Pattern>,

  /// Gitignore matcher for .licenseignore files
  gitignore: Option<Gitignore>,

  /// Workspace root used for .licenseignore matching
  root_dir: Option<PathBuf>,
}

impl IgnoreManager {
  /// Creates a new ignore manager with the specified command-line ignore
  /// patterns.
  ///
  /// # Parameters
  ///
  /// * `cli_patterns` - Glob patterns from command-line arguments
  ///
  /// # Returns
  ///
  /// A new `IgnoreManager` instance or an error if initialization fails.
  ///
  /// # Errors
  ///
  /// Returns an error if any of the command-line patterns are invalid.
  pub fn new(cli_patterns: Vec<String>) -> Result<Self> {
    // Compile glob patterns from command-line
    let cli_patterns = cli_patterns
      .into_iter()
      .map(|p| Pattern::new(&p))
      .collect::<Result<Vec<_>, _>>()
      .with_context(|| "Invalid glob pattern")?;

    Ok(Self {
      cli_patterns,
      gitignore: None,
      root_dir: None,
    })
  }

  /// Loads .licenseignore files from the specified directory and its parents up
  /// to the workspace root.
  ///
  /// This method also loads the global ignore file specified by the
  /// GLOBAL_LICENSE_IGNORE environment variable, if set.
  ///
  /// # Parameters
  ///
  /// * `dir` - Directory to start searching for .licenseignore files
  /// * `workspace_root` - Root of the workspace for ignore traversal
  ///
  /// # Returns
  ///
  /// `Ok(())` if successful, or an error if loading fails.
  ///
  /// # Errors
  ///
  /// Returns an error if:
  /// - The .licenseignore file exists but cannot be read
  /// - The global ignore file exists but cannot be read
  pub fn load_licenseignore_files(&mut self, dir: &Path, workspace_root: &Path) -> Result<()> {
    let root_dir = if dir.starts_with(workspace_root) {
      workspace_root
    } else {
      dir
    };
    let mut builder = GitignoreBuilder::new(root_dir);

    // Add global ignore file if specified by environment variable
    if let Ok(global_ignore_path) = env::var("GLOBAL_LICENSE_IGNORE") {
      let global_path = PathBuf::from(global_ignore_path);
      if global_path.exists() {
        let content = fs::read_to_string(&global_path)
          .with_context(|| format!("Failed to read global ignore file: {}", global_path.display()))?;

        for line in content.lines() {
          if !line.trim().is_empty() && !line.trim().starts_with('#') {
            builder
              .add_line(None, line)
              .with_context(|| format!("Failed to add line from global ignore file: {}", global_path.display()))?;
          }
        }
      }
    }

    // Find and load .licenseignore files from the current directory all the way up
    // to the workspace root. We load them starting from the root and moving down to
    // ensure proper pattern precedence
    let mut licenseignore_files = Vec::new();
    let mut current_dir = dir.to_path_buf();

    // First, collect all .licenseignore files going up to the root
    loop {
      let ignore_path = current_dir.join(".licenseignore");
      if ignore_path.exists() {
        licenseignore_files.push((current_dir.clone(), ignore_path));
      }

      // Stop at the workspace root if we're inside one.
      if current_dir == root_dir {
        break;
      }

      // Move up to parent directory
      if !current_dir.pop() {
        break;
      }
    }

    // Reverse the collection so we process from root down to the target directory
    // This ensures proper precedence where patterns in directories closer to the
    // target directory override those from higher up
    licenseignore_files.reverse();

    // Now load each .licenseignore file in order from root to target dir
    for (dir_path, ignore_path) in licenseignore_files {
      debug!("Loading .licenseignore file: {}", ignore_path.display());
      let content = fs::read_to_string(&ignore_path)
        .with_context(|| format!("Failed to read .licenseignore file: {}", ignore_path.display()))?;

      // Compute the relative path from the builder root to this .licenseignore's
      // directory. When a .licenseignore is not at the root, anchored patterns
      // (those with a leading `/` or containing an internal `/`) must be
      // rewritten to include this prefix so they match relative to the
      // .licenseignore location, not the builder root. The `ignore` crate's
      // `from` parameter is stored but not used for matching.
      let dir_relative = dir_path.strip_prefix(root_dir).ok().map(|p| p.to_path_buf());

      for line in content.lines() {
        if !line.trim().is_empty() && !line.trim().starts_with('#') {
          let adjusted = adjust_pattern_for_root(line.trim(), dir_relative.as_deref());
          builder
            .add_line(Some(dir_path.clone()), &adjusted)
            .with_context(|| format!("Failed to add line from .licenseignore file: {}", ignore_path.display()))?;
        }
      }
    }

    // Build the gitignore matcher
    let gitignore = builder.build().with_context(|| "Failed to build gitignore matcher")?;

    self.gitignore = Some(gitignore);
    self.root_dir = Some(root_dir.to_path_buf());

    Ok(())
  }

  /// Checks if a file should be ignored based on all ignore patterns.
  ///
  /// This method combines checks from:
  /// - Command-line ignore patterns
  /// - .licenseignore files
  /// - Global ignore file
  ///
  /// # Parameters
  ///
  /// * `path` - Path to the file to check
  ///
  /// # Returns
  ///
  /// `true` if the file should be ignored, `false` otherwise.
  pub fn is_ignored(&self, path: &Path) -> bool {
    // First check command-line patterns
    if self.is_ignored_by_cli_patterns(path) {
      return true;
    }

    // Then check .licenseignore patterns
    if let Some(ref gitignore) = self.gitignore
      && let Some(ref root_dir) = self.root_dir
    {
      let path = if path.is_absolute() {
        Cow::Borrowed(path)
      } else {
        Cow::Owned(root_dir.join(path))
      };
      // Get the path relative to the root directory
      if let Ok(rel_path) = path.strip_prefix(root_dir) {
        let match_result = gitignore.matched_path_or_any_parents(rel_path, false);
        if match_result.is_ignore() {
          return true;
        }
      }
    }

    false
  }

  /// Checks if a file should be ignored based on command-line ignore patterns.
  ///
  /// # Parameters
  ///
  /// * `path` - Path to the file to check
  ///
  /// # Returns
  ///
  /// `true` if the file should be ignored, `false` otherwise.
  fn is_ignored_by_cli_patterns(&self, path: &Path) -> bool {
    if let Some(path_str) = path.to_str() {
      // Convert to a relative path string for matching
      let path_str = path_str.replace("\\", "/"); // Normalize for Windows paths

      // Get the file name and parent directories for more targeted matching
      let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

      // Extract the last few components of the path for matching
      // This helps with patterns like "vendor/" that should match regardless of the
      // full path
      let components: Vec<_> = path.components().collect();
      let mut partial_paths = Vec::new();

      // Build partial paths from the components
      for i in 0..components.len() {
        let partial_path = components[i..].iter().fold(String::new(), |mut acc, c| {
          if !acc.is_empty() {
            acc.push('/');
          }
          acc.push_str(c.as_os_str().to_str().unwrap_or(""));
          acc
        });
        partial_paths.push(partial_path);
      }

      for pattern in &self.cli_patterns {
        let pattern_str = pattern.as_str();

        // Special handling for directory patterns (ending with /)
        if let Some(dir_pattern) = pattern_str.strip_suffix('/') {
          // Check if any partial path matches the directory pattern
          for partial_path in &partial_paths {
            if partial_path.starts_with(dir_pattern)
              && (partial_path.len() == dir_pattern.len() || partial_path[dir_pattern.len()..].starts_with('/'))
            {
              return true;
            }
          }
        }

        // Try matching the pattern against the path
        if pattern.matches(&path_str) {
          return true;
        }

        // Try matching against file name
        if pattern.matches(file_name) {
          return true;
        }

        // Try matching against partial paths
        for partial_path in &partial_paths {
          if pattern.matches(partial_path) {
            return true;
          }
        }

        // Special handling for directory patterns without trailing slash
        // This handles patterns like "vendor" or "vendor/**"
        if !pattern_str.contains('*') && !pattern_str.contains('?') && !pattern_str.ends_with('/') {
          // Check if any partial path starts with the pattern
          for partial_path in &partial_paths {
            if partial_path.starts_with(pattern_str)
              && (partial_path.len() == pattern_str.len() || partial_path[pattern_str.len()..].starts_with('/'))
            {
              return true;
            }
          }
        }
      }
    }

    false
  }
}

/// Adjusts a `.licenseignore` pattern so that it is correctly anchored relative
/// to the builder root when the `.licenseignore` file lives in a subdirectory.
///
/// The `ignore` crate's `GitignoreBuilder` matches all patterns relative to its
/// root directory.  When a `.licenseignore` is located in a subdirectory (e.g.
/// `packages/app/.licenseignore`), anchored patterns like `/snapshots/` must be
/// rewritten to `packages/app/snapshots/` so that the match occurs at the
/// correct level.
///
/// The gitignore spec says:
/// - A leading `/` anchors the pattern to the directory containing the gitignore.
/// - A pattern with a `/` in the middle (not leading or trailing) is also anchored.
/// - A pattern with no `/` (or only a trailing `/`) matches at any depth and gets a `**/` prefix from the crate.
///
/// # Arguments
///
/// * `pattern` – A single trimmed, non-empty, non-comment line from a `.licenseignore` file.
/// * `dir_relative` – The relative path from the builder root to the directory containing the `.licenseignore` file.
///   `None` or an empty path means the file is at the root and no adjustment is needed.
fn adjust_pattern_for_root(pattern: &str, dir_relative: Option<&Path>) -> String {
  let dir_relative = match dir_relative {
    Some(p) if p != Path::new("") => p,
    _ => return pattern.to_string(), // Already at root, no adjustment needed
  };

  let dir_prefix = dir_relative.to_string_lossy();

  // Handle negation prefix: strip `!`, adjust the inner pattern, re-add `!`.
  if let Some(inner) = pattern.strip_prefix('!') {
    let adjusted_inner = adjust_pattern_for_root(inner, Some(dir_relative));
    return format!("!{adjusted_inner}");
  }

  // Handle escaped leading characters (`\!`, `\#`): pass through unchanged
  // since they are rare and the escape already disables special handling.
  if pattern.starts_with("\\!") || pattern.starts_with("\\#") {
    // These are literal patterns; apply the same anchoring logic to the
    // unescaped part but keep the escape.
    return pattern.to_string();
  }

  // Determine whether the pattern is anchored.
  // In gitignore semantics a pattern is anchored when:
  //   1. It starts with `/`   – explicitly anchored
  //   2. It contains a `/` in positions other than the very end – implicitly anchored
  // Unanchored patterns (e.g. `*.json`, `vendor/`) match anywhere in the tree
  // and do not need adjustment.

  if let Some(rest) = pattern.strip_prefix('/') {
    // Explicitly anchored: `/snapshots/` → `dir_prefix/snapshots/`
    return format!("/{dir_prefix}/{rest}");
  }

  // Check for an internal `/` (not counting a single trailing `/`).
  let without_trailing = pattern.strip_suffix('/').unwrap_or(pattern);
  if without_trailing.contains('/') {
    // Implicitly anchored: `tests/fixtures/` → `dir_prefix/tests/fixtures/`
    return format!("{dir_prefix}/{pattern}");
  }

  // Unanchored pattern (e.g. `*.json`, `vendor/`, `*.generated.ts`).
  // The ignore crate adds `**/` automatically – no adjustment needed.
  pattern.to_string()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_adjust_pattern_no_relative_dir() {
    // At root, patterns are unchanged
    assert_eq!(adjust_pattern_for_root("/snapshots/", None), "/snapshots/");
    assert_eq!(
      adjust_pattern_for_root("/snapshots/", Some(Path::new(""))),
      "/snapshots/"
    );
    assert_eq!(adjust_pattern_for_root("*.json", None), "*.json");
  }

  #[test]
  fn test_adjust_pattern_anchored_leading_slash() {
    let dir = Some(Path::new("myproject"));
    assert_eq!(adjust_pattern_for_root("/snapshots/", dir), "/myproject/snapshots/");
    assert_eq!(adjust_pattern_for_root("/build", dir), "/myproject/build");
  }

  #[test]
  fn test_adjust_pattern_anchored_internal_slash() {
    let dir = Some(Path::new("myproject"));
    assert_eq!(
      adjust_pattern_for_root("tests/fixtures/", dir),
      "myproject/tests/fixtures/"
    );
    assert_eq!(adjust_pattern_for_root("src/generated", dir), "myproject/src/generated");
  }

  #[test]
  fn test_adjust_pattern_unanchored_unchanged() {
    let dir = Some(Path::new("myproject"));
    // Unanchored: no leading `/`, no internal `/`
    assert_eq!(adjust_pattern_for_root("*.json", dir), "*.json");
    assert_eq!(adjust_pattern_for_root("*.generated.ts", dir), "*.generated.ts");
    assert_eq!(adjust_pattern_for_root("vendor/", dir), "vendor/");
    assert_eq!(adjust_pattern_for_root("*.log", dir), "*.log");
  }

  #[test]
  fn test_adjust_pattern_negation() {
    let dir = Some(Path::new("myproject"));
    assert_eq!(adjust_pattern_for_root("!important.json", dir), "!important.json");
    assert_eq!(
      adjust_pattern_for_root("!/snapshots/keep.rs", dir),
      "!/myproject/snapshots/keep.rs"
    );
  }

  #[test]
  fn test_adjust_pattern_nested_dir_relative() {
    let dir = Some(Path::new("packages/app"));
    assert_eq!(adjust_pattern_for_root("/snapshots/", dir), "/packages/app/snapshots/");
    assert_eq!(
      adjust_pattern_for_root("tests/fixtures/", dir),
      "packages/app/tests/fixtures/"
    );
    assert_eq!(adjust_pattern_for_root("*.json", dir), "*.json");
  }
}

//! # Ignore Module
//!
//! This module contains functionality for handling .licenseignore files and
//! determining which files should be ignored during license processing.
//!
//! It supports:
//! - .licenseignore files in directories (using gitignore-style pattern
//!   matching)
//! - A global ignore file specified by the GLOBAL_LICENSE_IGNORE environment
//!   variable
//! - Command-line ignore patterns

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::{env, fs};

use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::gitignore::{Gitignore, GitignoreBuilder};

use crate::verbose_log;

/// Manager for handling ignore patterns from various sources.
///
/// This struct combines ignore patterns from:
/// - Command-line arguments
/// - .licenseignore files in directories
/// - A global ignore file specified by the GLOBAL_LICENSE_IGNORE environment
///   variable
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
  /// Pre-compiled glob set from command-line arguments for zero-allocation
  /// matching
  cli_glob_set: GlobSet,

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
    // Build a GlobSet from command-line patterns for zero-allocation matching
    let mut builder = GlobSetBuilder::new();

    for pattern in cli_patterns {
      // Normalize pattern: convert backslashes to forward slashes
      let pattern = pattern.replace('\\', "/");

      // Helper to add a pattern to the builder
      let add_pattern = |b: &mut GlobSetBuilder, p: &str| -> Result<()> {
        b.add(Glob::new(p).with_context(|| format!("Invalid glob pattern: {}", p))?);
        Ok(())
      };

      // Handle directory patterns (ending with /)
      if let Some(dir_pattern) = pattern.strip_suffix('/') {
        // Add both the exact directory match and recursive match
        add_pattern(&mut builder, dir_pattern)?;
        add_pattern(&mut builder, &format!("{}/**", dir_pattern))?;
        add_pattern(&mut builder, &format!("**/{}/**", dir_pattern))?;
        add_pattern(&mut builder, &format!("**/{}", dir_pattern))?;
      } else if !pattern.contains('*') && !pattern.contains('?') {
        // Plain name without wildcards - treat as potential directory or file match
        add_pattern(&mut builder, &pattern)?;
        add_pattern(&mut builder, &format!("**/{}", pattern))?;
        add_pattern(&mut builder, &format!("{}/**", pattern))?;
        add_pattern(&mut builder, &format!("**/{}/**", pattern))?;
      } else {
        // Regular glob pattern with wildcards
        add_pattern(&mut builder, &pattern)?;

        // Also add **/ prefix to match pattern anywhere in path (for absolute paths)
        if !pattern.starts_with("**/") {
          add_pattern(&mut builder, &format!("**/{}", pattern))?;
        }
      }
    }

    let cli_glob_set = builder.build().with_context(|| "Failed to build glob set")?;

    Ok(Self {
      cli_glob_set,
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
        verbose_log!("Loading global ignore file: {}", global_path.display());
        let content = fs::read_to_string(&global_path)
          .with_context(|| format!("Failed to read global ignore file: {}", global_path.display()))?;

        for line in content.lines() {
          if !line.trim().is_empty() && !line.trim().starts_with('#') {
            builder
              .add_line(None, line)
              .with_context(|| format!("Failed to add line from global ignore file: {}", global_path.display()))?;
          }
        }
      } else {
        verbose_log!("Global ignore file not found: {}", global_path.display());
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
      verbose_log!("Loading .licenseignore file: {}", ignore_path.display());
      let content = fs::read_to_string(&ignore_path)
        .with_context(|| format!("Failed to read .licenseignore file: {}", ignore_path.display()))?;

      for line in content.lines() {
        if !line.trim().is_empty() && !line.trim().starts_with('#') {
          builder
            .add_line(Some(dir_path.clone()), line)
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
          verbose_log!("Skipping: {} (matches .licenseignore pattern)", path.display());
          return true;
        }
      }
    }

    false
  }

  /// Checks if a file should be ignored based on command-line ignore patterns.
  ///
  /// This uses a pre-compiled GlobSet for zero-allocation matching.
  ///
  /// # Parameters
  ///
  /// * `path` - Path to the file to check
  ///
  /// # Returns
  ///
  /// `true` if the file should be ignored, `false` otherwise.
  fn is_ignored_by_cli_patterns(&self, path: &Path) -> bool {
    if self.cli_glob_set.is_match(path) {
      verbose_log!("Skipping: {} (matches CLI ignore pattern)", path.display());
      return true;
    }
    false
  }
}

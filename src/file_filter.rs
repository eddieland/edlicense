//! # File Filter Module
//!
//! This module contains components for filtering files based on various
//! criteria such as ignore patterns, git tracking status, and change status.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::ignore::IgnoreManager;
use crate::{git, verbose_log};

/// Result of a file filtering operation.
pub struct FilterResult {
  /// Whether the file should be processed
  pub should_process: bool,
  /// Reason why the file should not be processed (if any)
  pub reason: Option<String>,
}

impl FilterResult {
  /// Creates a new FilterResult indicating the file should be processed.
  pub const fn process() -> Self {
    Self {
      should_process: true,
      reason: None,
    }
  }

  /// Creates a new FilterResult indicating the file should be skipped.
  pub fn skip(reason: impl Into<String>) -> Self {
    Self {
      should_process: false,
      reason: Some(reason.into()),
    }
  }
}

/// Trait for components that filter files based on certain criteria.
pub trait FileFilter: Send + Sync {
  /// Determines whether a file should be processed.
  ///
  /// # Parameters
  ///
  /// * `path` - The path to the file to check
  ///
  /// # Returns
  ///
  /// A `FilterResult` indicating whether the file should be processed and why
  /// not if applicable.
  fn should_process(&self, path: &Path) -> Result<FilterResult>;
}

/// Filter that excludes files matching ignore patterns.
pub struct IgnoreFilter {
  ignore_manager: IgnoreManager,
}

impl IgnoreFilter {
  /// Creates a new IgnoreFilter with the given IgnoreManager.
  #[allow(dead_code)]
  pub const fn new(ignore_manager: IgnoreManager) -> Self {
    Self { ignore_manager }
  }

  /// Creates a new IgnoreFilter from a list of ignore patterns.
  pub fn from_patterns(patterns: Vec<String>) -> Result<Self> {
    let ignore_manager = IgnoreManager::new(patterns)?;
    Ok(Self { ignore_manager })
  }

  /// Updates the ignore manager with .licenseignore files from a directory.
  #[allow(dead_code)]
  pub fn load_licenseignore_files(&mut self, dir: &Path) -> Result<()> {
    self.ignore_manager.load_licenseignore_files(dir)
  }

  /// Creates a new IgnoreFilter with updated ignore patterns from a directory.
  #[allow(dead_code)]
  pub fn with_licenseignore_files(&self, dir: &Path) -> Result<Self> {
    let mut ignore_manager = self.ignore_manager.clone();
    ignore_manager.load_licenseignore_files(dir)?;
    Ok(Self { ignore_manager })
  }
}

impl FileFilter for IgnoreFilter {
  fn should_process(&self, path: &Path) -> Result<FilterResult> {
    if self.ignore_manager.is_ignored(path) {
      verbose_log!("Skipping: {} (matches ignore pattern)", path.display());
      Ok(FilterResult::skip("Matches ignore pattern"))
    } else {
      Ok(FilterResult::process())
    }
  }
}

/// Filter that excludes files not tracked by git.
pub struct GitFilter {
  git_tracked_files: HashSet<PathBuf>,
}

impl GitFilter {
  /// Creates a new GitFilter with the given set of git-tracked files.
  #[allow(dead_code)]
  pub const fn new(git_tracked_files: HashSet<PathBuf>) -> Self {
    Self { git_tracked_files }
  }

  /// Creates a new GitFilter by querying git for tracked files.
  pub fn from_git() -> Result<Self> {
    let git_tracked_files = git::get_git_tracked_files()?;
    Ok(Self { git_tracked_files })
  }
}

impl FileFilter for GitFilter {
  fn should_process(&self, path: &Path) -> Result<FilterResult> {
    // Check if the file is in the tracked files list
    let is_tracked = self.git_tracked_files.iter().any(|tracked_path| {
      // Convert both paths to strings for comparison
      let tracked_str = tracked_path.to_string_lossy().to_string();
      let path_str = path.to_string_lossy().to_string();

      // Check if the path contains the tracked path or vice versa
      tracked_str.contains(&path_str) || path_str.contains(&tracked_str)
    });

    if !is_tracked {
      verbose_log!("Skipping: {} (not tracked by git)", path.display());
      Ok(FilterResult::skip("Not tracked by git".to_string()))
    } else {
      verbose_log!("Processing: {} (tracked by git)", path.display());
      Ok(FilterResult::process())
    }
  }
}

/// Filter that excludes files that haven't changed relative to a git reference.
pub struct RatchetFilter {
  changed_files: HashSet<PathBuf>,
}

impl RatchetFilter {
  /// Creates a new RatchetFilter with the given set of changed files.
  #[allow(dead_code)]
  pub const fn new(changed_files: HashSet<PathBuf>) -> Self {
    Self { changed_files }
  }

  /// Creates a new RatchetFilter by querying git for files changed relative to
  /// a reference.
  pub fn from_reference(reference: &str) -> Result<Self> {
    let changed_files = git::get_changed_files(reference)?;
    Ok(Self { changed_files })
  }
}

impl FileFilter for RatchetFilter {
  fn should_process(&self, path: &Path) -> Result<FilterResult> {
    // Debug print to see what paths we're comparing
    verbose_log!(
      "Checking if file should be processed in ratchet mode: {}",
      path.display()
    );

    // Try multiple comparison methods
    let mut is_changed = false;

    // Method 1: Direct path comparison
    if self.changed_files.contains(path) {
      verbose_log!("File {} matched exactly in changed files list", path.display());
      is_changed = true;
    }
    // Method 2: Filename-only comparison
    else if let Some(filename) = path.file_name() {
      let filename_str = filename.to_string_lossy();
      for changed_path in &self.changed_files {
        if let Some(changed_filename) = changed_path.file_name()
          && changed_filename.to_string_lossy() == filename_str
        {
          verbose_log!(
            "File {} matched by filename with {}",
            path.display(),
            changed_path.display()
          );
          is_changed = true;
          break;
        }
      }
    }

    // Method 3: Path canonicalization (handles .. and other path components)
    if !is_changed && let Ok(canonical_path) = std::fs::canonicalize(path) {
      for changed_path in &self.changed_files {
        if let Ok(canonical_changed) = std::fs::canonicalize(changed_path)
          && canonical_path == canonical_changed
        {
          verbose_log!(
            "File {} matched by canonical path with {}",
            path.display(),
            changed_path.display()
          );
          is_changed = true;
          break;
        }
      }
    }

    if !is_changed {
      verbose_log!("Skipping: {} (unchanged in ratchet mode)", path.display());
      Ok(FilterResult::skip("Unchanged in ratchet mode".to_string()))
    } else {
      verbose_log!("Processing: {} (changed in ratchet mode)", path.display());
      Ok(FilterResult::process())
    }
  }
}

/// Filter that combines multiple filters.
pub struct CompositeFilter {
  filters: Vec<Box<dyn FileFilter>>,
}

impl CompositeFilter {
  /// Creates a new CompositeFilter with the given filters.
  pub fn new(filters: Vec<Box<dyn FileFilter>>) -> Self {
    Self { filters }
  }

  /// Adds a filter to this CompositeFilter.
  #[allow(dead_code)]
  pub fn add_filter(&mut self, filter: Box<dyn FileFilter>) {
    self.filters.push(filter);
  }
}

impl FileFilter for CompositeFilter {
  fn should_process(&self, path: &Path) -> Result<FilterResult> {
    for filter in &self.filters {
      let result = filter.should_process(path)?;
      if !result.should_process {
        return Ok(result);
      }
    }
    Ok(FilterResult::process())
  }
}

/// Constructs a CompositeFilter from common filter options.
///
/// # Parameters
///
/// * `ignore_patterns` - Glob patterns for files to ignore
/// * `git_only` - Whether to only process files tracked by git
/// * `ratchet_reference` - Git reference for ratchet mode
///
/// # Returns
///
/// A new CompositeFilter with the specified filters.
pub fn create_default_filter(
  ignore_patterns: Vec<String>,
  git_only: bool,
  ratchet_reference: Option<String>,
) -> Result<CompositeFilter> {
  let mut filters: Vec<Box<dyn FileFilter>> = Vec::new();

  // Add ignore filter
  filters.push(Box::new(IgnoreFilter::from_patterns(ignore_patterns)?));

  // Add git filter if needed
  if git_only && git::is_git_repository() {
    verbose_log!("Git-only mode enabled, getting tracked files");
    filters.push(Box::new(GitFilter::from_git()?));
  }

  // Add ratchet filter if needed
  if let Some(reference) = ratchet_reference {
    filters.push(Box::new(RatchetFilter::from_reference(&reference)?));
  }

  Ok(CompositeFilter::new(filters))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_ignore_filter() {
    let patterns = vec!["*.bak".to_string(), "tmp/*".to_string()];
    let filter = IgnoreFilter::from_patterns(patterns).unwrap();

    // Should process regular file
    let result = filter.should_process(Path::new("src/main.rs")).unwrap();
    assert!(result.should_process);

    // Should not process ignored file
    let result = filter.should_process(Path::new("src/main.rs.bak")).unwrap();
    assert!(!result.should_process);
    assert!(result.reason.is_some());
  }

  #[test]
  fn test_composite_filter() {
    let mut composite = CompositeFilter::new(Vec::new());

    // Create a mock filter that only processes files with "pass" in their name
    struct MockFilter;
    impl FileFilter for MockFilter {
      fn should_process(&self, path: &Path) -> Result<FilterResult> {
        let path_str = path.to_string_lossy();
        if path_str.contains("pass") {
          Ok(FilterResult::process())
        } else {
          Ok(FilterResult::skip("Not a pass file".to_string()))
        }
      }
    }

    composite.add_filter(Box::new(MockFilter));

    // Should process file with "pass" in name
    let result = composite.should_process(Path::new("src/pass_test.rs")).unwrap();
    assert!(result.should_process);

    // Should not process other files
    let result = composite.should_process(Path::new("src/fail_test.rs")).unwrap();
    assert!(!result.should_process);
  }
}

//! # File Filter Module
//!
//! This module contains components for filtering files based on various
//! criteria such as ignore patterns, git tracking status, and change status.

use std::path::Path;

use anyhow::Result;

use crate::ignore::IgnoreManager;
use crate::verbose_log;

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
/// * `git_only` - Whether to only process files tracked by git (handled
///   upstream)
/// * `ratchet_reference` - Git reference for ratchet mode (handled upstream)
///
/// # Returns
///
/// A new CompositeFilter with the specified filters.
pub fn create_default_filter(ignore_patterns: Vec<String>) -> Result<CompositeFilter> {
  let filters: Vec<Box<dyn FileFilter>> = vec![Box::new(IgnoreFilter::from_patterns(ignore_patterns)?)];
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

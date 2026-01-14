//! # File Filter Module
//!
//! This module contains components for filtering files based on various
//! criteria such as ignore patterns, git tracking status, and change status.

use std::collections::HashSet;
use std::path::Path;

use anyhow::Result;

use crate::config::ExtensionConfig;
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
  pub fn load_licenseignore_files(&mut self, dir: &Path, workspace_root: &Path) -> Result<()> {
    self.ignore_manager.load_licenseignore_files(dir, workspace_root)
  }

  /// Creates a new IgnoreFilter with updated ignore patterns from a directory.
  #[allow(dead_code)]
  pub fn with_licenseignore_files(&self, dir: &Path, workspace_root: &Path) -> Result<Self> {
    let mut ignore_manager = self.ignore_manager.clone();
    ignore_manager.load_licenseignore_files(dir, workspace_root)?;
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

/// Filter that includes or excludes files based on their extension.
///
/// This filter implements two modes:
/// - Include mode: Only files with extensions in the include list are processed
/// - Exclude mode: Files with extensions in the exclude list are skipped
///
/// If both include and exclude are specified, include takes precedence.
pub struct ExtensionFilter {
  /// If Some, only process files with these extensions (lowercase).
  include: Option<HashSet<String>>,
  /// Extensions to exclude from processing (lowercase).
  exclude: HashSet<String>,
}

impl ExtensionFilter {
  /// Creates a new ExtensionFilter from an ExtensionConfig.
  ///
  /// Extensions are normalized to lowercase for case-insensitive matching.
  pub fn new(config: &ExtensionConfig) -> Self {
    let include = config
      .include
      .as_ref()
      .map(|exts| exts.iter().map(|e| e.to_lowercase()).collect::<HashSet<String>>());

    let exclude = config
      .exclude
      .iter()
      .map(|e| e.to_lowercase())
      .collect::<HashSet<String>>();

    Self { include, exclude }
  }

  /// Creates a new ExtensionFilter from CLI arguments.
  ///
  /// This allows building a filter from the `--include-ext` and `--exclude-ext`
  /// command line options.
  pub fn from_cli(include_exts: Vec<String>, exclude_exts: Vec<String>) -> Self {
    let include = if include_exts.is_empty() {
      None
    } else {
      Some(
        include_exts
          .into_iter()
          .map(|e| e.to_lowercase())
          .collect::<HashSet<String>>(),
      )
    };

    let exclude = exclude_exts
      .into_iter()
      .map(|e| e.to_lowercase())
      .collect::<HashSet<String>>();

    Self { include, exclude }
  }

  /// Merges CLI arguments into an existing ExtensionFilter from config.
  ///
  /// CLI arguments take precedence over config file settings:
  /// - If CLI specifies any include extensions, they completely replace config
  ///   includes
  /// - If CLI specifies any exclude extensions, they are added to config
  ///   excludes
  pub fn merge_cli(&mut self, include_exts: Vec<String>, exclude_exts: Vec<String>) {
    // CLI include completely replaces config include if specified
    if !include_exts.is_empty() {
      self.include = Some(
        include_exts
          .into_iter()
          .map(|e| e.to_lowercase())
          .collect::<HashSet<String>>(),
      );
    }

    // CLI exclude adds to config exclude
    for ext in exclude_exts {
      self.exclude.insert(ext.to_lowercase());
    }
  }

  /// Returns true if this filter has any active filtering rules.
  pub fn is_active(&self) -> bool {
    self.include.is_some() || !self.exclude.is_empty()
  }

  /// Gets the extension from a path, handling compound extensions like
  /// "min.js".
  fn get_extension(path: &Path) -> Option<String> {
    // First try the standard extension
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
      let ext_lower = ext.to_lowercase();

      // Check for compound extensions (e.g., "file.min.js" should match "min.js")
      if let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        && let Some(compound_ext_start) = stem.rfind('.')
      {
        let compound_part = &stem[compound_ext_start + 1..];
        let compound_ext = format!("{}.{}", compound_part.to_lowercase(), ext_lower);
        return Some(compound_ext);
      }

      return Some(ext_lower);
    }
    None
  }
}

impl FileFilter for ExtensionFilter {
  fn should_process(&self, path: &Path) -> Result<FilterResult> {
    let ext = Self::get_extension(path);

    match (&self.include, &ext) {
      // If include list exists, file extension must be in it
      (Some(include), Some(ext)) => {
        // Check both the compound extension and simple extension
        let simple_ext = path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase());

        let is_included = include.contains(ext) || simple_ext.as_ref().map(|e| include.contains(e)).unwrap_or(false);

        if !is_included {
          verbose_log!("Skipping: {} (extension not in include list)", path.display());
          Ok(FilterResult::skip("Extension not in include list"))
        } else {
          Ok(FilterResult::process())
        }
      }
      // If include list exists but file has no extension, skip it
      (Some(_), None) => {
        verbose_log!("Skipping: {} (no extension, include list specified)", path.display());
        Ok(FilterResult::skip("No extension, include list specified"))
      }
      // No include list - check exclude list
      (None, Some(ext)) => {
        // Check both the compound extension and simple extension
        let simple_ext = path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase());

        let is_excluded =
          self.exclude.contains(ext) || simple_ext.as_ref().map(|e| self.exclude.contains(e)).unwrap_or(false);

        if is_excluded {
          verbose_log!("Skipping: {} (extension in exclude list)", path.display());
          Ok(FilterResult::skip("Extension in exclude list"))
        } else {
          Ok(FilterResult::process())
        }
      }
      // No include list and no extension - process the file
      (None, None) => Ok(FilterResult::process()),
    }
  }
}

/// Constructs an IgnoreFilter using the provided ignore patterns.
///
/// # Parameters
///
/// * `ignore_patterns` - Glob patterns for files to ignore
///
/// # Returns
///
/// A new IgnoreFilter with the specified ignore patterns.
pub fn create_default_filter(ignore_patterns: Vec<String>) -> Result<IgnoreFilter> {
  IgnoreFilter::from_patterns(ignore_patterns)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::config::ExtensionConfig;

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
  fn test_extension_filter_include_only() {
    let config = ExtensionConfig {
      include: Some(vec!["rs".to_string(), "go".to_string()]),
      exclude: Vec::new(),
    };
    let filter = ExtensionFilter::new(&config);

    // Should process .rs files
    let result = filter.should_process(Path::new("src/main.rs")).unwrap();
    assert!(result.should_process);

    // Should process .go files
    let result = filter.should_process(Path::new("src/main.go")).unwrap();
    assert!(result.should_process);

    // Should NOT process .py files
    let result = filter.should_process(Path::new("src/script.py")).unwrap();
    assert!(!result.should_process);
    assert_eq!(result.reason, Some("Extension not in include list".to_string()));

    // Should NOT process files without extension when include is specified
    let result = filter.should_process(Path::new("Makefile")).unwrap();
    assert!(!result.should_process);
    assert_eq!(result.reason, Some("No extension, include list specified".to_string()));
  }

  #[test]
  fn test_extension_filter_exclude_only() {
    let config = ExtensionConfig {
      include: None,
      exclude: vec!["min.js".to_string(), "bak".to_string()],
    };
    let filter = ExtensionFilter::new(&config);

    // Should process .rs files
    let result = filter.should_process(Path::new("src/main.rs")).unwrap();
    assert!(result.should_process);

    // Should NOT process .bak files
    let result = filter.should_process(Path::new("src/main.bak")).unwrap();
    assert!(!result.should_process);
    assert_eq!(result.reason, Some("Extension in exclude list".to_string()));

    // Should NOT process .min.js files
    let result = filter.should_process(Path::new("dist/app.min.js")).unwrap();
    assert!(!result.should_process);
    assert_eq!(result.reason, Some("Extension in exclude list".to_string()));

    // Should process files without extension
    let result = filter.should_process(Path::new("Makefile")).unwrap();
    assert!(result.should_process);
  }

  #[test]
  fn test_extension_filter_case_insensitive() {
    let config = ExtensionConfig {
      include: Some(vec!["RS".to_string()]),
      exclude: Vec::new(),
    };
    let filter = ExtensionFilter::new(&config);

    // Should process .rs files (lowercase)
    let result = filter.should_process(Path::new("src/main.rs")).unwrap();
    assert!(result.should_process);

    // Should process .RS files (uppercase)
    let result = filter.should_process(Path::new("src/main.RS")).unwrap();
    assert!(result.should_process);
  }

  #[test]
  fn test_extension_filter_empty_config() {
    let config = ExtensionConfig::default();
    let filter = ExtensionFilter::new(&config);

    // Empty filter should not be active
    assert!(!filter.is_active());

    // Should process all files
    let result = filter.should_process(Path::new("src/main.rs")).unwrap();
    assert!(result.should_process);

    let result = filter.should_process(Path::new("Makefile")).unwrap();
    assert!(result.should_process);
  }

  #[test]
  fn test_extension_filter_from_cli() {
    let filter = ExtensionFilter::from_cli(vec!["rs".to_string(), "go".to_string()], Vec::new());

    assert!(filter.is_active());

    let result = filter.should_process(Path::new("src/main.rs")).unwrap();
    assert!(result.should_process);

    let result = filter.should_process(Path::new("src/script.py")).unwrap();
    assert!(!result.should_process);
  }

  #[test]
  fn test_extension_filter_merge_cli() {
    let config = ExtensionConfig {
      include: Some(vec!["rs".to_string()]),
      exclude: vec!["bak".to_string()],
    };
    let mut filter = ExtensionFilter::new(&config);

    // Merge CLI args - include should be replaced, exclude should be added
    filter.merge_cli(vec!["go".to_string()], vec!["tmp".to_string()]);

    // Now only go files should be included (CLI replaced config)
    let result = filter.should_process(Path::new("src/main.rs")).unwrap();
    assert!(!result.should_process);

    let result = filter.should_process(Path::new("src/main.go")).unwrap();
    assert!(result.should_process);
  }

  #[test]
  fn test_extension_filter_compound_extensions() {
    let config = ExtensionConfig {
      include: None,
      exclude: vec!["pb.go".to_string(), "generated.rs".to_string()],
    };
    let filter = ExtensionFilter::new(&config);

    // Should exclude .pb.go files
    let result = filter.should_process(Path::new("src/proto.pb.go")).unwrap();
    assert!(!result.should_process);

    // Should exclude .generated.rs files
    let result = filter.should_process(Path::new("src/api.generated.rs")).unwrap();
    assert!(!result.should_process);

    // Should process regular .go files
    let result = filter.should_process(Path::new("src/main.go")).unwrap();
    assert!(result.should_process);

    // Should process regular .rs files
    let result = filter.should_process(Path::new("src/main.rs")).unwrap();
    assert!(result.should_process);
  }
}

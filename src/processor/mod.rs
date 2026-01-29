//! # Processor Module
//!
//! This module contains the core functionality for processing files and
//! directories, adding license headers, and checking for existing licenses.
//!
//! The module is organized into several submodules:
//! - [`file_io`] - File reading and writing operations
//! - [`content_transformer`] - Content transformation utilities (prefix extraction, year updates)
//! - [`file_collector`] - File collection, pattern matching, and directory traversal
//!
//! The [`Processor`] struct is the main entry point for all file operations,
//! orchestrating the submodules to provide a cohesive API.

mod content_transformer;
mod file_collector;
mod file_io;

// Re-export public types
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
pub use content_transformer::ContentTransformer;
pub use file_collector::{FileCollector, absolutize_path};
pub use file_io::FileIO;
use rayon::prelude::*;
use tracing::{debug, trace};

use crate::diff::DiffManager;
use crate::file_filter::{ExtensionFilter, FileFilter, FilterResult, IgnoreFilter, create_default_filter};
use crate::git::RatchetOptions;
use crate::ignore::IgnoreManager;
use crate::license_detection::{LicenseDetector, SimpleLicenseDetector};
use crate::report::{FileAction, FileReport};
use crate::templates::{LicenseData, TemplateManager};
use crate::{git, info_log};

/// Configuration for creating a Processor instance.
pub struct ProcessorConfig {
  pub template_manager: TemplateManager,
  pub license_data: LicenseData,
  pub workspace_root: PathBuf,
  pub workspace_is_git: bool,

  // Behavior flags
  pub check_only: bool,
  pub preserve_years: bool,
  pub git_only: bool,

  // Ratchet mode
  pub ratchet_reference: Option<String>,
  pub ratchet_committed_only: bool,

  // Optional components
  pub ignore_patterns: Vec<String>,
  pub diff_manager: Option<DiffManager>,
  pub license_detector: Option<Box<dyn LicenseDetector + Send + Sync>>,
  pub extension_filter: Option<ExtensionFilter>,
}

impl ProcessorConfig {
  /// Creates a new ProcessorConfig with required fields and sensible defaults.
  ///
  /// Use struct update syntax to override specific fields:
  /// ```ignore
  /// ProcessorConfig {
  ///     check_only: true,
  ///     ..ProcessorConfig::new(template_manager, license_data, workspace_root)
  /// }
  /// ```
  pub fn new(template_manager: TemplateManager, license_data: LicenseData, workspace_root: PathBuf) -> Self {
    Self {
      template_manager,
      license_data,
      workspace_root,
      workspace_is_git: false,
      check_only: false,
      preserve_years: false,
      git_only: false,
      ratchet_reference: None,
      ratchet_committed_only: false,
      ignore_patterns: vec![],
      diff_manager: None,
      license_detector: None,
      extension_filter: None,
    }
  }
}

/// Passthrough filter that allows all files.
struct PassthroughFilter;

impl FileFilter for PassthroughFilter {
  fn should_process(&self, _path: &Path) -> Result<FilterResult> {
    Ok(FilterResult::process())
  }
}

/// Processor for handling license operations on files.
///
/// The `Processor` is responsible for:
/// - Scanning directories recursively
/// - Identifying files that need license headers
/// - Adding or updating license headers
/// - Checking for existing licenses
/// - Handling ratchet mode (only processing changed files)
/// - Showing diffs in dry run mode
/// - Filtering files based on git repository (when git_only is enabled)
/// - Collecting report data about processed files
pub struct Processor {
  /// Root of the current workspace.
  workspace_root: PathBuf,

  /// Template manager for rendering license templates
  template_manager: TemplateManager,

  /// License data (year, etc.) for rendering templates
  license_data: LicenseData,

  /// File filter for determining which files to process
  file_filter: IgnoreFilter,

  /// Manager for handling ignore patterns (used for directory-specific ignore
  /// patterns)
  ignore_manager: IgnoreManager,

  /// Whether to only check for licenses without modifying files
  check_only: bool,

  /// Whether to preserve existing years in license headers
  preserve_years: bool,

  /// Manager for handling diff creation and rendering
  diff_manager: DiffManager,

  /// Counter for the total number of files processed
  pub files_processed: Arc<std::sync::atomic::AtomicUsize>,

  /// Collection of file reports for generating reports
  pub file_reports: Arc<Mutex<Vec<FileReport>>>,

  /// Whether to collect report data
  collect_report_data: bool,

  /// License detector for checking if files have license headers
  license_detector: Arc<Box<dyn LicenseDetector + Send + Sync>>,

  /// Cache for ignore managers to avoid redundant .licenseignore file loading
  ignore_manager_cache: Arc<Mutex<HashMap<PathBuf, IgnoreManager>>>,

  /// Whether to only process git-tracked files
  git_only: bool,

  /// Git reference for ratchet mode
  ratchet_reference: Option<String>,

  /// Options for ratchet mode (staged/unstaged file inclusion)
  ratchet_options: RatchetOptions,

  /// Optional extension filter for include/exclude based filtering
  extension_filter: Option<ExtensionFilter>,

  /// Content transformer for prefix extraction and year updates
  content_transformer: ContentTransformer,

  /// File collector for pattern matching and directory traversal
  file_collector: FileCollector,
}

impl Processor {
  /// Creates a new processor with the specified configuration.
  ///
  /// # Parameters
  ///
  /// * `config` - Configuration for the processor including template manager, license data, behavior flags, and
  ///   optional components
  ///
  /// # Returns
  ///
  /// A new `Processor` instance or an error if initialization fails.
  ///
  /// # Errors
  ///
  /// Returns an error if:
  /// - Any of the ignore patterns are invalid
  /// - Ratchet mode is enabled but the git repository cannot be accessed
  pub fn new(config: ProcessorConfig) -> Result<Self> {
    if (config.git_only || config.ratchet_reference.is_some()) && !config.workspace_is_git {
      return Err(anyhow::anyhow!(
        "Git-only or ratchet mode requires a git-backed workspace"
      ));
    }

    // Create ignore manager for base ignore patterns
    let ignore_manager = IgnoreManager::new(config.ignore_patterns.clone())?;

    // Create a composite file filter with all filtering conditions
    let file_filter = create_default_filter(config.ignore_patterns)?;

    let diff_manager = config.diff_manager.unwrap_or_else(|| DiffManager::new(false, None));

    let license_detector = config
      .license_detector
      .unwrap_or_else(|| Box::new(SimpleLicenseDetector::new()));

    // Determine ratchet options based on --ratchet-committed-only flag
    let ratchet_options = if config.ratchet_committed_only {
      RatchetOptions::committed_only()
    } else {
      RatchetOptions::default()
    };

    // Create content transformer with the current year
    let content_transformer = ContentTransformer::new(config.license_data.year.clone());

    // Create file collector
    let file_collector = FileCollector::new(config.workspace_root.clone());

    Ok(Self {
      template_manager: config.template_manager,
      license_data: config.license_data,
      file_filter,
      ignore_manager,
      check_only: config.check_only,
      preserve_years: config.preserve_years,
      diff_manager,
      workspace_root: config.workspace_root,
      files_processed: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
      file_reports: Arc::new(Mutex::new(Vec::new())),
      collect_report_data: true, // Enable report data collection by default
      license_detector: Arc::new(license_detector),
      ignore_manager_cache: Arc::new(Mutex::new(HashMap::new())),
      git_only: config.git_only,
      ratchet_reference: config.ratchet_reference,
      ratchet_options,
      extension_filter: config.extension_filter,
      content_transformer,
      file_collector,
    })
  }

  /// Checks if a file should be processed based on both the ignore filter
  /// and the optional extension filter.
  fn should_process_file(&self, path: &Path) -> Result<FilterResult> {
    // First check the ignore filter
    let filter_result = self.file_filter.should_process(path)?;
    if !filter_result.should_process {
      return Ok(filter_result);
    }

    // Then check the extension filter if present
    if let Some(ref ext_filter) = self.extension_filter {
      let ext_result = ext_filter.should_process(path)?;
      if !ext_result.should_process {
        return Ok(ext_result);
      }
    }

    Ok(FilterResult::process())
  }

  /// Processes a list of file or directory patterns.
  ///
  /// This is the main entry point for processing files. It handles:
  /// - Individual files
  /// - Directories (recursively)
  /// - Glob patterns
  ///
  /// # Parameters
  ///
  /// * `patterns` - A slice of strings representing file paths, directory paths, or glob patterns
  ///
  /// # Returns
  ///
  /// `true` if any files were missing license headers, `false` otherwise.
  /// In check-only mode, this can be used to determine if the check passed or
  /// failed.
  ///
  /// # Errors
  ///
  /// Returns an error if:
  /// - A glob pattern is invalid
  /// - Directory traversal fails
  pub fn process(&self, patterns: &[String]) -> Result<bool> {
    if self.should_use_git_list() {
      let files = self.collect_files(patterns)?;
      return self.process_collected(files);
    }

    let mut all_files = Vec::new();

    // Collect all files from patterns
    for pattern in patterns {
      let maybe_path = PathBuf::from(pattern);
      if maybe_path.is_file() {
        all_files.push(maybe_path);
      } else if maybe_path.is_dir() {
        let dir_files = self.file_collector.traverse_directory(&maybe_path)?;
        all_files.extend(dir_files);
      } else {
        // Try to use the pattern as a glob
        let entries = glob::glob(pattern).with_context(|| format!("Invalid glob pattern: {}", pattern))?;

        for entry in entries {
          match entry {
            Ok(path) => {
              if path.is_file() {
                all_files.push(path);
              } else if path.is_dir() {
                let dir_files = self.file_collector.traverse_directory(&path)?;
                all_files.extend(dir_files);
              }
            }
            Err(e) => {
              eprintln!("Error with glob pattern: {}", e);
            }
          }
        }
      }
    }

    // Deduplicate files to prevent race conditions when overlapping patterns
    // yield the same file (e.g., "src" and "src/main.rs" both specified)
    let all_files: Vec<PathBuf> = all_files.into_iter().collect::<HashSet<_>>().into_iter().collect();

    // Filter files with ignore context and process
    let files = self.filter_files_with_ignore_context(all_files)?;
    let filter_with_licenseignore = self
      .file_filter
      .with_licenseignore_files(&self.workspace_root, &self.workspace_root)?;
    self.process_files_with_filter(files, &filter_with_licenseignore)
  }

  /// Process files from a pre-collected list (git-only/ratchet mode).
  ///
  /// Use this when files have already been collected via [`collect_files`] to
  /// avoid repeating the git operation.
  pub fn process_collected(&self, files: Vec<PathBuf>) -> Result<bool> {
    let files = self.filter_files_with_ignore_context(files)?;
    let passthrough_filter = PassthroughFilter;
    self.process_files_with_filter(files, &passthrough_filter)
  }

  pub const fn should_use_git_list(&self) -> bool {
    self.git_only || self.ratchet_reference.is_some()
  }

  pub fn collect_files(&self, patterns: &[String]) -> Result<Vec<PathBuf>> {
    // Check ratchet_reference first since it's a more specific filter than
    // git_only. When both are set, ratchet should take precedence to return
    // only changed files.
    let files: Vec<PathBuf> = if let Some(reference) = &self.ratchet_reference {
      git::get_changed_files_for_workspace(&self.workspace_root, reference, &self.ratchet_options)?
        .into_iter()
        .collect()
    } else if self.git_only {
      git::get_git_tracked_files(&self.workspace_root)?.into_iter().collect()
    } else {
      return Ok(Vec::new());
    };

    if files.is_empty() {
      return Ok(Vec::new());
    }

    let current_dir = std::env::current_dir().with_context(|| "Failed to get current directory")?;
    let matchers = self.file_collector.build_pattern_matchers(patterns, &current_dir)?;

    let selected: Vec<PathBuf> = files
      .into_iter()
      .filter_map(|file| {
        let normalized = file_collector::normalize_relative_path(&file, &self.workspace_root);
        if self.file_collector.matches_any_pattern(&normalized, &matchers) {
          Some(self.workspace_root.join(&normalized))
        } else {
          None
        }
      })
      .collect();

    Ok(selected)
  }

  /// Processes a directory recursively, adding or checking license headers in
  /// all files.
  ///
  /// This method:
  /// 1. Recursively finds all files in the directory
  /// 2. Filters out files that match ignore patterns
  /// 3. Processes each file in parallel using Rayon
  ///
  /// # Parameters
  ///
  /// * `dir` - Path to the directory to process
  ///
  /// # Returns
  ///
  /// `true` if any files were missing license headers, `false` otherwise.
  ///
  /// # Errors
  ///
  /// Returns an error if directory traversal fails or if file processing fails.
  #[allow(dead_code)] // Used by library consumers and tests, not in the CLI binary
  pub fn process_directory(&self, dir: &Path) -> Result<bool> {
    // Use FileCollector to traverse the directory
    let all_files = self.file_collector.traverse_directory(dir)?;

    // Create a filter that includes .licenseignore patterns from the directory
    let filter_with_licenseignore = self.file_filter.with_licenseignore_files(dir, &self.workspace_root)?;

    self.process_files_with_filter(all_files, &filter_with_licenseignore)
  }

  /// Batch size for processing files to reduce overhead.
  const BATCH_SIZE: usize = 8;

  fn process_files_with_filter(&self, files: Vec<PathBuf>, filter: &dyn FileFilter) -> Result<bool> {
    if files.is_empty() {
      debug!("No files to process");
      return Ok(false);
    }

    let mut local_reports = Vec::with_capacity(1000);

    // Filter files using the file_filter directly
    let filter_start = std::time::Instant::now();
    let files: Vec<_> = files
      .into_iter()
      .filter(|p| {
        // Skip symlinks - use symlink_metadata to check without following
        match std::fs::symlink_metadata(p) {
          Ok(metadata) => {
            if metadata.file_type().is_symlink() {
              trace!("Skipping: {} (symlink)", p.display());
              if self.collect_report_data {
                local_reports.push(FileReport {
                  path: p.to_path_buf(),
                  has_license: false,
                  action_taken: Some(FileAction::Skipped),
                  ignored: true,
                  ignored_reason: Some("Symlink".to_string()),
                });
              }
              return false;
            }
          }
          Err(_) => {
            // Can't stat the file, skip it
            return false;
          }
        }

        // Check the passed-in filter first
        match filter.should_process(p) {
          Ok(result) => {
            if !result.should_process {
              let reason_display = result.reason.as_deref().unwrap_or("Unknown reason");
              trace!("Skipping: {} ({})", p.display(), reason_display);

              if self.collect_report_data {
                let file_report = FileReport {
                  path: p.to_path_buf(),
                  has_license: false,
                  action_taken: Some(FileAction::Skipped),
                  ignored: true,
                  ignored_reason: result.reason,
                };

                local_reports.push(file_report);
              }

              return false;
            }
          }
          Err(_) => return false,
        }

        // Then check the extension filter if present
        if let Some(ref ext_filter) = self.extension_filter {
          match ext_filter.should_process(p) {
            Ok(result) => {
              if !result.should_process {
                let reason_display = result.reason.as_deref().unwrap_or("Unknown reason");
                trace!("Skipping: {} ({})", p.display(), reason_display);

                if self.collect_report_data {
                  let file_report = FileReport {
                    path: p.to_path_buf(),
                    has_license: false,
                    action_taken: Some(FileAction::Skipped),
                    ignored: true,
                    ignored_reason: result.reason,
                  };

                  local_reports.push(file_report);
                }

                return false;
              }
            }
            Err(_) => return false,
          }
        }

        // Skip files with no defined comment style (unknown extensions)
        if !self.template_manager.can_handle_file_type(p) {
          trace!("Skipping: {} (no comment style defined for extension)", p.display());
          if self.collect_report_data {
            local_reports.push(FileReport {
              path: p.to_path_buf(),
              has_license: false,
              action_taken: Some(FileAction::Skipped),
              ignored: true,
              ignored_reason: Some("No comment style defined for extension".to_string()),
            });
          }
          return false;
        }

        true
      })
      .collect();

    debug!(
      "Filtered to {} files to process in {}ms",
      files.len(),
      filter_start.elapsed().as_millis()
    );

    if files.is_empty() {
      debug!("No files to process after filtering");

      if self.collect_report_data && !local_reports.is_empty() {
        let mut reports = self.file_reports.lock().expect("mutex poisoned");
        reports.extend(local_reports);
      }

      return Ok(false);
    }

    let files_len = files.len();
    debug!("Processing {} files with rayon", files_len);

    let process_start = std::time::Instant::now();

    // Batch processing: process files in chunks using rayon
    let batches: Vec<Vec<PathBuf>> = files.chunks(Self::BATCH_SIZE).map(|chunk| chunk.to_vec()).collect();

    let batch_count = batches.len();
    debug!(
      "Processing {} files in {} batches (batch size: {})",
      files_len,
      batch_count,
      Self::BATCH_SIZE
    );

    // Process batches in parallel using rayon
    let batch_results: Vec<(Vec<FileReport>, bool)> = batches
      .into_par_iter()
      .map(|batch| self.process_file_batch(batch))
      .collect();

    // Merge results from all batches
    let mut has_missing_license = false;
    for (batch_reports, batch_has_missing) in batch_results {
      local_reports.extend(batch_reports);
      if batch_has_missing {
        has_missing_license = true;
      }
    }

    debug!(
      "Processed {} files in {}ms",
      files_len,
      process_start.elapsed().as_millis()
    );

    if self.collect_report_data && !local_reports.is_empty() {
      let mut reports = self.file_reports.lock().expect("mutex poisoned");
      reports.extend(local_reports);
    }

    Ok(has_missing_license)
  }

  /// Process a batch of files and return collected reports and error status.
  fn process_file_batch(&self, files: Vec<PathBuf>) -> (Vec<FileReport>, bool) {
    let mut batch_reports = Vec::with_capacity(files.len());
    let mut has_missing = false;

    for path in files {
      let result = self.process_single_file(&path, &mut batch_reports);
      if let Err(e) = result {
        if self.check_only && e.to_string().contains("Missing license header") {
          has_missing = true;
        } else {
          eprintln!("Error processing {}: {}", path.display(), e);
          has_missing = true;
        }
      }
    }

    (batch_reports, has_missing)
  }

  /// Process a single file, collecting reports locally.
  fn process_single_file(&self, path: &Path, batch_reports: &mut Vec<FileReport>) -> Result<()> {
    // Increment the files processed counter
    self.files_processed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // Read file prefix
    let (prefix_bytes, prefix_content, file_len) = FileIO::read_license_check_prefix(path)?;

    let diff_requested = self.diff_manager.show_diff || self.diff_manager.save_diff_path.is_some();
    let has_license = self.has_license(&prefix_content);
    let needs_full_content = if self.check_only {
      if !has_license {
        diff_requested
      } else {
        !self.preserve_years && diff_requested
      }
    } else if has_license {
      !self.preserve_years
    } else {
      true
    };

    // Only read full content if needed
    let content = if needs_full_content && prefix_bytes.len() as u64 >= file_len {
      // We already have all the content from the prefix read
      prefix_content
    } else if needs_full_content {
      FileIO::read_full_content(path)?
    } else {
      prefix_content
    };

    if self.check_only {
      if !has_license {
        // Generate diffs if show_diff is enabled or save_diff_path is provided
        if self.diff_manager.show_diff || self.diff_manager.save_diff_path.is_some() {
          let license_text = match self.template_manager.render(&self.license_data) {
            Ok(text) => text,
            Err(e) => return Err(anyhow::anyhow!("Failed to render license template: {}", e)),
          };

          let Some(formatted_license) = self.template_manager.format_for_file_type(&license_text, path) else {
            trace!("Skipping: {} (no comment style defined for extension)", path.display());
            if self.collect_report_data {
              batch_reports.push(FileReport {
                path: path.to_path_buf(),
                has_license: false,
                action_taken: Some(FileAction::Skipped),
                ignored: true,
                ignored_reason: Some("No comment style defined for extension".to_string()),
              });
            }
            return Ok(());
          };

          let (prefix, content_without_prefix) = self.content_transformer.extract_prefix(&content);
          let new_content = format!("{}{}{}", prefix, formatted_license, content_without_prefix);

          if let Err(e) = self.diff_manager.display_diff(path, &content, &new_content) {
            eprintln!("Warning: Failed to display diff for {}: {}", path.display(), e);
          }
        }

        if self.collect_report_data {
          batch_reports.push(FileReport {
            path: path.to_path_buf(),
            has_license,
            action_taken: None,
            ignored: false,
            ignored_reason: None,
          });
        }

        return Err(anyhow::anyhow!("Missing license header"));
      } else if !self.preserve_years {
        let updated_content = self.content_transformer.update_year_in_license(&content)?;
        if updated_content != content {
          if (self.diff_manager.show_diff || self.diff_manager.save_diff_path.is_some())
            && let Err(e) = self.diff_manager.display_diff(path, &content, updated_content.as_ref())
          {
            eprintln!("Warning: Failed to display diff for {}: {}", path.display(), e);
          }

          if self.collect_report_data {
            batch_reports.push(FileReport {
              path: path.to_path_buf(),
              has_license,
              action_taken: Some(FileAction::YearUpdated),
              ignored: false,
              ignored_reason: None,
            });
          }
        } else {
          if self.collect_report_data {
            batch_reports.push(FileReport {
              path: path.to_path_buf(),
              has_license,
              action_taken: Some(FileAction::NoActionNeeded),
              ignored: false,
              ignored_reason: None,
            });
          }
        }
      } else {
        if self.collect_report_data {
          batch_reports.push(FileReport {
            path: path.to_path_buf(),
            has_license,
            action_taken: Some(FileAction::NoActionNeeded),
            ignored: false,
            ignored_reason: None,
          });
        }
      }
      return Ok(());
    }

    if has_license {
      if !self.preserve_years {
        let updated_content = self.content_transformer.update_year_in_license(&content)?;
        if updated_content != content {
          FileIO::write_file(path, updated_content.as_ref())?;
          info_log!("Updated year in: {}", path.display());

          if self.collect_report_data {
            batch_reports.push(FileReport {
              path: path.to_path_buf(),
              has_license: true,
              action_taken: Some(FileAction::YearUpdated),
              ignored: false,
              ignored_reason: None,
            });
          }
        } else {
          if self.collect_report_data {
            batch_reports.push(FileReport {
              path: path.to_path_buf(),
              has_license: true,
              action_taken: Some(FileAction::NoActionNeeded),
              ignored: false,
              ignored_reason: None,
            });
          }
        }
      } else {
        if self.collect_report_data {
          batch_reports.push(FileReport {
            path: path.to_path_buf(),
            has_license: true,
            action_taken: Some(FileAction::NoActionNeeded),
            ignored: false,
            ignored_reason: None,
          });
        }
      }
    } else {
      let license_text = match self.template_manager.render(&self.license_data) {
        Ok(text) => text,
        Err(e) => return Err(anyhow::anyhow!("Failed to render license template: {}", e)),
      };

      let Some(formatted_license) = self.template_manager.format_for_file_type(&license_text, path) else {
        trace!("Skipping: {} (no comment style defined for extension)", path.display());
        if self.collect_report_data {
          batch_reports.push(FileReport {
            path: path.to_path_buf(),
            has_license: false,
            action_taken: Some(FileAction::Skipped),
            ignored: true,
            ignored_reason: Some("No comment style defined for extension".to_string()),
          });
        }
        return Ok(());
      };

      let (prefix, content_remainder) = self.content_transformer.extract_prefix(&content);
      // For empty files, don't include the trailing blank line separator
      let license_to_use = if content_remainder.trim().is_empty() {
        formatted_license.trim_end().to_string() + "\n"
      } else {
        formatted_license
      };
      let new_content = format!("{}{}{}", prefix, license_to_use, content_remainder);

      FileIO::write_file(path, &new_content)?;
      info_log!("Added license to: {}", path.display());

      if self.collect_report_data {
        batch_reports.push(FileReport {
          path: path.to_path_buf(),
          has_license: true,
          action_taken: Some(FileAction::Added),
          ignored: false,
          ignored_reason: None,
        });
      }
    }

    Ok(())
  }

  fn filter_files_with_ignore_context(&self, files: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    let mut filtered = Vec::with_capacity(files.len());
    let mut local_reports = Vec::new();

    for path in files {
      // Skip symlinks
      match std::fs::symlink_metadata(&path) {
        Ok(metadata) => {
          if metadata.file_type().is_symlink() {
            trace!("Skipping: {} (symlink)", path.display());
            if self.collect_report_data {
              local_reports.push(FileReport {
                path: path.to_path_buf(),
                has_license: false,
                action_taken: Some(FileAction::Skipped),
                ignored: true,
                ignored_reason: Some("Symlink".to_string()),
              });
            }
            continue;
          }
        }
        Err(_) => {
          continue;
        }
      }

      let mut ignored = false;
      let absolute_path = absolutize_path(&path)?;

      if let Some(parent_dir) = absolute_path.parent()
        && parent_dir.exists()
      {
        let ignore_manager = {
          let mut cache = self.ignore_manager_cache.lock().expect("mutex poisoned");

          if let Some(cached_manager) = cache.get(parent_dir) {
            trace!("Using cached ignore manager for: {}", parent_dir.display());
            cached_manager.clone()
          } else {
            trace!("Creating new ignore manager for: {}", parent_dir.display());
            let mut new_manager = self.ignore_manager.clone();
            new_manager.load_licenseignore_files(parent_dir, &self.workspace_root)?;
            cache.insert(parent_dir.to_path_buf(), new_manager.clone());
            new_manager
          }
        };

        if ignore_manager.is_ignored(&absolute_path) {
          trace!("Skipping: {} (matches .licenseignore pattern)", path.display());
          ignored = true;

          if self.collect_report_data {
            let file_report = FileReport {
              path: path.to_path_buf(),
              has_license: false,
              action_taken: Some(FileAction::Skipped),
              ignored: true,
              ignored_reason: Some("Matches .licenseignore pattern".to_string()),
            };

            local_reports.push(file_report);
          }
        }
      }

      if !ignored {
        filtered.push(path);
      }
    }

    if self.collect_report_data && !local_reports.is_empty() {
      let mut reports = self.file_reports.lock().expect("mutex poisoned");
      reports.extend(local_reports);
    }

    Ok(filtered)
  }

  /// Checks if the content already has a license header.
  pub fn has_license(&self, content: &str) -> bool {
    // Fast path: check for common license indicators before using the full detector
    if content.starts_with("// Copyright") || content.starts_with("/* Copyright") || content.starts_with("# Copyright")
    {
      return true;
    }

    self.license_detector.has_license(content)
  }

  /// Collects all files that would be processed without actually processing
  /// them.
  pub fn collect_planned_files(&self, patterns: &[String]) -> Result<Vec<PathBuf>> {
    let mut all_files = Vec::new();

    if self.should_use_git_list() {
      let files = self.collect_files(patterns)?;
      let files = self.filter_files_for_plan(files)?;
      return Ok(files);
    }

    for pattern in patterns {
      let maybe_path = PathBuf::from(pattern);
      if maybe_path.is_file() {
        if self.should_include_file_for_plan(&maybe_path)? {
          all_files.push(absolutize_path(&maybe_path)?);
        }
      } else if maybe_path.is_dir() {
        let dir_files = self.collect_directory_files(&maybe_path)?;
        all_files.extend(dir_files);
      } else {
        let entries = glob::glob(pattern).with_context(|| format!("Invalid glob pattern: {}", pattern))?;

        for entry in entries {
          match entry {
            Ok(path) => {
              if path.is_file() {
                if self.should_include_file_for_plan(&path)? {
                  all_files.push(absolutize_path(&path)?);
                }
              } else if path.is_dir() {
                let dir_files = self.collect_directory_files(&path)?;
                all_files.extend(dir_files);
              }
            }
            Err(e) => {
              eprintln!("Error with glob pattern: {}", e);
            }
          }
        }
      }
    }

    all_files.sort();
    Ok(all_files)
  }

  fn collect_directory_files(&self, dir: &Path) -> Result<Vec<PathBuf>> {
    let all_files = self.file_collector.traverse_directory(dir)?;
    let filter_with_licenseignore = self.file_filter.with_licenseignore_files(dir, &self.workspace_root)?;
    let filtered_files = self.filter_files_with_filter_sync(all_files, &filter_with_licenseignore)?;
    Ok(filtered_files)
  }

  fn filter_files_with_filter_sync(&self, files: Vec<PathBuf>, filter: &dyn FileFilter) -> Result<Vec<PathBuf>> {
    let mut filtered = Vec::with_capacity(files.len());

    for path in files {
      match std::fs::symlink_metadata(&path) {
        Ok(metadata) => {
          if metadata.file_type().is_symlink() {
            continue;
          }
        }
        Err(_) => continue,
      }

      match filter.should_process(&path) {
        Ok(result) => {
          if !result.should_process {
            continue;
          }
        }
        Err(_) => continue,
      }

      if let Some(ref ext_filter) = self.extension_filter {
        match ext_filter.should_process(&path) {
          Ok(result) => {
            if !result.should_process {
              continue;
            }
          }
          Err(_) => continue,
        }
      }

      if !self.template_manager.can_handle_file_type(&path) {
        continue;
      }

      filtered.push(path);
    }

    Ok(filtered)
  }

  fn filter_files_for_plan(&self, files: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    let mut filtered = Vec::with_capacity(files.len());

    for path in files {
      match std::fs::symlink_metadata(&path) {
        Ok(metadata) => {
          if metadata.file_type().is_symlink() {
            continue;
          }
        }
        Err(_) => continue,
      }

      let filter_result = self.should_process_file(&path)?;
      if !filter_result.should_process {
        continue;
      }

      if !self.template_manager.can_handle_file_type(&path) {
        continue;
      }

      let absolute_path = absolutize_path(&path)?;

      if let Some(parent_dir) = absolute_path.parent()
        && parent_dir.exists()
      {
        let ignore_manager = {
          let mut cache = self.ignore_manager_cache.lock().expect("mutex poisoned");

          if let Some(cached_manager) = cache.get(parent_dir) {
            cached_manager.clone()
          } else {
            let mut new_manager = self.ignore_manager.clone();
            new_manager.load_licenseignore_files(parent_dir, &self.workspace_root)?;
            cache.insert(parent_dir.to_path_buf(), new_manager.clone());
            new_manager
          }
        };

        if ignore_manager.is_ignored(&absolute_path) {
          continue;
        }
      }

      filtered.push(absolute_path);
    }

    Ok(filtered)
  }

  fn should_include_file_for_plan(&self, path: &Path) -> Result<bool> {
    match std::fs::symlink_metadata(path) {
      Ok(metadata) => {
        if metadata.file_type().is_symlink() {
          return Ok(false);
        }
      }
      Err(_) => return Ok(false),
    }

    let absolute_path = absolutize_path(path)?;

    let filter_result = self.should_process_file(&absolute_path)?;
    if !filter_result.should_process {
      return Ok(false);
    }

    if !self.template_manager.can_handle_file_type(&absolute_path) {
      return Ok(false);
    }

    if let Some(parent_dir) = absolute_path.parent()
      && parent_dir.exists()
    {
      let ignore_manager = {
        let mut cache = self.ignore_manager_cache.lock().expect("mutex poisoned");

        if let Some(cached_manager) = cache.get(parent_dir) {
          cached_manager.clone()
        } else {
          let mut new_manager = self.ignore_manager.clone();
          new_manager.load_licenseignore_files(parent_dir, &self.workspace_root)?;
          cache.insert(parent_dir.to_path_buf(), new_manager.clone());
          new_manager
        }
      };

      if ignore_manager.is_ignored(&absolute_path) {
        return Ok(false);
      }
    }

    Ok(true)
  }
}

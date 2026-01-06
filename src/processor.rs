//! # Processor Module
//!
//! This module contains the core functionality for processing files and
//! directories, adding license headers, and checking for existing licenses.
//!
//! The [`Processor`] struct is the main entry point for all file operations.

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};

use anyhow::{Context, Result};
use regex::Regex;
use tokio::fs;
use tokio::io::AsyncReadExt;

use crate::diff::DiffManager;
use crate::file_filter::{CompositeFilter, FileFilter, IgnoreFilter, create_default_filter};
use crate::ignore::IgnoreManager;
use crate::license_detection::{LicenseDetector, SimpleLicenseDetector};
use crate::report::{FileAction, FileReport};
use crate::templates::{LicenseData, TemplateManager};
use crate::{git, info_log, verbose_log};

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
  /// Template manager for rendering license templates
  template_manager: TemplateManager,

  /// License data (year, etc.) for rendering templates
  license_data: LicenseData,

  /// Composite file filter for determining which files to process
  file_filter: CompositeFilter,

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
  pub file_reports: Arc<tokio::sync::Mutex<Vec<FileReport>>>,

  /// Whether to collect report data
  collect_report_data: bool,

  /// License detector for checking if files have license headers
  license_detector: Arc<Box<dyn LicenseDetector + Send + Sync>>,

  /// Cache for ignore managers to avoid redundant .licenseignore file loading
  ignore_manager_cache: Arc<tokio::sync::Mutex<std::collections::HashMap<PathBuf, IgnoreManager>>>,

  /// Whether to only process git-tracked files
  git_only: bool,

  /// Git reference for ratchet mode
  ratchet_reference: Option<String>,
}

const LICENSE_READ_LIMIT: usize = 8 * 1024;

enum PatternMatcher {
  Any,
  File(PathBuf),
  Dir(PathBuf),
  Glob(glob::Pattern),
}

impl Processor {
  /// Creates a new processor with the specified configuration.
  ///
  /// # Parameters
  ///
  /// * `template_manager` - Manager for license templates
  /// * `license_data` - Data for rendering license templates (year, etc.)
  /// * `ignore_patterns` - Glob patterns for files to ignore
  /// * `check_only` - Whether to only check for licenses without modifying
  ///   files
  /// * `preserve_years` - Whether to preserve existing years in license headers
  /// * `ratchet_reference` - Git reference for ratchet mode (only process
  ///   changed files)
  /// * `diff_manager` - Optional manager for handling diff creation and
  ///   rendering. If not provided, a default one will be created.
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
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    template_manager: TemplateManager,
    license_data: LicenseData,
    ignore_patterns: Vec<String>,
    check_only: bool,
    preserve_years: bool,
    ratchet_reference: Option<String>,
    diff_manager: Option<DiffManager>,
    git_only: bool,
    license_detector: Option<Box<dyn LicenseDetector + Send + Sync>>,
  ) -> Result<Self> {
    // Create ignore manager for base ignore patterns
    let ignore_manager = IgnoreManager::new(ignore_patterns.clone())?;

    // Create a composite file filter with all filtering conditions
    let file_filter = create_default_filter(ignore_patterns)?;

    let diff_manager = diff_manager.unwrap_or_else(|| DiffManager::new(false, None));

    let license_detector = license_detector.unwrap_or_else(|| Box::new(SimpleLicenseDetector::new()));

    Ok(Self {
      template_manager,
      license_data,
      file_filter,
      ignore_manager,
      check_only,
      preserve_years,
      diff_manager,
      files_processed: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
      file_reports: Arc::new(tokio::sync::Mutex::new(Vec::new())),
      collect_report_data: true, // Enable report data collection by default
      license_detector: Arc::new(license_detector),
      ignore_manager_cache: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
      git_only,
      ratchet_reference,
    })
  }

  // No clone_for_task method needed

  /// Processes a list of file or directory patterns.
  ///
  /// This is the main entry point for processing files. It handles:
  /// - Individual files
  /// - Directories (recursively)
  /// - Glob patterns
  ///
  /// # Parameters
  ///
  /// * `patterns` - A slice of strings representing file paths, directory
  ///   paths, or glob patterns
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
  pub async fn process(&self, patterns: &[String]) -> Result<bool> {
    let has_missing_license = Arc::new(AtomicBool::new(false));

    if self.should_use_git_list() {
      let files = self.collect_files(patterns)?;
      let ignore_filter = IgnoreFilter::new(self.ignore_manager.clone());
      return self.process_files_with_filter(files, &ignore_filter, None).await;
    }

    // Process each pattern
    for pattern in patterns {
      // Check if the pattern is a file or directory
      let maybe_path = PathBuf::from(pattern);
      if maybe_path.is_file() {
        // Process a single file
        // Load .licenseignore files from the file's parent directory
        let result = self.process_file_with_ignore_context(&maybe_path).await;
        if let Err(e) = result {
          eprintln!("Error processing {}: {}", maybe_path.display(), e);
          has_missing_license.store(true, Ordering::Relaxed);
        }
      } else if maybe_path.is_dir() {
        // Process a directory recursively
        let has_missing = self.process_directory(&maybe_path).await?;
        if has_missing {
          has_missing_license.store(true, Ordering::Relaxed);
        }
      } else {
        // Try to use the pattern as a glob
        let entries = glob::glob(pattern).with_context(|| format!("Invalid glob pattern: {}", pattern))?;

        // Process glob entries sequentially but with async operations
        for entry in entries {
          match entry {
            Ok(path) => {
              if path.is_file() {
                // Process a single file matching the glob pattern
                // Load .licenseignore files from the file's parent directory
                let result = self.process_file_with_ignore_context(&path).await;
                if let Err(e) = result {
                  eprintln!("Error processing {}: {}", path.display(), e);
                  has_missing_license.store(true, Ordering::Relaxed);
                }
              } else if path.is_dir() {
                let has_missing = self.process_directory(&path).await?;
                if has_missing {
                  has_missing_license.store(true, Ordering::Relaxed);
                }
              }
            }
            Err(e) => {
              eprintln!("Error with glob pattern: {}", e);
            }
          }
        }
      }
    }

    Ok(has_missing_license.load(Ordering::Relaxed))
  }

  const fn should_use_git_list(&self) -> bool {
    self.git_only || self.ratchet_reference.is_some()
  }

  fn collect_files(&self, patterns: &[String]) -> Result<Vec<PathBuf>> {
    let files = if self.git_only {
      git::get_git_tracked_files()?.into_iter().collect::<Vec<_>>()
    } else if let Some(reference) = &self.ratchet_reference {
      git::get_changed_files(reference)?.into_iter().collect::<Vec<_>>()
    } else {
      Vec::new()
    };

    if files.is_empty() {
      return Ok(Vec::new());
    }

    let current_dir = std::env::current_dir().with_context(|| "Failed to get current directory")?;
    let matchers = build_pattern_matchers(patterns, &current_dir)?;

    let mut selected = Vec::new();
    for file in files {
      let normalized = normalize_relative_path(&file, &current_dir);
      if matches_any_pattern(&normalized, &matchers) {
        selected.push(normalized);
      }
    }

    Ok(selected)
  }

  /// Process a file with ignore context from its parent directory.
  ///
  /// This ensures that .licenseignore files in the file's directory are
  /// applied even to explicitly named files.
  async fn process_file_with_ignore_context(&self, path: &Path) -> Result<()> {
    // Create a local reports collection for this file
    let local_reports = Arc::new(tokio::sync::Mutex::new(Vec::new()));

    // Get the parent directory of the file to load directory-specific ignore
    // patterns
    if let Some(parent_dir) = path.parent() {
      // Create a temporary ignore filter with the parent directory's ignore patterns
      if parent_dir.exists() {
        // Check cache first for the ignore manager
        let ignore_manager = {
          let mut cache = self.ignore_manager_cache.lock().await;

          if let Some(cached_manager) = cache.get(parent_dir) {
            // Use cached ignore manager
            verbose_log!("Using cached ignore manager for: {}", parent_dir.display());
            cached_manager.clone()
          } else {
            // Create new ignore manager and cache it
            verbose_log!("Creating new ignore manager for: {}", parent_dir.display());
            let mut new_manager = self.ignore_manager.clone();
            new_manager.load_licenseignore_files(parent_dir)?;

            // Store in cache
            cache.insert(parent_dir.to_path_buf(), new_manager.clone());
            new_manager
          }
        };

        // Check if the file is ignored by the parent directory-specific patterns
        if ignore_manager.is_ignored(path) {
          verbose_log!("Skipping: {} (matches .licenseignore pattern)", path.display());

          // Add to local reports if collecting report data
          if self.collect_report_data {
            let file_report = FileReport {
              path: path.to_path_buf(),
              has_license: false, // We don't know, but we're skipping it
              action_taken: Some(FileAction::Skipped),
              ignored: true,
              ignored_reason: Some("Matches .licenseignore pattern".to_string()),
            };

            let mut reports = local_reports.lock().await;
            reports.push(file_report);
          }

          // Update the shared reports with the local collection
          if self.collect_report_data {
            let local_report_data = {
              let mut reports = local_reports.lock().await;
              std::mem::take(&mut *reports)
            };

            if !local_report_data.is_empty() {
              let mut reports = self.file_reports.lock().await;
              reports.extend(local_report_data);
            }
          }

          return Ok(());
        }
      }
    }

    // Process the file normally with the local reports collection
    let result = self.process_file_with_local_reports(path, &local_reports).await;

    // Update the shared reports with the local collection
    if self.collect_report_data {
      let local_report_data = {
        let mut reports = local_reports.lock().await;
        std::mem::take(&mut *reports)
      };

      if !local_report_data.is_empty() {
        let mut reports = self.file_reports.lock().await;
        reports.extend(local_report_data);
      }
    }

    result
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
  ///
  /// # Performance
  ///
  /// This method uses async operations for processing files in a directory.
  pub async fn process_directory(&self, dir: &Path) -> Result<bool> {
    self.process_directory_internal(dir, None).await
  }

  /// Processes a directory recursively with a caller-specified concurrency.
  #[allow(dead_code)]
  pub async fn process_directory_with_concurrency(&self, dir: &Path, concurrency: usize) -> Result<bool> {
    self.process_directory_internal(dir, Some(concurrency)).await
  }

  async fn process_directory_internal(&self, dir: &Path, concurrency_override: Option<usize>) -> Result<bool> {
    // Pre-allocate vectors for better performance
    let mut all_files = Vec::with_capacity(1000);

    // Asynchronous directory traversal with optimized memory usage
    let mut dirs_to_process = std::collections::VecDeque::with_capacity(100);
    dirs_to_process.push_back(dir.to_path_buf());

    // Process directories in batches for better performance
    verbose_log!("Scanning directory: {}", dir.display());
    let start_time = std::time::Instant::now();

    while let Some(current_dir) = dirs_to_process.pop_front() {
      let read_dir_result = tokio::fs::read_dir(&current_dir).await;
      if let Err(e) = read_dir_result {
        eprintln!("Error reading directory {}: {}", current_dir.display(), e);
        continue;
      }

      let mut entries = read_dir_result.expect("Valid read_dir");
      while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();

        // Use metadata instead of file_type for better performance
        if let Ok(metadata) = entry.metadata().await {
          if metadata.is_dir() {
            dirs_to_process.push_back(path);
          } else if metadata.is_file() {
            all_files.push(path);
          }
        }
      }
    }

    verbose_log!(
      "Found {} files in {}ms",
      all_files.len(),
      start_time.elapsed().as_millis()
    );
    self
      .process_files_with_filter(all_files, &self.file_filter, concurrency_override)
      .await
  }

  async fn process_files_with_filter(
    &self,
    files: Vec<PathBuf>,
    filter: &dyn FileFilter,
    concurrency_override: Option<usize>,
  ) -> Result<bool> {
    let has_missing_license = Arc::new(AtomicBool::new(false));
    let has_missing_clone = Arc::clone(&has_missing_license);

    if files.is_empty() {
      verbose_log!("No files to process");
      return Ok(false);
    }

    let mut local_reports = Vec::with_capacity(1000);

    // Filter files using the file_filter directly - optimized to avoid unnecessary
    // clones
    let filter_start = std::time::Instant::now();
    let files: Vec<_> = files
      .into_iter()
      .filter(|p| match filter.should_process(p) {
        Ok(result) => {
          if !result.should_process {
            let reason_clone = result.reason.clone();
            let reason_display = reason_clone.clone().unwrap_or_else(|| "Unknown reason".to_string());
            verbose_log!("Skipping: {} ({})", p.display(), reason_display);

            if self.collect_report_data {
              let file_report = FileReport {
                path: p.to_path_buf(),
                has_license: false,
                action_taken: Some(FileAction::Skipped),
                ignored: true,
                ignored_reason: reason_clone,
              };

              local_reports.push(file_report);
            }

            false
          } else {
            true
          }
        }
        Err(_) => false,
      })
      .collect();

    verbose_log!(
      "Filtered to {} files to process in {}ms",
      files.len(),
      filter_start.elapsed().as_millis()
    );

    if files.is_empty() {
      verbose_log!("No files to process after filtering");

      if self.collect_report_data && !local_reports.is_empty() {
        let mut reports = self.file_reports.lock().await;
        reports.extend(local_reports);
      }

      return Ok(false);
    }

    let num_cpus = num_cpus::get();
    let files_len = files.len();
    let mut concurrency = std::cmp::min(num_cpus * 4, files_len);
    concurrency = std::cmp::max(concurrency, 1);

    if let Some(override_concurrency) = concurrency_override {
      let override_concurrency = std::cmp::max(override_concurrency, 1);
      concurrency = std::cmp::min(override_concurrency, files_len);
      verbose_log!(
        "Processing {} files with concurrency {} (override)",
        files_len,
        concurrency
      );
    } else {
      verbose_log!("Processing {} files with concurrency {}", files_len, concurrency);
    }

    let (report_sender, report_receiver) = tokio::sync::mpsc::channel::<FileReport>(concurrency * 2);

    use futures::stream::{self, StreamExt};

    let process_start = std::time::Instant::now();

    stream::iter(files)
      .map(|path| {
        let has_missing = Arc::clone(&has_missing_license);
        let processor = self;
        let sender = report_sender.clone();

        async move {
          let result = processor.process_file_efficient_no_filter(&path, sender.clone()).await;
          if let Err(e) = result {
            if processor.check_only && e.to_string().contains("Missing license header") {
              has_missing.store(true, Ordering::Relaxed);
            } else {
              eprintln!("Error processing {}: {}", path.display(), e);
              has_missing.store(true, Ordering::Relaxed);
            }
          }
        }
      })
      .buffer_unordered(concurrency)
      .collect::<Vec<_>>()
      .await;

    drop(report_sender);

    let mut receiver = report_receiver;
    while let Ok(report) = receiver.try_recv() {
      local_reports.push(report);
    }

    verbose_log!(
      "Processed {} files in {}ms",
      files_len,
      process_start.elapsed().as_millis()
    );

    if self.collect_report_data && !local_reports.is_empty() {
      let mut reports = self.file_reports.lock().await;
      reports.extend(local_reports);
    }

    Ok(has_missing_clone.load(Ordering::Relaxed))
  }

  /// Processes a single file, adding or checking a license header.
  ///
  /// This method:
  /// 1. Checks if the file should be ignored
  /// 2. In ratchet mode, checks if the file has changed
  /// 3. Reads the file content
  /// 4. Checks if the file already has a license header
  /// 5. In check-only mode:
  ///    - If show_diff is enabled, shows a diff of what would be changed
  ///    - Otherwise, returns an error if the license is missing
  /// 6. Otherwise, adds a license header or updates the year in an existing one
  ///
  /// # Parameters
  ///
  /// * `path` - Path to the file to process
  ///
  /// # Returns
  ///
  /// `Ok(())` if the file was processed successfully, or an error if:
  /// - The file cannot be read or written
  /// - The file is missing a license header in check-only mode
  /// - License template rendering fails
  #[allow(dead_code)]
  pub async fn process_file(&self, path: &Path) -> Result<()> {
    // Use the local reports version with an empty reports collection that will be
    // discarded
    let dummy_reports = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    self.process_file_with_local_reports(path, &dummy_reports).await
  }

  /// Processes a single file with a local reports collection to reduce mutex
  /// contention.
  ///
  /// This version of process_file uses a local reports collection passed from
  /// the caller instead of directly updating the shared file_reports mutex.
  /// This reduces lock contention when processing files concurrently.
  ///
  /// # Parameters
  ///
  /// * `path` - Path to the file to process
  /// * `local_reports` - Local collection for file reports
  ///
  /// # Returns
  ///
  /// `Ok(())` if the file was processed successfully, or an error if:
  /// - The file cannot be read or written
  /// - The file is missing a license header in check-only mode
  /// - License template rendering fails
  pub async fn process_file_with_local_reports(
    &self,
    path: &Path,
    local_reports: &Arc<tokio::sync::Mutex<Vec<FileReport>>>,
  ) -> Result<()> {
    verbose_log!("Processing file: {}", path.display());

    // Use our composite file filter to determine if we should process this file
    let filter_result = self.file_filter.should_process(path)?;
    if !filter_result.should_process {
      let reason = filter_result
        .reason
        .clone()
        .unwrap_or_else(|| "Unknown reason".to_string());
      verbose_log!("Skipping: {} ({})", path.display(), reason);

      // Add to local reports if collecting report data
      if self.collect_report_data {
        let file_report = FileReport {
          path: path.to_path_buf(),
          has_license: false, // We don't know, but we're skipping it
          action_taken: Some(FileAction::Skipped),
          ignored: true,
          ignored_reason: filter_result.reason.clone(),
        };

        let mut reports = local_reports.lock().await;
        reports.push(file_report);
      }

      return Ok(());
    }

    // Increment the files processed counter
    self.files_processed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    let diff_requested = self.diff_manager.show_diff || self.diff_manager.save_diff_path.is_some();
    let prefix_content = self.read_license_check_prefix(path).await?;
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

    let content = if needs_full_content {
      self.read_full_content(path).await?
    } else {
      prefix_content
    };

    verbose_log!("File has license: {}", has_license);

    if self.check_only {
      if !has_license {
        // In check-only mode, we need to signal that a license is missing
        // This is used by the test_processor_with_licenseignore test

        // Generate diffs if show_diff is enabled or save_diff_path is provided
        if self.diff_manager.show_diff || self.diff_manager.save_diff_path.is_some() {
          // Generate what the content would look like with a license
          let license_text = self
            .template_manager
            .render(&self.license_data)
            .with_context(|| "Failed to render license template")?;

          let formatted_license = self.template_manager.format_for_file_type(&license_text, path);

          // Handle shebang or other special headers
          let (prefix, content_without_prefix) = self.extract_prefix(&content);

          // Combine prefix, license, and content
          let new_content = format!("{}{}{}", prefix, formatted_license, content_without_prefix);

          // Generate and display/save the diff
          self.diff_manager.display_diff(path, &content, &new_content)?;
        }

        // Add to local reports if collecting report data
        if self.collect_report_data {
          let file_report = FileReport {
            path: path.to_path_buf(),
            has_license,
            action_taken: None, // No action taken in check mode
            ignored: false,
            ignored_reason: None,
          };

          let mut reports = local_reports.lock().await;
          reports.push(file_report);
        }

        // Signal that a license is missing by returning an error
        // This will be caught by the process_directory method and set
        // has_missing_license to true
        return Err(anyhow::anyhow!("Missing license header"));
      } else if !self.preserve_years && (self.diff_manager.show_diff || self.diff_manager.save_diff_path.is_some()) {
        // Check if we would update the year in the license
        let updated_content = self.update_year_in_license(&content)?;
        if updated_content != content {
          // Generate and display/save the diff
          self
            .diff_manager
            .display_diff(path, &content, updated_content.as_ref())?;
        }

        // Add to local reports if collecting report data
        if self.collect_report_data {
          let file_report = FileReport {
            path: path.to_path_buf(),
            has_license,
            action_taken: None, // No action taken in check mode, but would update year
            ignored: false,
            ignored_reason: None,
          };

          let mut reports = local_reports.lock().await;
          reports.push(file_report);
        }
      } else {
        // File has license and we wouldn't update it
        if self.collect_report_data {
          let file_report = FileReport {
            path: path.to_path_buf(),
            has_license,
            action_taken: Some(FileAction::NoActionNeeded),
            ignored: false,
            ignored_reason: None,
          };

          let mut reports = local_reports.lock().await;
          reports.push(file_report);
        }
      }
      return Ok(());
    }

    if has_license {
      // If the file has a license and we're not in preserve_years mode,
      // check if we need to update the year
      if !self.preserve_years {
        let updated_content = self.update_year_in_license(&content)?;
        if updated_content != content {
          verbose_log!("Updating year in: {}", path.display());

          fs::write(path, updated_content.as_ref().as_bytes())
            .await
            .with_context(|| format!("Failed to write to file: {}", path.display()))?;

          // Log the updated file with colors
          info_log!("Updated year in: {}", path.display());

          // Add to local reports if collecting report data
          if self.collect_report_data {
            let file_report = FileReport {
              path: path.to_path_buf(),
              has_license: true,
              action_taken: Some(FileAction::YearUpdated),
              ignored: false,
              ignored_reason: None,
            };

            let mut reports = local_reports.lock().await;
            reports.push(file_report);
          }
        } else {
          // No changes needed - add to report
          if self.collect_report_data {
            let file_report = FileReport {
              path: path.to_path_buf(),
              has_license: true,
              action_taken: Some(FileAction::NoActionNeeded),
              ignored: false,
              ignored_reason: None,
            };

            let mut reports = local_reports.lock().await;
            reports.push(file_report);
          }
        }
      } else {
        // Preserve years mode enabled - add to report
        if self.collect_report_data {
          let file_report = FileReport {
            path: path.to_path_buf(),
            has_license: true,
            action_taken: Some(FileAction::NoActionNeeded),
            ignored: false,
            ignored_reason: None,
          };

          let mut reports = local_reports.lock().await;
          reports.push(file_report);
        }
      }
    } else {
      // Add license to the file
      let license_text = self
        .template_manager
        .render(&self.license_data)
        .with_context(|| "Failed to render license template")?;

      verbose_log!("Rendered license text:\n{}", license_text);

      let formatted_license = self.template_manager.format_for_file_type(&license_text, path);

      verbose_log!("Formatted license for file type:\n{}", formatted_license);

      // Handle shebang or other special headers
      let (prefix, content) = self.extract_prefix(&content);

      // Combine prefix, license, and content
      let new_content = format!("{}{}{}", prefix, formatted_license, content);

      verbose_log!("Writing updated content to: {}", path.display());

      // Write the updated content back to the file
      fs::write(path, &new_content)
        .await
        .with_context(|| format!("Failed to write to file: {}", path.display()))?;

      // Log the added license with colors
      info_log!("Added license to: {}", path.display());

      // Add to local reports if collecting report data
      if self.collect_report_data {
        let file_report = FileReport {
          path: path.to_path_buf(),
          has_license: true, // Now it has a license
          action_taken: Some(FileAction::Added),
          ignored: false,
          ignored_reason: None,
        };

        let mut reports = local_reports.lock().await;
        reports.push(file_report);
      }
    }

    Ok(())
  }

  /// A more efficient version of process_file that uses a channel for report
  /// collection.
  ///
  /// This method avoids mutex contention by using a channel to send reports
  /// back to the caller. It also uses more efficient file I/O operations.
  ///
  /// # Parameters
  ///
  /// * `path` - Path to the file to process
  /// * `report_sender` - Channel sender for file reports
  ///
  /// # Returns
  ///
  /// `Ok(())` if the file was processed successfully, or an error if:
  /// - The file cannot be read or written
  /// - The file is missing a license header in check-only mode
  /// - License template rendering fails
  #[allow(dead_code)]
  pub async fn process_file_efficient(
    &self,
    path: &Path,
    report_sender: tokio::sync::mpsc::Sender<FileReport>,
  ) -> Result<()> {
    // Use our composite file filter to determine if we should process this file
    let filter_result = self.file_filter.should_process(path)?;
    if !filter_result.should_process {
      // Only log in verbose mode to reduce I/O overhead
      verbose_log!(
        "Skipping: {} ({})",
        path.display(),
        filter_result
          .reason
          .clone()
          .unwrap_or_else(|| "Unknown reason".to_string())
      );

      // Send report through channel if collecting report data
      if self.collect_report_data {
        let file_report = FileReport {
          path: path.to_path_buf(),
          has_license: false, // We don't know, but we're skipping it
          action_taken: Some(FileAction::Skipped),
          ignored: true,
          ignored_reason: filter_result.reason,
        };

        // Try to send the report, but don't wait if the channel is full
        let _ = report_sender.try_send(file_report);
      }

      return Ok(());
    }

    self.process_file_efficient_no_filter(path, report_sender).await
  }

  /// A more efficient version of process_file that uses a channel for report
  /// collection but does not apply filtering.
  ///
  /// This method avoids mutex contention by using a channel to send reports
  /// back to the caller. It also uses more efficient file I/O operations.
  ///
  /// # Parameters
  ///
  /// * `path` - Path to the file to process
  /// * `report_sender` - Channel sender for file reports
  ///
  /// # Returns
  ///
  /// `Ok(())` if the file was processed successfully, or an error if:
  /// - The file cannot be read or written
  /// - The file is missing a license header in check-only mode
  /// - License template rendering fails
  async fn process_file_efficient_no_filter(
    &self,
    path: &Path,
    report_sender: tokio::sync::mpsc::Sender<FileReport>,
  ) -> Result<()> {
    // Increment the files processed counter
    self.files_processed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // Get file metadata to check if we need to read the file
    let metadata = match tokio::fs::metadata(path).await {
      Ok(meta) => meta,
      Err(e) => {
        return Err(anyhow::anyhow!("Failed to get metadata for {}: {}", path.display(), e));
      }
    };

    // Skip empty files
    if metadata.len() == 0 {
      if self.collect_report_data {
        let file_report = FileReport {
          path: path.to_path_buf(),
          has_license: false,
          action_taken: Some(FileAction::Skipped),
          ignored: true,
          ignored_reason: Some("Empty file".to_string()),
        };
        let _ = report_sender.try_send(file_report);
      }
      return Ok(());
    }

    let diff_requested = self.diff_manager.show_diff || self.diff_manager.save_diff_path.is_some();
    let prefix_content = self.read_license_check_prefix(path).await?;
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

    let content = if needs_full_content {
      self.read_full_content(path).await?
    } else {
      prefix_content
    };

    if self.check_only {
      if !has_license {
        // In check-only mode, we need to signal that a license is missing

        // Generate diffs if show_diff is enabled or save_diff_path is provided
        if self.diff_manager.show_diff || self.diff_manager.save_diff_path.is_some() {
          // Generate what the content would look like with a license
          let license_text = match self.template_manager.render(&self.license_data) {
            Ok(text) => text,
            Err(e) => return Err(anyhow::anyhow!("Failed to render license template: {}", e)),
          };

          let formatted_license = self.template_manager.format_for_file_type(&license_text, path);
          let (prefix, content_without_prefix) = self.extract_prefix(&content);
          let new_content = format!("{}{}{}", prefix, formatted_license, content_without_prefix);

          // Generate and display/save the diff
          if let Err(e) = self.diff_manager.display_diff(path, &content, &new_content) {
            eprintln!("Warning: Failed to display diff for {}: {}", path.display(), e);
          }
        }

        // Send report through channel if collecting report data
        if self.collect_report_data {
          let file_report = FileReport {
            path: path.to_path_buf(),
            has_license,
            action_taken: None, // No action taken in check mode
            ignored: false,
            ignored_reason: None,
          };

          let _ = report_sender.try_send(file_report);
        }

        // Signal that a license is missing by returning an error
        return Err(anyhow::anyhow!("Missing license header"));
      } else if !self.preserve_years && (self.diff_manager.show_diff || self.diff_manager.save_diff_path.is_some()) {
        // Check if we would update the year in the license
        let updated_content = self.update_year_in_license(&content)?;
        if updated_content != content {
          // Generate and display/save the diff
          if let Err(e) = self.diff_manager.display_diff(path, &content, updated_content.as_ref()) {
            eprintln!("Warning: Failed to display diff for {}: {}", path.display(), e);
          }
        }

        // Send report through channel if collecting report data
        if self.collect_report_data {
          let file_report = FileReport {
            path: path.to_path_buf(),
            has_license,
            action_taken: None, // No action taken in check mode, but would update year
            ignored: false,
            ignored_reason: None,
          };

          let _ = report_sender.try_send(file_report);
        }
      } else {
        // File has license and we wouldn't update it
        if self.collect_report_data {
          let file_report = FileReport {
            path: path.to_path_buf(),
            has_license,
            action_taken: Some(FileAction::NoActionNeeded),
            ignored: false,
            ignored_reason: None,
          };

          let _ = report_sender.try_send(file_report);
        }
      }
      return Ok(());
    }

    if has_license {
      // If the file has a license and we're not in preserve_years mode,
      // check if we need to update the year
      if !self.preserve_years {
        // Fast path: check if we need to update the year
        let updated_content = self.update_year_in_license(&content)?;
        if updated_content != content {
          // Write the updated content back to the file with optimized I/O
          if let Err(e) = fs::write(path, updated_content.as_ref().as_bytes()).await {
            return Err(anyhow::anyhow!("Failed to write to file {}: {}", path.display(), e));
          }

          // Log the updated file with colors
          info_log!("Updated year in: {}", path.display());

          // Send report through channel if collecting report data
          if self.collect_report_data {
            let file_report = FileReport {
              path: path.to_path_buf(),
              has_license: true,
              action_taken: Some(FileAction::YearUpdated),
              ignored: false,
              ignored_reason: None,
            };

            let _ = report_sender.try_send(file_report);
          }
        } else {
          // No changes needed - add to report
          if self.collect_report_data {
            let file_report = FileReport {
              path: path.to_path_buf(),
              has_license: true,
              action_taken: Some(FileAction::NoActionNeeded),
              ignored: false,
              ignored_reason: None,
            };

            let _ = report_sender.try_send(file_report);
          }
        }
      } else {
        // Preserve years mode enabled - add to report
        if self.collect_report_data {
          let file_report = FileReport {
            path: path.to_path_buf(),
            has_license: true,
            action_taken: Some(FileAction::NoActionNeeded),
            ignored: false,
            ignored_reason: None,
          };

          let _ = report_sender.try_send(file_report);
        }
      }
    } else {
      // Add license to the file
      let license_text = match self.template_manager.render(&self.license_data) {
        Ok(text) => text,
        Err(e) => return Err(anyhow::anyhow!("Failed to render license template: {}", e)),
      };

      let formatted_license = self.template_manager.format_for_file_type(&license_text, path);
      let (prefix, content_remainder) = self.extract_prefix(&content);
      let new_content = format!("{}{}{}", prefix, formatted_license, content_remainder);

      // Write the updated content back to the file with optimized I/O
      if let Err(e) = fs::write(path, &new_content).await {
        return Err(anyhow::anyhow!("Failed to write to file {}: {}", path.display(), e));
      }

      // Log the added license with colors
      info_log!("Added license to: {}", path.display());

      // Send report through channel if collecting report data
      if self.collect_report_data {
        let file_report = FileReport {
          path: path.to_path_buf(),
          has_license: true, // Now it has a license
          action_taken: Some(FileAction::Added),
          ignored: false,
          ignored_reason: None,
        };

        let _ = report_sender.try_send(file_report);
      }
    }

    Ok(())
  }

  /// Checks if the content already has a license header.
  ///
  /// This method delegates to the configured license detector to determine
  /// if a file already contains a license header.
  ///
  /// # Parameters
  ///
  /// * `content` - The file content to check
  ///
  /// # Returns
  ///
  /// `true` if the content appears to have a license header, `false` otherwise.
  pub fn has_license(&self, content: &str) -> bool {
    // Fast path: check for common license indicators before using the full detector
    if content.starts_with("// Copyright") || content.starts_with("/* Copyright") || content.starts_with("# Copyright")
    {
      return true;
    }

    // Use the full detector for more complex cases
    self.license_detector.has_license(content)
  }

  /// Extracts special prefixes (like shebang) from file content.
  ///
  /// This method identifies and preserves special file prefixes such as:
  /// - Shebangs (#!)
  /// - XML declarations (<?xml)
  /// - HTML doctypes (<!doctype)
  /// - Ruby encoding comments (# encoding:)
  /// - PHP opening tags (<?php)
  /// - Dockerfile directives (# escape, # syntax)
  ///
  /// # Parameters
  ///
  /// * `content` - The file content to process
  ///
  /// # Returns
  ///
  /// A tuple containing:
  /// - The extracted prefix as a String (with added newlines for proper
  ///   separation)
  /// - The remaining content as a string slice
  pub fn extract_prefix<'a>(&self, content: &'a str) -> (String, &'a str) {
    // Common prefixes to preserve
    let prefixes = [
      "#!",                       // shebang
      "<?xml",                    // XML declaration
      "<!doctype",                // HTML doctype
      "# encoding:",              // Ruby encoding
      "# frozen_string_literal:", // Ruby interpreter instruction
      "<?php",                    // PHP opening tag
      "# escape",                 // Dockerfile directive
      "# syntax",                 // Dockerfile directive
    ];

    // Check if the content starts with any of the prefixes
    let first_line_end = content.find('\n').unwrap_or(content.len());
    let first_line = &content[..first_line_end].to_lowercase();

    for prefix in &prefixes {
      if first_line.starts_with(prefix) {
        let mut prefix_str = content[..=first_line_end].to_string();
        if !prefix_str.ends_with('\n') {
          prefix_str.push('\n');
        }
        // Add an extra newline to ensure separation between shebang and license
        prefix_str.push('\n');
        return (prefix_str, &content[first_line_end + 1..]);
      }
    }

    (String::new(), content)
  }

  /// Updates the year in existing license headers.
  ///
  /// This method finds copyright year references in license headers and updates
  /// them to the current year specified in the license data. It handles various
  /// copyright symbol formats including "(c)", "©", or no symbol at all.
  ///
  /// # Parameters
  ///
  /// * `content` - The file content to process
  ///
  /// # Returns
  ///
  /// The updated content with the year references replaced, or an error if the
  /// regex pattern compilation fails.
  pub fn update_year_in_license<'a>(&self, content: &'a str) -> Result<Cow<'a, str>> {
    let current_year = &self.license_data.year;

    // Fast path: if the content already contains the current year in a copyright
    // statement, we can skip the regex processing entirely
    if content.contains(&format!("Copyright (c) {} ", current_year))
      || content.contains(&format!("Copyright © {} ", current_year))
      || content.contains(&format!("Copyright {} ", current_year))
    {
      return Ok(Cow::Borrowed(content));
    }

    // Regex to find copyright year patterns - match all copyright symbol formats
    static YEAR_REGEX: LazyLock<Regex> =
      LazyLock::new(|| Regex::new(r"(?i)(copyright\s+(?:\(c\)|©)?\s+)(\d{4})(\s+)").expect("year regex must compile"));

    let mut needs_update = false;
    for caps in YEAR_REGEX.captures_iter(content) {
      if &caps[2] != current_year {
        needs_update = true;
        break;
      }
    }

    if !needs_update {
      return Ok(Cow::Borrowed(content));
    }

    // Update single year to current year
    let content = YEAR_REGEX.replace_all(content, |caps: &regex::Captures| {
      let prefix = &caps[1];
      let year = &caps[2];
      let suffix = &caps[3];

      if year != current_year {
        format!("{}{}{}", prefix, current_year, suffix)
      } else {
        // Keep as is if already current
        caps[0].to_string()
      }
    });

    Ok(content)
  }

  /// Reads the initial portion of a file for license checking.
  ///
  /// This method reads up to LICENSE_READ_LIMIT bytes from the start of the
  /// file. It attempts to interpret the bytes as UTF-8, handling invalid
  /// sequences by truncating at the last valid character.
  ///
  /// # Parameters
  ///
  /// * `path` - Path to the file to read
  ///
  /// # Returns
  ///
  /// A String containing the read content, or an error if reading fails.
  async fn read_license_check_prefix(&self, path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)
      .await
      .with_context(|| format!("Failed to read file: {}", path.display()))?;
    let mut buf = vec![0u8; LICENSE_READ_LIMIT];
    let read_len = file
      .read(&mut buf)
      .await
      .with_context(|| format!("Failed to read file: {}", path.display()))?;
    buf.truncate(read_len);

    match std::str::from_utf8(&buf) {
      Ok(prefix) => Ok(prefix.to_string()),
      Err(e) => {
        let valid_up_to = e.valid_up_to();
        if valid_up_to == 0 {
          Err(anyhow::anyhow!("Failed to read file {}: {}", path.display(), e))
        } else {
          Ok(String::from_utf8_lossy(&buf[..valid_up_to]).to_string())
        }
      }
    }
  }

  async fn read_full_content(&self, path: &Path) -> Result<String> {
    fs::read_to_string(path)
      .await
      .with_context(|| format!("Failed to read file: {}", path.display()))
  }
}

fn build_pattern_matchers(patterns: &[String], current_dir: &Path) -> Result<Vec<PatternMatcher>> {
  if patterns.is_empty() {
    return Ok(Vec::new());
  }

  let mut matchers = Vec::with_capacity(patterns.len());
  for pattern in patterns {
    let raw_path = PathBuf::from(pattern);
    if raw_path.exists() {
      let normalized = normalize_relative_path(&raw_path, current_dir);
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
      let glob_pattern = glob::Pattern::new(pattern).with_context(|| format!("Invalid glob pattern: {}", pattern))?;
      matchers.push(PatternMatcher::Glob(glob_pattern));
    }
  }

  Ok(matchers)
}

fn matches_any_pattern(path: &Path, matchers: &[PatternMatcher]) -> bool {
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

fn normalize_relative_path(path: &Path, current_dir: &Path) -> PathBuf {
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

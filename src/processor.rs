//! # Processor Module
//!
//! This module contains the core functionality for processing files and directories,
//! adding license headers, and checking for existing licenses.
//!
//! The [`Processor`] struct is the main entry point for all file operations.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result};
use rayon::prelude::*;
use regex::Regex;
use walkdir::WalkDir;

use crate::diff::DiffManager;
use crate::git;
use crate::ignore::IgnoreManager;
use crate::report::{FileAction, FileReport};
use crate::templates::{LicenseData, TemplateManager};
use crate::{info_log, verbose_log};

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

    /// Manager for handling ignore patterns
    ignore_manager: IgnoreManager,

    /// Whether to only check for licenses without modifying files
    check_only: bool,

    /// Whether to preserve existing years in license headers
    preserve_years: bool,

    /// Set of files that have changed (used in ratchet mode)
    pub changed_files: Option<HashSet<PathBuf>>,

    /// Whether to only process files in the git repository
    git_only: bool,

    /// Set of files tracked by git (used when git_only is true)
    git_tracked_files: Option<HashSet<PathBuf>>,

    /// Manager for handling diff creation and rendering
    diff_manager: DiffManager,

    /// Counter for the total number of files processed
    pub files_processed: std::sync::atomic::AtomicUsize,

    /// Collection of file reports for generating reports
    pub file_reports: Arc<Mutex<Vec<FileReport>>>,

    /// Whether to collect report data
    collect_report_data: bool,
}

impl Processor {
    /// Creates a new processor with the specified configuration.
    ///
    /// # Parameters
    ///
    /// * `template_manager` - Manager for license templates
    /// * `license_data` - Data for rendering license templates (year, etc.)
    /// * `ignore_patterns` - Glob patterns for files to ignore
    /// * `check_only` - Whether to only check for licenses without modifying files
    /// * `preserve_years` - Whether to preserve existing years in license headers
    /// * `ratchet_reference` - Git reference for ratchet mode (only process changed files)
    /// * `diff_manager` - Optional manager for handling diff creation and rendering. If not provided, a default one will be created.
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
        git_only: Option<bool>,
    ) -> Result<Self> {
        // Create ignore manager
        let ignore_manager = IgnoreManager::new(ignore_patterns)?;

        // Initialize changed_files if ratchet mode is enabled
        let changed_files = if let Some(ref reference) = ratchet_reference {
            Some(git::get_changed_files(reference)?)
        } else {
            None
        };

        // Determine if we should only process git files
        // This always defaults to false unless explicitly set to true
        // Note: This uses your current working directory ($CWD) to detect the git repository.
        // You should always run edlicense from inside the git repository when git detection is enabled.
        let is_git_repo = git::is_git_repository();
        let git_only = git_only.unwrap_or(false);

        // Initialize git_tracked_files if git_only is true
        // This uses your current working directory ($CWD) to determine which files are tracked.
        let git_tracked_files = if git_only && is_git_repo {
            verbose_log!("Git-only mode enabled, getting tracked files");
            Some(git::get_git_tracked_files()?)
        } else {
            None
        };

        // Use provided diff_manager or create a default one
        let diff_manager = diff_manager.unwrap_or_else(|| DiffManager::new(false, None));

        Ok(Self {
            template_manager,
            license_data,
            ignore_manager,
            check_only,
            preserve_years,
            changed_files,
            git_only,
            git_tracked_files,
            diff_manager,
            files_processed: std::sync::atomic::AtomicUsize::new(0),
            file_reports: Arc::new(Mutex::new(Vec::new())),
            collect_report_data: true, // Enable report data collection by default
        })
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
    /// In check-only mode, this can be used to determine if the check passed or failed.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - A glob pattern is invalid
    /// - Directory traversal fails
    pub fn process(&self, patterns: &[String]) -> Result<bool> {
        let has_missing_license = Arc::new(AtomicBool::new(false));

        // Process each pattern
        for pattern in patterns {
            // Check if the pattern is a file or directory
            let path = PathBuf::from(pattern);
            if path.is_file() {
                // Process a single file
                // Load .licenseignore files from the file's parent directory
                let result = self.process_file_with_ignore_context(&path);
                if let Err(e) = result {
                    eprintln!("Error processing {}: {}", path.display(), e);
                    has_missing_license.store(true, Ordering::Relaxed);
                }
            } else if path.is_dir() {
                // Process a directory recursively
                let has_missing = self.process_directory(&path)?;
                if has_missing {
                    has_missing_license.store(true, Ordering::Relaxed);
                }
            } else {
                // Try to use the pattern as a glob
                let entries = glob::glob(pattern).with_context(|| format!("Invalid glob pattern: {}", pattern))?;

                for entry in entries {
                    match entry {
                        Ok(path) => {
                            if path.is_file() {
                                // Process a single file matching the glob pattern
                                // Load .licenseignore files from the file's parent directory
                                let result = self.process_file_with_ignore_context(&path);
                                if let Err(e) = result {
                                    eprintln!("Error processing {}: {}", path.display(), e);
                                    has_missing_license.store(true, Ordering::Relaxed);
                                }
                            } else if path.is_dir() {
                                let has_missing = self.process_directory(&path)?;
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

    /// Process a file with ignore context from its parent directory.
    ///
    /// This ensures that .licenseignore files in the file's directory are
    /// applied even to explicitly named files.
    fn process_file_with_ignore_context(&self, path: &Path) -> Result<()> {
        // Get the parent directory of the file
        if let Some(parent_dir) = path.parent() {
            // Clone the ignore manager and load .licenseignore files from parent
            let mut ignore_manager = self.ignore_manager.clone();
            if parent_dir.exists() {
                ignore_manager.load_licenseignore_files(parent_dir)?;
            }

            // Check if the file should be ignored
            if ignore_manager.is_ignored(path) {
                verbose_log!("Skipping: {} (matches .licenseignore pattern)", path.display());

                // Add to report if collecting report data
                if self.collect_report_data {
                    let file_report = FileReport {
                        path: path.to_path_buf(),
                        has_license: false, // We don't know, but we're skipping it
                        action_taken: Some(FileAction::Skipped),
                        ignored: true,
                        ignored_reason: Some("Matches .licenseignore pattern".to_string()),
                    };

                    if let Ok(mut reports) = self.file_reports.lock() {
                        reports.push(file_report);
                    }
                }

                return Ok(());
            }
        }

        // Process the file normally
        self.process_file(path)
    }

    /// Processes a directory recursively, adding or checking license headers in all files.
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
    /// This method uses parallel processing via Rayon to improve performance
    /// when dealing with large directories.
    pub fn process_directory(&self, dir: &Path) -> Result<bool> {
        let has_missing_license = Arc::new(AtomicBool::new(false));

        // Load .licenseignore files for this directory
        // Note: We need to clone ignore_manager because we can't mutate self
        let mut ignore_manager = self.ignore_manager.clone();
        ignore_manager.load_licenseignore_files(dir)?;

        // Collect all files in the directory
        let all_files: Vec<_> = WalkDir::new(dir)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_path_buf())
            .collect();

        // Filter out ignored files and log them
        let files: Vec<_> = all_files
            .into_iter()
            .filter(|p| {
                let should_ignore = ignore_manager.is_ignored(p);
                if should_ignore {
                    verbose_log!("Skipping: {} (matches ignore pattern)", p.display());
                }
                !should_ignore
            })
            .collect();

        // Process files in parallel
        files.par_iter().for_each(|path| {
            let result = self.process_file(path);
            if let Err(e) = result {
                // If we're in check-only mode and the error is "Missing license header",
                // this is expected and we should set has_missing_license to true
                if self.check_only && e.to_string().contains("Missing license header") {
                    has_missing_license.store(true, Ordering::Relaxed);
                } else {
                    // For other errors, print them
                    eprintln!("Error processing {}: {}", path.display(), e);
                    // Still set has_missing_license to true for any error
                    has_missing_license.store(true, Ordering::Relaxed);
                }
            }
        });

        Ok(has_missing_license.load(Ordering::Relaxed))
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
    pub fn process_file(&self, path: &Path) -> Result<()> {
        verbose_log!("Processing file: {}", path.display());

        // Skip files that match ignore patterns
        if self.ignore_manager.is_ignored(path) {
            verbose_log!("Skipping: {} (matches ignore pattern)", path.display());

            // Add to report if collecting report data
            if self.collect_report_data {
                let file_report = FileReport {
                    path: path.to_path_buf(),
                    has_license: false, // We don't know, but we're skipping it
                    action_taken: Some(FileAction::Skipped),
                    ignored: true,
                    ignored_reason: Some("Matches ignore pattern".to_string()),
                };

                if let Ok(mut reports) = self.file_reports.lock() {
                    reports.push(file_report);
                }
            }

            return Ok(());
        }

        // Skip files that aren't tracked by git when git_only is enabled
        if self.git_only {
            if let Some(ref git_tracked_files) = self.git_tracked_files {
                // Check if the file is in the tracked files list
                let is_tracked = git_tracked_files.iter().any(|tracked_path| {
                    // Convert both paths to strings for comparison
                    let tracked_str = tracked_path.to_string_lossy().to_string();
                    let path_str = path.to_string_lossy().to_string();

                    // Check if the path contains the tracked path or vice versa
                    tracked_str.contains(&path_str) || path_str.contains(&tracked_str)
                });

                if !is_tracked {
                    verbose_log!("Skipping: {} (not tracked by git)", path.display());

                    // Add to report if collecting report data
                    if self.collect_report_data {
                        let file_report = FileReport {
                            path: path.to_path_buf(),
                            has_license: false, // We don't know, but we're skipping it
                            action_taken: Some(FileAction::Skipped),
                            ignored: true,
                            ignored_reason: Some("Not tracked by git".to_string()),
                        };

                        if let Ok(mut reports) = self.file_reports.lock() {
                            reports.push(file_report);
                        }
                    }

                    return Ok(());
                }
                verbose_log!("Processing: {} (tracked by git)", path.display());
            } else {
                // If git_only is true but git_tracked_files is None, we're not in a git repo
                // This should have been caught in main.rs with an error, but just in case:
                return Err(anyhow::anyhow!("Git-only mode is enabled, but not in a git repository"));
            }
        }

        // Skip files that haven't changed in ratchet mode
        if let Some(ref changed_files) = self.changed_files {
            if !changed_files.contains(path) {
                verbose_log!("Skipping: {} (unchanged in ratchet mode)", path.display());

                // Add to report if collecting report data
                if self.collect_report_data {
                    let file_report = FileReport {
                        path: path.to_path_buf(),
                        has_license: false, // We don't know, but we're skipping it
                        action_taken: Some(FileAction::Skipped),
                        ignored: true,
                        ignored_reason: Some("Unchanged in ratchet mode".to_string()),
                    };

                    if let Ok(mut reports) = self.file_reports.lock() {
                        reports.push(file_report);
                    }
                }

                return Ok(());
            }
            verbose_log!("Processing: {} (changed in ratchet mode)", path.display());
        }

        // Increment the files processed counter
        self.files_processed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Read file content
        let content = fs::read_to_string(path).with_context(|| format!("Failed to read file: {}", path.display()))?;

        // Check if the file already has a license
        let has_license = self.has_license(&content);
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

                // Add to report if collecting report data
                if self.collect_report_data {
                    let file_report = FileReport {
                        path: path.to_path_buf(),
                        has_license,
                        action_taken: None, // No action taken in check mode
                        ignored: false,
                        ignored_reason: None,
                    };

                    if let Ok(mut reports) = self.file_reports.lock() {
                        reports.push(file_report);
                    }
                }

                // Signal that a license is missing by returning an error
                // This will be caught by the process_directory method and set has_missing_license to true
                return Err(anyhow::anyhow!("Missing license header"));
            } else if !self.preserve_years
                && (self.diff_manager.show_diff || self.diff_manager.save_diff_path.is_some())
            {
                // Check if we would update the year in the license
                let updated_content = self.update_year_in_license(&content)?;
                if updated_content != content {
                    // Generate and display/save the diff
                    self.diff_manager.display_diff(path, &content, &updated_content)?;
                }

                // Add to report if collecting report data
                if self.collect_report_data {
                    let file_report = FileReport {
                        path: path.to_path_buf(),
                        has_license,
                        action_taken: None, // No action taken in check mode, but would update year
                        ignored: false,
                        ignored_reason: None,
                    };

                    if let Ok(mut reports) = self.file_reports.lock() {
                        reports.push(file_report);
                    }
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

                    if let Ok(mut reports) = self.file_reports.lock() {
                        reports.push(file_report);
                    }
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
                    fs::write(path, updated_content)
                        .with_context(|| format!("Failed to write to file: {}", path.display()))?;

                    // Log the updated file with colors
                    info_log!("Updated year in: {}", path.display());

                    // Add to report if collecting report data
                    if self.collect_report_data {
                        let file_report = FileReport {
                            path: path.to_path_buf(),
                            has_license: true,
                            action_taken: Some(FileAction::YearUpdated),
                            ignored: false,
                            ignored_reason: None,
                        };

                        if let Ok(mut reports) = self.file_reports.lock() {
                            reports.push(file_report);
                        }
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

                        if let Ok(mut reports) = self.file_reports.lock() {
                            reports.push(file_report);
                        }
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

                    if let Ok(mut reports) = self.file_reports.lock() {
                        reports.push(file_report);
                    }
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
            fs::write(path, new_content).with_context(|| format!("Failed to write to file: {}", path.display()))?;

            // Log the added license with colors
            info_log!("Added license to: {}", path.display());

            // Add to report if collecting report data
            if self.collect_report_data {
                let file_report = FileReport {
                    path: path.to_path_buf(),
                    has_license: true, // Now it has a license
                    action_taken: Some(FileAction::Added),
                    ignored: false,
                    ignored_reason: None,
                };

                if let Ok(mut reports) = self.file_reports.lock() {
                    reports.push(file_report);
                }
            }
        }

        Ok(())
    }

    /// Checks if the content already has a license header.
    ///
    /// This method examines the first 1000 characters of the content to determine
    /// if it already contains a license header.
    ///
    /// # Parameters
    ///
    /// * `content` - The file content to check
    ///
    /// # Returns
    ///
    /// `true` if the content appears to have a license header, `false` otherwise.
    ///
    /// # Implementation Details
    ///
    /// The check is case-insensitive and only examines the first 1000 characters
    /// of the file for performance reasons, as license headers are typically
    /// at the beginning of files.
    pub fn has_license(&self, content: &str) -> bool {
        // Take the first 1000 characters (or less if the file is shorter)
        let check_len = std::cmp::min(content.len(), 1000);
        let check_content = &content[..check_len];

        // Convert to lowercase for case-insensitive matching
        let check_content_lower = check_content.to_lowercase();

        // Based on addlicense's implementation, we check for common license indicators
        // without requiring the specific year (which is too strict)
        check_content_lower.contains("copyright")
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
    /// - The extracted prefix as a String (with added newlines for proper separation)
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
    pub fn update_year_in_license(&self, content: &str) -> Result<String> {
        // Regex to find copyright year patterns - match all copyright symbol formats
        let year_regex = Regex::new(r"(?i)(copyright\s+(?:\(c\)|©)?\s+)(\d{4})(\s+)")?;

        let current_year = &self.license_data.year;

        verbose_log!("Updating year to: {}", current_year);

        // Update single year to current year
        let content = year_regex
            .replace_all(content, |caps: &regex::Captures| {
                let prefix = &caps[1];
                let year = &caps[2];
                let suffix = &caps[3];

                if year != current_year {
                    verbose_log!("Replacing year {} with {}", year, current_year);
                    format!("{}{}{}", prefix, current_year, suffix)
                } else {
                    // Keep as is if already current
                    caps[0].to_string()
                }
            })
            .to_string();

        Ok(content)
    }
}

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use git2::{Repository, StatusOptions};
use glob::Pattern;
use rayon::prelude::*;
use regex::Regex;
use walkdir::WalkDir;

use crate::templates::{LicenseData, TemplateManager};
use crate::{info_log, verbose_log};

/// Processor for handling license operations on files
pub struct Processor {
    template_manager: TemplateManager,
    license_data: LicenseData,
    ignore_patterns: Vec<Pattern>,
    check_only: bool,
    preserve_years: bool,
    ratchet_reference: Option<String>,
    pub changed_files: Option<HashSet<PathBuf>>,
}

impl Processor {
    /// Create a new processor
    pub fn new(
        template_manager: TemplateManager,
        license_data: LicenseData,
        ignore_patterns: Vec<String>,
        check_only: bool,
        preserve_years: bool,
        ratchet_reference: Option<String>,
    ) -> Result<Self> {
        // Compile glob patterns
        let ignore_patterns = ignore_patterns
            .into_iter()
            .map(|p| Pattern::new(&p))
            .collect::<Result<Vec<_>, _>>()
            .with_context(|| "Invalid glob pattern")?;

        // Initialize changed_files if ratchet mode is enabled
        let changed_files = if let Some(ref reference) = ratchet_reference {
            Some(Self::get_changed_files(reference)?)
        } else {
            None
        };

        Ok(Self {
            template_manager,
            license_data,
            ignore_patterns,
            check_only,
            preserve_years,
            ratchet_reference,
            changed_files,
        })
    }

    /// Get the list of files that have changed relative to a git reference
    fn get_changed_files(reference: &str) -> Result<HashSet<PathBuf>> {
        verbose_log!("Getting changed files relative to: {}", reference);

        // Open the git repository
        let repo = Repository::open(".").with_context(|| "Failed to open git repository")?;

        // Get the reference commit
        let reference_obj = repo
            .revparse_single(reference)
            .with_context(|| format!("Failed to find git reference: {}", reference))?;

        let reference_commit = reference_obj
            .peel_to_commit()
            .with_context(|| format!("Failed to get commit for reference: {}", reference))?;

        // Create a diff between the reference commit and the working directory
        let reference_tree = reference_commit
            .tree()
            .with_context(|| "Failed to get tree for reference commit")?;

        let mut changed_files = HashSet::new();

        // Get the status of files in the working directory
        let mut status_opts = StatusOptions::new();
        status_opts.include_untracked(true);

        let statuses = repo
            .statuses(Some(&mut status_opts))
            .with_context(|| "Failed to get git status")?;

        // Add all changed files to the set
        for entry in statuses.iter() {
            if let Some(path) = entry.path() {
                let status = entry.status();

                // Check if the file is modified, added, or untracked
                if status.is_wt_modified()
                    || status.is_wt_new()
                    || status.is_wt_renamed()
                    || status.is_index_modified()
                    || status.is_index_new()
                    || status.is_index_renamed()
                {
                    verbose_log!("Changed file: {}", path);
                    changed_files.insert(PathBuf::from(path));
                }
            }
        }

        // Also check for files that have been modified between the reference and HEAD
        let head_obj = repo.head().with_context(|| "Failed to get HEAD reference")?;

        let head_commit = head_obj.peel_to_commit().with_context(|| "Failed to get HEAD commit")?;

        let head_tree = head_commit
            .tree()
            .with_context(|| "Failed to get tree for HEAD commit")?;

        let diff = repo
            .diff_tree_to_tree(Some(&reference_tree), Some(&head_tree), None)
            .with_context(|| "Failed to create diff between reference and HEAD")?;

        diff.foreach(
            &mut |delta, _| {
                if let Some(new_file) = delta.new_file().path() {
                    verbose_log!("Changed file (in diff): {:?}", new_file);
                    changed_files.insert(PathBuf::from(new_file));
                }
                true
            },
            None,
            None,
            None,
        )
        .with_context(|| "Failed to process diff")?;

        verbose_log!("Found {} changed files", changed_files.len());

        Ok(changed_files)
    }

    /// Process a list of file or directory patterns
    pub fn process(&self, patterns: &[String]) -> Result<bool> {
        let has_missing_license = Arc::new(AtomicBool::new(false));

        // Process each pattern
        for pattern in patterns {
            // Check if the pattern is a file or directory
            let path = PathBuf::from(pattern);
            if path.is_file() {
                // Process a single file
                let result = self.process_file(&path);
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
                                let result = self.process_file(&path);
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

    /// Process a directory recursively
    pub fn process_directory(&self, dir: &Path) -> Result<bool> {
        let has_missing_license = Arc::new(AtomicBool::new(false));

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
                let should_ignore = self.should_ignore(p);
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
                eprintln!("Error processing {}: {}", path.display(), e);
                has_missing_license.store(true, Ordering::Relaxed);
            }
        });

        Ok(has_missing_license.load(Ordering::Relaxed))
    }

    /// Process a single file
    pub fn process_file(&self, path: &Path) -> Result<()> {
        verbose_log!("Processing file: {}", path.display());

        // Skip files that match ignore patterns
        if self.should_ignore(path) {
            verbose_log!("Skipping: {} (matches ignore pattern)", path.display());
            return Ok(());
        }

        // Skip files that haven't changed in ratchet mode
        if let Some(ref changed_files) = self.changed_files {
            if !changed_files.contains(path) {
                verbose_log!("Skipping: {} (unchanged in ratchet mode)", path.display());
                return Ok(());
            }
            verbose_log!("Processing: {} (changed in ratchet mode)", path.display());
        }

        // Read file content
        let content = fs::read_to_string(path).with_context(|| format!("Failed to read file: {}", path.display()))?;

        // Check if the file already has a license
        let has_license = self.has_license(&content);
        verbose_log!("File has license: {}", has_license);

        if self.check_only {
            if !has_license {
                info_log!("{}", path.display());
                return Err(anyhow::anyhow!("Missing license header"));
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
                    info_log!("Updated: {}", path.display());
                    fs::write(path, updated_content)
                        .with_context(|| format!("Failed to write to file: {}", path.display()))?;
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
            info_log!("Added license to: {}", path.display());

            // Write the updated content back to the file
            fs::write(path, new_content).with_context(|| format!("Failed to write to file: {}", path.display()))?;
        }

        Ok(())
    }

    /// Check if a file should be ignored based on ignore patterns
    pub fn should_ignore(&self, path: &Path) -> bool {
        if let Some(path_str) = path.to_str() {
            // Convert to a relative path string for matching
            let path_str = path_str.replace("\\", "/"); // Normalize for Windows paths
            
            for pattern in &self.ignore_patterns {
                // Try matching the pattern against the path
                if pattern.matches(&path_str) {
                    verbose_log!("Skipping: {} (matches ignore pattern: {})", path.display(), pattern);
                    return true;
                }
                
                // Also try matching with ./ prefix for relative paths
                if path_str.starts_with("./") {
                    if pattern.matches(&path_str[2..]) {
                        verbose_log!("Skipping: {} (matches ignore pattern: {})", path.display(), pattern);
                        return true;
                    }
                } else {
                    // Try with ./ prefix added
                    let with_prefix = format!("./{}", path_str);
                    if pattern.matches(&with_prefix) {
                        verbose_log!("Skipping: {} (matches ignore pattern: {})", path.display(), pattern);
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check if content already has a license header
    pub fn has_license(&self, content: &str) -> bool {
        // Take the first 1000 characters (or less if the file is shorter)
        let check_len = std::cmp::min(content.len(), 1000);
        let check_content = &content[..check_len].to_lowercase();

        check_content.contains("copyright")
            || check_content.contains("mozilla public")
            || check_content.contains("spdx-license-identifier")
            || self.is_generated(content)
    }

    /// Check if the file is generated
    pub fn is_generated(&self, content: &str) -> bool {
        // Common patterns for generated files
        let go_generated = Regex::new(r"(?m)^.{1,2} Code generated .* DO NOT EDIT\.$").unwrap();
        let cargo_raze = Regex::new(r"(?m)^DO NOT EDIT! Replaced on runs of cargo-raze$").unwrap();

        go_generated.is_match(content) || cargo_raze.is_match(content)
    }

    /// Extract prefix (like shebang) from content
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

    /// Update year in existing license
    pub fn update_year_in_license(&self, content: &str) -> Result<String> {
        // Regex to find copyright year patterns - match all copyright symbol formats
        // Added (?i) flag to make the regex case-insensitive
        // Modified to handle all copyright symbol formats: (c), ©, or no symbol at all
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

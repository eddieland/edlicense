//! # Diff Module
//!
//! This module contains functionality for creating and rendering diffs between original and modified content.
//! It's used primarily for showing what would be changed when adding or updating license headers.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Result;
use similar::{ChangeTag, TextDiff};

/// Manages diff creation and rendering for license header changes.
///
/// This struct handles:
/// - Generating diffs between original and modified content
/// - Displaying diffs to stderr with colorization
/// - Saving diffs to a file
pub struct DiffManager {
  /// Whether to show diffs in dry run mode
  pub show_diff: bool,

  /// Path to save the diff to in dry run mode
  pub save_diff_path: Option<PathBuf>,
}

impl DiffManager {
  /// Creates a new DiffManager with the specified configuration.
  ///
  /// # Parameters
  ///
  /// * `show_diff` - Whether to show diffs in dry run mode
  /// * `save_diff_path` - Path to save the diff to in dry run mode
  ///
  /// # Returns
  ///
  /// A new `DiffManager` instance.
  pub fn new(show_diff: bool, save_diff_path: Option<PathBuf>) -> Self {
    Self {
      show_diff,
      save_diff_path,
    }
  }

  /// Displays and/or saves a diff between the original and new content.
  ///
  /// This method uses the `similar` crate to generate a diff showing what would be changed in the file.
  ///
  /// If show_diff is enabled, the diff will be displayed to stderr with colorization.
  /// If save_diff_path is provided, the diff will be saved to that file.
  /// Multiple diffs from different files will be appended to the same file,
  /// creating a single consolidated diff file.
  ///
  /// # Parameters
  ///
  /// * `path` - Path to the file being processed
  /// * `original` - Original file content
  /// * `new` - New file content with license header
  pub fn display_diff(&self, path: &Path, original: &str, new: &str) -> Result<()> {
    // Only print to stderr if show_diff is enabled
    if self.show_diff {
      eprintln!("Diff for {}:", path.display());
    }

    let diff = TextDiff::from_lines(original, new);

    // Create a string to hold the diff content for saving to file
    let mut diff_content = String::new();

    // Add a header for this file's diff
    diff_content.push_str(&format!("Diff for {}:\n", path.display()));

    for change in diff.iter_all_changes() {
      let sign = match change.tag() {
        ChangeTag::Delete => "-",
        ChangeTag::Insert => "+",
        ChangeTag::Equal => " ",
      };

      // Print to stderr for console output if show_diff is enabled
      if self.show_diff {
        eprint!("{}{}", sign, change);
      }

      // Add to diff_content for file output
      diff_content.push_str(&format!("{}{}", sign, change));
    }

    // Only print newline if show_diff is enabled
    if self.show_diff {
      eprintln!();
    }

    diff_content.push('\n');

    // If save_diff_path is provided, append the diff to the file
    if let Some(ref diff_path) = self.save_diff_path {
      // Create or append to the diff file
      let file_result = OpenOptions::new().create(true).append(true).open(diff_path);

      match file_result {
        Ok(mut file) => {
          if let Err(e) = file.write_all(diff_content.as_bytes()) {
            eprintln!("Error writing to diff file: {}", e);
          }
        }
        Err(e) => {
          eprintln!("Error opening diff file: {}", e);
        }
      }
    }

    Ok(())
  }
}

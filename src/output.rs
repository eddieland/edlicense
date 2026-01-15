//! # Output Module
//!
//! This module centralizes all user-facing output for the edlicense tool.
//! It provides consistent formatting, colors, and symbols for terminal output.
//!
//! ## Design Goals
//!
//! - **Informative**: Show actionable information without requiring flags
//! - **Scannable**: Use formatting to make output easy to parse visually
//! - **Progressive**: More detail with `-v`, silence with `-q`
//! - **Scriptable**: Keep stdout predictable for piping/automation

use std::path::Path;

use owo_colors::{OwoColorize, Stream};

use crate::logging::{is_quiet, is_verbose};
use crate::report::{FileAction, FileReport, ProcessingSummary};

/// Symbols used in output
pub mod symbols {
  /// Success/has license
  pub const SUCCESS: &str = "\u{2713}"; // ✓
  /// Missing license/failure
  pub const FAILURE: &str = "\u{2717}"; // ✗
  /// Ignored/skipped (used in verbose mode - Phase 2)
  #[allow(dead_code)]
  pub const IGNORED: &str = "-";
  /// Year updated
  pub const UPDATED: &str = "\u{21bb}"; // ↻
}

/// Maximum number of files to show in the default output before truncating
const DEFAULT_FILE_LIST_LIMIT: usize = 20;

/// Print the initial "Checking N files..." or "Processing N files..." message.
///
/// - In modify mode: "Processing N files..."
/// - In check mode: "Checking N files..."
pub fn print_start_message(file_count: usize, modify_mode: bool) {
  if is_quiet() {
    return;
  }

  let verb = if modify_mode { "Processing" } else { "Checking" };
  let files_word = if file_count == 1 { "file" } else { "files" };

  println!("{} {} {}...", verb, file_count, files_word);
}

/// Print a blank line for visual separation (respects quiet mode).
pub fn print_blank_line() {
  if !is_quiet() {
    println!();
  }
}

/// Print the list of files missing license headers.
///
/// Shows up to `limit` files (or `DEFAULT_FILE_LIST_LIMIT` if None).
/// In verbose mode, shows all files.
/// Files are sorted alphabetically by path.
pub fn print_missing_files(files: &[&FileReport], workspace_root: Option<&Path>, limit: Option<usize>) {
  if files.is_empty() {
    return;
  }

  // Sort files alphabetically by path
  let mut sorted_files: Vec<_> = files.to_vec();
  sorted_files.sort_by(|a, b| a.path.cmp(&b.path));

  if is_quiet() {
    // In quiet mode, just print the file paths (for scripting)
    for file in &sorted_files {
      let display_path = make_relative_path(&file.path, workspace_root);
      println!("{}", display_path);
    }
    return;
  }

  let count = sorted_files.len();
  let header = format!(
    "{} {} {} missing license headers:",
    symbols::FAILURE.if_supports_color(Stream::Stdout, |s| s.red()),
    count,
    if count == 1 { "file" } else { "files" }
  );
  println!("{}", header);

  let show_all = is_verbose();
  let effective_limit = if show_all {
    count
  } else {
    limit.unwrap_or(DEFAULT_FILE_LIST_LIMIT)
  };

  for file in sorted_files.iter().take(effective_limit) {
    let display_path = make_relative_path(&file.path, workspace_root);
    println!("  {}", display_path);
  }

  if !show_all && count > effective_limit {
    let remaining = count - effective_limit;
    println!(
      "  {} ... and {} more (use -v to see all)",
      "".if_supports_color(Stream::Stdout, |s| s.dimmed()),
      remaining
    );
  }
}

/// Print the list of files with outdated license years.
///
/// Shows up to `limit` files (or `DEFAULT_FILE_LIST_LIMIT` if None).
/// In verbose mode, shows all files.
/// Files are sorted alphabetically by path.
pub fn print_outdated_files(files: &[&FileReport], workspace_root: Option<&Path>, limit: Option<usize>) {
  if files.is_empty() {
    return;
  }

  // Sort files alphabetically by path
  let mut sorted_files: Vec<_> = files.to_vec();
  sorted_files.sort_by(|a, b| a.path.cmp(&b.path));

  if is_quiet() {
    // In quiet mode, just print the file paths (for scripting)
    for file in &sorted_files {
      let display_path = make_relative_path(&file.path, workspace_root);
      println!("{}", display_path);
    }
    return;
  }

  let count = sorted_files.len();
  let header = format!(
    "{} {} {} with outdated year:",
    symbols::UPDATED.if_supports_color(Stream::Stdout, |s| s.yellow()),
    count,
    if count == 1 { "file" } else { "files" }
  );
  println!("{}", header);

  let show_all = is_verbose();
  let effective_limit = if show_all {
    count
  } else {
    limit.unwrap_or(DEFAULT_FILE_LIST_LIMIT)
  };

  for file in sorted_files.iter().take(effective_limit) {
    let display_path = make_relative_path(&file.path, workspace_root);
    println!("  {}", display_path);
  }

  if !show_all && count > effective_limit {
    let remaining = count - effective_limit;
    println!(
      "  {} ... and {} more (use -v to see all)",
      "".if_supports_color(Stream::Stdout, |s| s.dimmed()),
      remaining
    );
  }
}

/// Print the list of files that had licenses added.
pub fn print_added_files(files: &[&FileReport], workspace_root: Option<&Path>) {
  if is_quiet() || files.is_empty() {
    return;
  }

  let count = files.len();
  let header = format!(
    "{} Added license to {} {}:",
    symbols::SUCCESS.if_supports_color(Stream::Stdout, |s| s.green()),
    count,
    if count == 1 { "file" } else { "files" }
  );
  println!("{}", header);

  let show_all = is_verbose();
  let limit = if show_all { count } else { DEFAULT_FILE_LIST_LIMIT };

  for file in files.iter().take(limit) {
    let display_path = make_relative_path(&file.path, workspace_root);
    println!("  {}", display_path);
  }

  if !show_all && count > limit {
    let remaining = count - limit;
    println!(
      "  {} ... and {} more (use -v to see all)",
      "".if_supports_color(Stream::Stdout, |s| s.dimmed()),
      remaining
    );
  }
}

/// Print the list of files that had years updated.
pub fn print_updated_files(files: &[&FileReport], workspace_root: Option<&Path>) {
  if is_quiet() || files.is_empty() {
    return;
  }

  let count = files.len();
  let header = format!(
    "{} Updated year in {} {}:",
    symbols::UPDATED.if_supports_color(Stream::Stdout, |s| s.yellow()),
    count,
    if count == 1 { "file" } else { "files" }
  );
  println!("{}", header);

  let show_all = is_verbose();
  let limit = if show_all { count } else { DEFAULT_FILE_LIST_LIMIT };

  for file in files.iter().take(limit) {
    let display_path = make_relative_path(&file.path, workspace_root);
    println!("  {}", display_path);
  }

  if !show_all && count > limit {
    let remaining = count - limit;
    println!(
      "  {} ... and {} more (use -v to see all)",
      "".if_supports_color(Stream::Stdout, |s| s.dimmed()),
      remaining
    );
  }
}

/// Print the success message when all files have license headers.
pub fn print_all_files_ok() {
  if is_quiet() {
    return;
  }

  println!(
    "{} All files have license headers.",
    symbols::SUCCESS.if_supports_color(Stream::Stdout, |s| s.green())
  );
}

/// Print the processing summary.
///
/// Format: "Summary: X OK, Y missing, Z ignored"
/// In verbose mode, also shows timing.
pub fn print_summary(summary: &ProcessingSummary) {
  if is_quiet() {
    return;
  }

  let ok_count = summary.files_with_license;
  let missing_count = summary.files_without_license;
  let ignored_count = summary.files_ignored;

  let ok_str = ok_count.if_supports_color(Stream::Stdout, |s| s.cyan());
  let missing_str = if missing_count > 0 {
    missing_count.if_supports_color(Stream::Stdout, |s| s.red()).to_string()
  } else {
    missing_count
      .if_supports_color(Stream::Stdout, |s| s.cyan())
      .to_string()
  };
  let ignored_str = ignored_count.if_supports_color(Stream::Stdout, |s| s.dimmed());

  let mut summary_line = format!(
    "Summary: {} OK, {} missing, {} ignored",
    ok_str, missing_str, ignored_str
  );

  // Show timing in verbose mode
  if is_verbose() {
    summary_line.push_str(&format!(" ({:.2}s)", summary.processing_time.as_secs_f64()));
  }

  println!("{}", summary_line);
}

/// Print a hint for the user about what to do next.
pub fn print_hint(message: &str) {
  if is_quiet() {
    return;
  }

  println!("{}", message.if_supports_color(Stream::Stdout, |s| s.yellow()));
}

/// Print verbose per-file status during processing.
/// Only shown in verbose mode.
///
/// Note: This function is planned for Phase 2 (verbose mode improvements).
#[allow(dead_code)]
pub fn print_file_status_verbose(path: &Path, status: FileStatus, workspace_root: Option<&Path>) {
  if !is_verbose() {
    return;
  }

  let display_path = make_relative_path(path, workspace_root);
  let (symbol, message) = match status {
    FileStatus::HasLicense => (
      symbols::SUCCESS
        .if_supports_color(Stream::Stdout, |s| s.green())
        .to_string(),
      display_path,
    ),
    FileStatus::MissingLicense => (
      symbols::FAILURE
        .if_supports_color(Stream::Stdout, |s| s.red())
        .to_string(),
      format!("{} (missing header)", display_path),
    ),
    FileStatus::LicenseAdded => (
      symbols::SUCCESS
        .if_supports_color(Stream::Stdout, |s| s.green())
        .to_string(),
      format!("{} (added)", display_path),
    ),
    FileStatus::YearUpdated => (
      symbols::UPDATED
        .if_supports_color(Stream::Stdout, |s| s.yellow())
        .to_string(),
      format!("{} (year updated)", display_path),
    ),
    FileStatus::Ignored(reason) => (
      symbols::IGNORED
        .if_supports_color(Stream::Stdout, |s| s.dimmed())
        .to_string(),
      format!(
        "{} (ignored: {})",
        display_path.if_supports_color(Stream::Stdout, |s| s.dimmed()),
        reason
      ),
    ),
  };

  println!("  {} {}", symbol, message);
}

/// Status of a file for verbose output.
///
/// Note: This enum is planned for Phase 2 (verbose mode improvements).
#[allow(dead_code)]
pub enum FileStatus<'a> {
  /// File already has a license header
  HasLicense,
  /// File is missing a license header
  MissingLicense,
  /// License was added to the file
  LicenseAdded,
  /// Year was updated in the file
  YearUpdated,
  /// File was ignored with a reason
  Ignored(&'a str),
}

/// Categorize file reports into different groups for output.
#[allow(dead_code)] // Some fields are used in Phase 2 (verbose mode)
pub struct CategorizedReports<'a> {
  /// Files missing license headers (not ignored, no license)
  pub missing: Vec<&'a FileReport>,
  /// Files that had licenses added
  pub added: Vec<&'a FileReport>,
  /// Files that had years updated
  pub updated: Vec<&'a FileReport>,
  /// Files that already had correct licenses (used in verbose mode - Phase 2)
  pub ok: Vec<&'a FileReport>,
  /// Files that were ignored (used in verbose mode - Phase 2)
  pub ignored: Vec<&'a FileReport>,
}

impl<'a> CategorizedReports<'a> {
  /// Categorize a slice of file reports.
  pub fn from_reports(reports: &'a [FileReport]) -> Self {
    let mut missing = Vec::new();
    let mut added = Vec::new();
    let mut updated = Vec::new();
    let mut ok = Vec::new();
    let mut ignored = Vec::new();

    for report in reports {
      if report.ignored {
        ignored.push(report);
        continue;
      }

      match &report.action_taken {
        Some(FileAction::Added) => added.push(report),
        Some(FileAction::YearUpdated) => updated.push(report),
        Some(FileAction::NoActionNeeded) => ok.push(report),
        Some(FileAction::Skipped) => ignored.push(report),
        None => {
          if report.has_license {
            ok.push(report);
          } else {
            missing.push(report);
          }
        }
      }
    }

    Self {
      missing,
      added,
      updated,
      ok,
      ignored,
    }
  }
}

/// Make a path relative to the workspace root for display.
fn make_relative_path(path: &Path, workspace_root: Option<&Path>) -> String {
  if let Some(root) = workspace_root {
    path
      .strip_prefix(root)
      .map(|p| p.to_string_lossy().to_string())
      .unwrap_or_else(|_| path.to_string_lossy().to_string())
  } else {
    path.to_string_lossy().to_string()
  }
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use super::*;
  use crate::report::FileAction;

  fn create_test_report(path: &str, has_license: bool, action: Option<FileAction>, ignored: bool) -> FileReport {
    FileReport {
      path: PathBuf::from(path),
      has_license,
      action_taken: action,
      ignored,
      ignored_reason: if ignored { Some("test".to_string()) } else { None },
    }
  }

  #[test]
  fn test_categorize_reports_missing() {
    let reports = vec![create_test_report("src/main.rs", false, None, false)];

    let categorized = CategorizedReports::from_reports(&reports);

    assert_eq!(categorized.missing.len(), 1);
    assert!(categorized.added.is_empty());
    assert!(categorized.updated.is_empty());
    assert!(categorized.ok.is_empty());
    assert!(categorized.ignored.is_empty());
  }

  #[test]
  fn test_categorize_reports_added() {
    let reports = vec![create_test_report("src/main.rs", false, Some(FileAction::Added), false)];

    let categorized = CategorizedReports::from_reports(&reports);

    assert!(categorized.missing.is_empty());
    assert_eq!(categorized.added.len(), 1);
    assert!(categorized.updated.is_empty());
    assert!(categorized.ok.is_empty());
    assert!(categorized.ignored.is_empty());
  }

  #[test]
  fn test_categorize_reports_updated() {
    let reports = vec![create_test_report(
      "src/main.rs",
      true,
      Some(FileAction::YearUpdated),
      false,
    )];

    let categorized = CategorizedReports::from_reports(&reports);

    assert!(categorized.missing.is_empty());
    assert!(categorized.added.is_empty());
    assert_eq!(categorized.updated.len(), 1);
    assert!(categorized.ok.is_empty());
    assert!(categorized.ignored.is_empty());
  }

  #[test]
  fn test_categorize_reports_ok() {
    let reports = vec![create_test_report(
      "src/main.rs",
      true,
      Some(FileAction::NoActionNeeded),
      false,
    )];

    let categorized = CategorizedReports::from_reports(&reports);

    assert!(categorized.missing.is_empty());
    assert!(categorized.added.is_empty());
    assert!(categorized.updated.is_empty());
    assert_eq!(categorized.ok.len(), 1);
    assert!(categorized.ignored.is_empty());
  }

  #[test]
  fn test_categorize_reports_ignored() {
    let reports = vec![create_test_report("src/main.rs", false, None, true)];

    let categorized = CategorizedReports::from_reports(&reports);

    assert!(categorized.missing.is_empty());
    assert!(categorized.added.is_empty());
    assert!(categorized.updated.is_empty());
    assert!(categorized.ok.is_empty());
    assert_eq!(categorized.ignored.len(), 1);
  }

  #[test]
  fn test_categorize_reports_mixed() {
    let reports = vec![
      create_test_report("src/main.rs", true, Some(FileAction::NoActionNeeded), false),
      create_test_report("src/new.rs", false, None, false),
      create_test_report("src/added.rs", false, Some(FileAction::Added), false),
      create_test_report("src/updated.rs", true, Some(FileAction::YearUpdated), false),
      create_test_report("src/ignored.rs", false, None, true),
    ];

    let categorized = CategorizedReports::from_reports(&reports);

    assert_eq!(categorized.ok.len(), 1);
    assert_eq!(categorized.missing.len(), 1);
    assert_eq!(categorized.added.len(), 1);
    assert_eq!(categorized.updated.len(), 1);
    assert_eq!(categorized.ignored.len(), 1);
  }

  #[test]
  fn test_make_relative_path_with_root() {
    let path = PathBuf::from("/workspace/project/src/main.rs");
    let root = PathBuf::from("/workspace/project");

    let result = make_relative_path(&path, Some(&root));
    assert_eq!(result, "src/main.rs");
  }

  #[test]
  fn test_make_relative_path_without_root() {
    let path = PathBuf::from("/workspace/project/src/main.rs");

    let result = make_relative_path(&path, None);
    assert_eq!(result, "/workspace/project/src/main.rs");
  }
}

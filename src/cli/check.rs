//! # Check Command
//!
//! This module implements the check/modify command for license headers.
//! This is the default command when no subcommand is specified.

use std::path::PathBuf;
use std::process;
use std::time::Instant;

use anyhow::{Context, Result};
use chrono::Datelike;
use clap::Args;

use crate::diff::DiffManager;
use crate::logging::{ColorMode, init_tracing, set_quiet, set_verbose};
use crate::processor::Processor;
use crate::report::{ProcessingSummary, ReportFormat, ReportGenerator};
use crate::templates::{LicenseData, TemplateManager};
use crate::workspace::resolve_workspace;
use crate::{info_log, verbose_log};

/// Arguments for the check command
#[derive(Args, Debug, Default)]
pub struct CheckArgs {
  /// File or directory patterns to process. Directories are processed
  /// recursively.
  #[arg(required = false)]
  pub patterns: Vec<String>,

  /// Dry run mode: only check for license headers without modifying files
  /// (default)
  #[arg(long, group = "mode", hide = true)]
  pub dry_run: bool,

  /// Modify mode: add or update license headers in files
  #[arg(
    long,
    group = "mode",
    help = "Modify mode: add or update license headers in files

[default: --dry-run]"
  )]
  pub modify: bool,

  /// Show diff of changes in dry run mode
  #[arg(long)]
  pub show_diff: bool,

  /// Save diff of changes to a file in dry run mode
  #[arg(long, short = 'o', value_name = "FILE")]
  pub save_diff: Option<PathBuf>,

  /// Custom license file to use
  #[arg(long, short = 'f', required = false, value_name = "FILE")]
  pub license_file: Option<PathBuf>,

  /// File patterns to ignore (supports glob patterns)
  #[arg(long, short = 'i')]
  pub ignore: Vec<String>,

  /// Copyright year(s)
  #[arg(long)]
  pub year: Option<String>,

  /// Increase verbosity (-v info, -vv debug, -vvv trace)
  #[arg(short, long, action = clap::ArgAction::Count)]
  pub verbose: u8,

  /// Suppress all output except errors
  #[arg(short, long, conflicts_with = "verbose")]
  pub quiet: bool,

  /// Preserve existing years in license headers
  #[arg(long)]
  pub preserve_years: bool,

  /// Ratchet mode: only check and format files that have changed relative to a
  /// git reference
  #[arg(long, value_name = "REF")]
  pub ratchet: Option<String>,

  /// Path to a global license ignore file (overrides GLOBAL_LICENSE_IGNORE
  /// environment variable)
  #[arg(long, value_name = "FILE")]
  pub global_ignore_file: Option<PathBuf>,

  /// Only consider files in the current git repository
  #[arg(long, default_value = "false", default_missing_value = "true", num_args = 0..=1)]
  pub git_only: Option<bool>,

  /// Control when to use colored output (auto, never, always)
  #[arg(
    long,
    value_name = "WHEN",
    num_args = 0..=1,
    default_value_t = ColorMode::Auto,
    default_missing_value = "always",
    value_enum
  )]
  pub colors: ColorMode,

  /// Generate an HTML report of license status and save to the specified path
  #[arg(long, value_name = "OUTPUT")]
  pub report_html: Option<PathBuf>,

  /// Generate a JSON report of license status and save to the specified path
  #[arg(long, value_name = "OUTPUT")]
  pub report_json: Option<PathBuf>,

  /// Generate a CSV report of license status and save to the specified path
  #[arg(long, value_name = "OUTPUT")]
  pub report_csv: Option<PathBuf>,

  /// Skip git repository ownership check. Useful when running in Docker or
  /// other containerized environments where the repository may be owned by a
  /// different user.
  #[arg(long)]
  pub skip_git_owner_check: bool,
}

impl CheckArgs {
  /// Validate the arguments and return an error if invalid
  fn validate(&self) -> Result<(), String> {
    if self.patterns.is_empty() {
      return Err("Missing required argument: <PATTERNS>...".to_string());
    }
    if self.license_file.is_none() {
      return Err("Missing required argument: --license-file <FILE>".to_string());
    }
    Ok(())
  }
}

/// Run the check command with the given arguments
pub async fn run_check(args: CheckArgs) -> Result<()> {
  // Validate arguments
  if let Err(e) = args.validate() {
    eprintln!("ERROR: {e}");
    process::exit(1);
  }

  // Initialize tracing subscriber for structured logging
  init_tracing(args.quiet, args.verbose);

  // Set legacy logging mode for existing verbose_log!/info_log! macros
  if args.verbose > 0 {
    set_verbose();
  } else if args.quiet {
    set_quiet();
  }
  args.colors.apply();

  // Disable git ownership check if requested (useful in Docker)
  if args.skip_git_owner_check {
    verbose_log!("Disabling git repository ownership check");
    // SAFETY: This is safe to call as long as no git operations are in progress.
    // We call this early, before any Repository operations.
    unsafe {
      let _ = git2::opts::set_verify_owner_validation(false);
    }
  }

  // Set global ignore file if provided
  if let Some(ref global_ignore_file) = args.global_ignore_file {
    if let Some(path_str) = global_ignore_file.to_str() {
      // SAFETY:
      // This is safe because we control the lifetime of the program
      unsafe {
        std::env::set_var("GLOBAL_LICENSE_IGNORE", path_str);
      }
      verbose_log!("Setting GLOBAL_LICENSE_IGNORE to {}", global_ignore_file.display());
    } else {
      eprintln!("Warning: Could not convert global ignore file path to string");
    }
  }

  let year = match args.year {
    Some(ref y) => y.clone(),
    None => chrono::Local::now().year().to_string(),
  };

  let license_data = LicenseData { year };

  // Safe to unwrap because we validated above
  let license_file = args.license_file.as_ref().expect("a license file");

  let mut template_manager = TemplateManager::new();
  template_manager
    .load_template(license_file)
    .with_context(|| format!("Failed to load license template from {}", license_file.display()))?;

  // Determine mode (dry run is default if neither is specified or if dry_run is
  // explicitly set)
  let check_only = args.dry_run || !args.modify;

  let diff_manager = DiffManager::new(args.show_diff, args.save_diff);

  let workspace = resolve_workspace(&args.patterns)?;
  let workspace_root = workspace.root().to_path_buf();

  let git_only = args.git_only.unwrap_or(false);
  if git_only {
    if workspace.is_git() {
      info_log!("Git repository detected, only processing tracked files");
      verbose_log!("Using workspace root: {}", workspace_root.display());
    } else {
      eprintln!("ERROR: Git-only mode is enabled, but not in a git repository");
      eprintln!("When --git-only is enabled, you must run edlicense from inside a git repository");
      process::exit(1);
    }
  }

  let processor = Processor::new(
    template_manager,
    license_data,
    args.ignore,
    check_only,
    args.preserve_years,
    args.ratchet,
    Some(diff_manager),
    git_only,
    None, // Use the default LicenseDetector implementation
    workspace_root,
    workspace.is_git(),
  )?;

  // Start timing
  let start_time = Instant::now();

  let has_missing_license = processor.process(&args.patterns).await?;

  // Calculate elapsed time
  let elapsed = start_time.elapsed();

  // Get the total number of files processed
  let files_processed = processor.files_processed.load(std::sync::atomic::Ordering::Relaxed);

  // Log the results
  if files_processed == 1 {
    info_log!(
      "Processed {} file in {:.2} seconds",
      files_processed,
      elapsed.as_secs_f64()
    );
  } else {
    info_log!(
      "Processed {} files in {:.2} seconds",
      files_processed,
      elapsed.as_secs_f64()
    );
  }

  // Get file reports from processor for report generation
  let file_reports = processor.file_reports.lock().await.clone();

  // Create report summary
  let summary = ProcessingSummary::from_reports(&file_reports, elapsed);

  // Generate HTML report if requested
  if let Some(output_path) = args.report_html {
    let report_generator = ReportGenerator::new(ReportFormat::Html, output_path.clone());
    if let Err(e) = report_generator.generate(&file_reports, &summary) {
      eprintln!("Error generating HTML report: {}", e);
    } else {
      info_log!("Generated HTML report at {}", output_path.display());
    }
  }

  // Generate JSON report if requested
  if let Some(output_path) = args.report_json {
    let report_generator = ReportGenerator::new(ReportFormat::Json, output_path.clone());
    if let Err(e) = report_generator.generate(&file_reports, &summary) {
      eprintln!("Error generating JSON report: {}", e);
    } else {
      info_log!("Generated JSON report at {}", output_path.display());
    }
  }

  // Generate CSV report if requested
  if let Some(output_path) = args.report_csv {
    let report_generator = ReportGenerator::new(ReportFormat::Csv, output_path.clone());
    if let Err(e) = report_generator.generate(&file_reports, &summary) {
      eprintln!("Error generating CSV report: {}", e);
    } else {
      info_log!("Generated CSV report at {}", output_path.display());
    }
  }

  // Exit with non-zero code if in dry run mode and there are missing licenses
  if check_only && has_missing_license {
    eprintln!("Error: Some files are missing license headers");
    process::exit(1);
  }

  Ok(())
}

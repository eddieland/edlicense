//! # Main Module
//!
//! This module contains the CLI implementation with Clap.

mod diff;
mod file_filter;
mod git;
mod ignore;
mod license_detection;
mod logging;
mod processor;
mod report;
mod templates;

use std::path::PathBuf;
use std::process;
use std::time::Instant;

use anyhow::{Context, Result};
use chrono::Datelike;
use clap::Parser;
use clap::builder::styling::{AnsiColor, Color, Style, Styles};

use crate::diff::DiffManager;
use crate::logging::{ColorMode, set_color_mode, set_verbose};
use crate::processor::Processor;
use crate::report::{ProcessingSummary, ReportFormat, ReportGenerator};
use crate::templates::{LicenseData, TemplateManager};

const CUSTOM_STYLES: Styles = Styles::styled()
  .header(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green))).bold())
  .usage(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green))).bold())
  .literal(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Blue))).bold())
  .placeholder(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Cyan))))
  .error(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Red))).bold())
  .valid(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green))))
  .invalid(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow))));

#[derive(Parser, Debug)]
#[command(
  author,
  version,
  about,
  styles = CUSTOM_STYLES,
  after_help = "Examples:
  # Check license headers without modifying files
  edlicense --license-file LICENSE.txt src/

  # Add or update license headers
  edlicense --modify --license-file custom.txt --year 2023 include/ src/

  # Show diff of potential changes without modifying files
  edlicense --show-diff --license-file LICENSE.txt src/**/*.rs

  # Save diff output to a file
  edlicense --save-diff changes.diff --license-file LICENSE.txt src/

  # Only process files changed since a specific git commit
  edlicense --ratchet=HEAD^ --license-file LICENSE.txt --modify .

  # Only process git-tracked files
  edlicense --git-only=true --license-file LICENSE.txt --modify .

  # Generate an HTML report of license status
  edlicense --report-html report.html --license-file LICENSE.txt src/

  # Ignore specific files or patterns
  edlicense --ignore \"**/vendor/**\" --ignore \"**/*.json\" --license-file LICENSE.txt src/
",
  help_template = "{before-help}{name} v{version}
{about-section}
{usage-heading} {usage}

{all-args}{after-help}
"
)]
struct Args {
  /// File or directory patterns to process. Directories are processed recursively.
  #[arg(required = true)]
  patterns: Vec<String>,

  /// Dry run mode: only check for license headers without modifying files (default)
  #[arg(long, group = "mode", hide = true)]
  dry_run: bool,

  /// Modify mode: add or update license headers in files
  #[arg(
    long,
    group = "mode",
    help = "Modify mode: add or update license headers in files

[default: --dry-run]"
  )]
  modify: bool,

  /// Show diff of changes in dry run mode
  #[arg(long)]
  show_diff: bool,

  /// Save diff of changes to a file in dry run mode
  #[arg(long, short = 'o', value_name = "FILE")]
  save_diff: Option<PathBuf>,

  /// Custom license file to use
  #[arg(long, short = 'f', required = true, value_name = "FILE")]
  license_file: PathBuf,

  /// File patterns to ignore (supports glob patterns)
  #[arg(long, short = 'i')]
  ignore: Vec<String>,

  /// Copyright year(s)
  #[arg(long)]
  year: Option<String>,

  /// Verbose mode: print names of modified files
  #[arg(long, short = 'v')]
  verbose: bool,

  /// Preserve existing years in license headers
  #[arg(long)]
  preserve_years: bool,

  /// Ratchet mode: only check and format files that have changed relative to a git reference
  #[arg(long, value_name = "REF")]
  ratchet: Option<String>,

  /// Path to a global license ignore file (overrides GLOBAL_LICENSE_IGNORE environment variable)
  #[arg(long, value_name = "FILE")]
  global_ignore_file: Option<PathBuf>,

  /// Only consider files in the current git repository
  #[arg(long, default_value = "false", default_missing_value = "true")]
  git_only: Option<bool>,

  /// Control when to use colored output (auto, never, always)
  #[arg(
    long,
    value_name = "WHEN",
    num_args = 0..=1,
    default_value_t = ColorMode::Auto,
    default_missing_value = "always",
    value_enum
  )]
  colors: ColorMode,

  /// Generate an HTML report of license status and save to the specified path
  #[arg(long, value_name = "OUTPUT")]
  report_html: Option<PathBuf>,

  /// Generate a JSON report of license status and save to the specified path
  #[arg(long, value_name = "OUTPUT")]
  report_json: Option<PathBuf>,

  /// Generate a CSV report of license status and save to the specified path
  #[arg(long, value_name = "OUTPUT")]
  report_csv: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
  let args = Args::parse();
  set_verbose(args.verbose);
  set_color_mode(args.colors);

  // Set global ignore file if provided
  if let Some(ref global_ignore_file) = args.global_ignore_file {
    if let Some(path_str) = global_ignore_file.to_str() {
      // Set the environment variable
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

  let mut template_manager = TemplateManager::new();
  template_manager
    .load_template(&args.license_file)
    .with_context(|| format!("Failed to load license template from {}", args.license_file.display()))?;

  // Determine mode (dry run is default if neither is specified or if dry_run is explicitly set)
  let check_only = args.dry_run || !args.modify;

  let diff_manager = DiffManager::new(args.show_diff, args.save_diff);

  let git_only = args.git_only.unwrap_or(false);
  if git_only {
    let is_git_repo = git::is_git_repository();

    if is_git_repo {
      info_log!("Git repository detected, only processing tracked files");
      verbose_log!("Using current working directory to determine git repository and tracked files");
    } else {
      eprintln!("ERROR: Git-only mode is enabled, but not in a git repository");
      eprintln!("When --git-only=true, you must run edlicense from inside a git repository");
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

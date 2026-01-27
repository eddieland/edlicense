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
use tracing::debug;

use crate::config::{CliOverrides, Config, load_config};
use crate::diff::DiffManager;
use crate::file_filter::ExtensionFilter;
use crate::info_log;
use crate::logging::{ColorMode, init_tracing, set_quiet, set_verbose};
use crate::output::{
  CategorizedReports, print_added_files, print_all_files_ok, print_blank_line, print_hint, print_missing_files,
  print_outdated_files, print_start_message, print_summary, print_updated_files,
};
use crate::processor::Processor;
use crate::report::{ProcessingSummary, ReportFormat, ReportGenerator};
use crate::templates::{LicenseData, TemplateManager, create_resolver};
use crate::tree::print_tree;
use crate::workspace::resolve_workspace;

/// Arguments for the check command
#[derive(Args, Debug, Default)]
pub struct CheckArgs {
  /// File or directory patterns to process. Directories are processed
  /// recursively.
  #[arg(required = false)]
  pub patterns: Vec<String>,

  /// Plan tree mode: show a tree of files that would be checked without
  /// inspecting file contents
  #[arg(long, short = 't')]
  pub plan_tree: bool,

  /// Path to config file (default: .edlicense.toml in workspace root)
  #[arg(long, value_name = "FILE")]
  pub config: Option<PathBuf>,

  /// Ignore config file even if present
  #[arg(long)]
  pub no_config: bool,

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

  /// Only process files with these extensions (repeatable, case-insensitive)
  #[arg(long, value_name = "EXT")]
  pub include_ext: Vec<String>,

  /// Exclude files with these extensions (repeatable, case-insensitive)
  #[arg(long, value_name = "EXT")]
  pub exclude_ext: Vec<String>,

  /// Override comment style for an extension (repeatable, format: EXT:STYLE)
  /// Example: --comment-style "java:// " --comment-style "xyz:# "
  #[arg(long, value_name = "EXT:STYLE")]
  pub comment_style: Vec<String>,

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

  /// Only consider files in the current git repository (default when in a git
  /// repository)
  #[arg(long, default_missing_value = "true", num_args = 0..=1)]
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
    // --license-file is not required in plan-tree mode
    if self.license_file.is_none() && !self.plan_tree {
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

  // Set verbose mode for output formatting and info_log! macro
  if args.verbose > 0 {
    set_verbose();
  } else if args.quiet {
    set_quiet();
  }
  args.colors.apply();

  // Handle plan-tree mode early - doesn't need license file
  if args.plan_tree {
    return run_plan_tree(&args).await;
  }

  // Disable git ownership check if requested (useful in Docker)
  if args.skip_git_owner_check {
    debug!("Disabling git repository ownership check");
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
      debug!("Setting GLOBAL_LICENSE_IGNORE to {}", global_ignore_file.display());
    } else {
      eprintln!("Warning: Could not convert global ignore file path to string");
    }
  }

  let year = args.year.unwrap_or_else(|| chrono::Local::now().year().to_string());

  let license_data = LicenseData { year };

  // Safe to unwrap because we validated above
  let license_file = args.license_file.as_ref().expect("a license file");

  // Determine mode (dry run is default if neither is specified or if dry_run is
  // explicitly set)
  let check_only = args.dry_run || !args.modify;

  let diff_manager = DiffManager::new(args.show_diff, args.save_diff);
  diff_manager.init()?;

  let workspace = resolve_workspace(&args.patterns)?;
  let workspace_root = workspace.root().to_path_buf();

  let git_only = args.git_only.unwrap_or_else(|| workspace.is_git());
  if git_only {
    if workspace.is_git() {
      info_log!("Git repository detected, only processing tracked files");
      debug!("Using workspace root: {}", workspace_root.display());
    } else {
      eprintln!("ERROR: Git-only mode is enabled, but not in a git repository");
      eprintln!("When --git-only is enabled, you must run edlicense from inside a git repository");
      process::exit(1);
    }
  }

  // Load configuration file if present
  let mut config = load_config(args.config.as_deref(), &workspace_root, args.no_config)?;

  if config.is_some() {
    debug!("Using configuration file for comment style overrides");
  }

  // Parse and apply CLI comment style overrides
  let cli_overrides = if !args.comment_style.is_empty() {
    match CliOverrides::from_cli_args(&args.comment_style) {
      Ok(overrides) => Some(overrides),
      Err(e) => {
        eprintln!("ERROR: {}", e);
        process::exit(1);
      }
    }
  } else {
    None
  };

  // Merge CLI overrides into config (creating default config if none exists)
  if let Some(overrides) = cli_overrides {
    let cfg = config.get_or_insert_with(Config::default);
    cfg.merge_cli_overrides(overrides);
  }

  // Create the extension filter from config and CLI args
  let extension_filter = {
    let mut filter = config
      .as_ref()
      .map(|c| ExtensionFilter::new(&c.extensions))
      .unwrap_or_else(|| ExtensionFilter::from_cli(Vec::new(), Vec::new()));

    // Merge CLI args (CLI takes precedence over config)
    filter.merge_cli(args.include_ext, args.exclude_ext);

    if filter.is_active() {
      debug!("Extension filtering is active");
      Some(filter)
    } else {
      None
    }
  };

  // Create the comment style resolver
  let resolver = create_resolver(config);

  // Create template manager with the resolver
  let mut template_manager = TemplateManager::with_resolver(resolver);
  template_manager
    .load_template(license_file)
    .with_context(|| format!("Failed to load license template from {}", license_file.display()))?;

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
    workspace_root.clone(),
    workspace.is_git(),
    extension_filter,
  )?;

  // Collect files to get count for start message.
  // When using git-list mode, collect once and reuse for processing to avoid
  // duplicate git operations.
  let collected_files: Option<Vec<std::path::PathBuf>> = if processor.should_use_git_list() {
    Some(processor.collect_files(&args.patterns)?)
  } else {
    None
  };

  let file_count = if let Some(ref files) = collected_files {
    files.len()
  } else {
    processor.collect_planned_files(&args.patterns).await?.len()
  };

  // Print start message with file count
  print_start_message(file_count, !check_only);

  // Short-circuit if no files to process
  if file_count == 0 {
    print_blank_line();
    print_all_files_ok();
    return Ok(());
  }

  // Start timing
  let start_time = Instant::now();

  let has_missing_license = if let Some(files) = collected_files {
    processor.process_collected(files).await?
  } else {
    processor.process(&args.patterns).await?
  };

  // Calculate elapsed time
  let elapsed = start_time.elapsed();

  // Get file reports from processor for report generation (take ownership to
  // avoid clone)
  let file_reports = std::mem::take(&mut *processor.file_reports.lock().await);

  // Create report summary
  let summary = ProcessingSummary::from_reports(&file_reports, elapsed);

  // Categorize reports for output
  let categorized = CategorizedReports::from_reports(&file_reports);

  // Print the output based on mode
  // Note: has_missing_license is the authoritative flag - it's set when files
  // fail to process (unreadable, etc.) even if they don't appear in file_reports
  print_blank_line();

  if check_only {
    // Check mode: show missing and outdated files
    let has_missing = !categorized.missing.is_empty();
    let has_outdated = !categorized.updated.is_empty();

    if !has_missing_license && !has_outdated {
      print_all_files_ok();
    } else {
      // Split the limit between lists if both have content
      let limit = if has_missing && has_outdated { Some(10) } else { None };

      if has_missing {
        print_missing_files(&categorized.missing, Some(&workspace_root), limit);
      }
      if has_outdated {
        if has_missing {
          print_blank_line();
        }
        print_outdated_files(&categorized.updated, Some(&workspace_root), limit);
      }
    }
    // If has_missing_license but categorized.missing is empty, files failed to
    // process - errors were already logged, so we skip the success message
  } else {
    // Modify mode: show what was changed
    if !categorized.added.is_empty() {
      print_added_files(&categorized.added, Some(&workspace_root));
    }
    if !categorized.updated.is_empty() {
      if !categorized.added.is_empty() {
        print_blank_line();
      }
      print_updated_files(&categorized.updated, Some(&workspace_root));
    }
    // Only show success if nothing was changed AND no failures occurred
    if categorized.added.is_empty() && categorized.updated.is_empty() && !has_missing_license {
      print_all_files_ok();
    }
  }

  // Print summary
  print_blank_line();
  print_summary(&summary, check_only);

  // Print hint if there are issues in check mode
  let has_outdated = !categorized.updated.is_empty();
  if check_only && (has_missing_license || has_outdated) {
    print_blank_line();
    let hint = match (has_missing_license, has_outdated) {
      (true, true) => "Run with --modify to add missing headers and update years.",
      (true, false) => "Run with --modify to add missing headers.",
      (false, true) => "Run with --modify to update outdated years.",
      (false, false) => unreachable!(),
    };
    print_hint(hint);
  }

  // Generate HTML report if requested
  if let Some(ref output_path) = args.report_html {
    let report_generator = ReportGenerator::new(ReportFormat::Html, output_path);
    if let Err(e) = report_generator.generate(&file_reports, &summary) {
      eprintln!("Error generating HTML report: {}", e);
    } else {
      info_log!("Generated HTML report at {}", output_path.display());
    }
  }

  // Generate JSON report if requested
  if let Some(ref output_path) = args.report_json {
    let report_generator = ReportGenerator::new(ReportFormat::Json, output_path);
    if let Err(e) = report_generator.generate(&file_reports, &summary) {
      eprintln!("Error generating JSON report: {}", e);
    } else {
      info_log!("Generated JSON report at {}", output_path.display());
    }
  }

  // Generate CSV report if requested
  if let Some(ref output_path) = args.report_csv {
    let report_generator = ReportGenerator::new(ReportFormat::Csv, output_path);
    if let Err(e) = report_generator.generate(&file_reports, &summary) {
      eprintln!("Error generating CSV report: {}", e);
    } else {
      info_log!("Generated CSV report at {}", output_path.display());
    }
  }

  // Exit with non-zero code if in check mode and there are issues
  if check_only && (has_missing_license || has_outdated) {
    process::exit(1);
  }

  Ok(())
}

/// Run in plan-tree mode: show a tree of files that would be checked.
async fn run_plan_tree(args: &CheckArgs) -> Result<()> {
  // Disable git ownership check if requested (useful in Docker)
  if args.skip_git_owner_check {
    debug!("Disabling git repository ownership check");
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
      debug!("Setting GLOBAL_LICENSE_IGNORE to {}", global_ignore_file.display());
    } else {
      eprintln!("Warning: Could not convert global ignore file path to string");
    }
  }

  let workspace = resolve_workspace(&args.patterns)?;
  let workspace_root = workspace.root().to_path_buf();

  let git_only = args.git_only.unwrap_or_else(|| workspace.is_git());
  if git_only && !workspace.is_git() {
    eprintln!("ERROR: Git-only mode is enabled, but not in a git repository");
    eprintln!("When --git-only is enabled, you must run edlicense from inside a git repository");
    process::exit(1);
  }

  // Load configuration file if present (needed for extension filtering)
  let config = load_config(args.config.as_deref(), &workspace_root, args.no_config)?;

  // Create the extension filter from config and CLI args
  let extension_filter = {
    let mut filter = config
      .as_ref()
      .map(|c| ExtensionFilter::new(&c.extensions))
      .unwrap_or_else(|| ExtensionFilter::from_cli(Vec::new(), Vec::new()));

    // Merge CLI args (CLI takes precedence over config)
    filter.merge_cli(args.include_ext.clone(), args.exclude_ext.clone());

    if filter.is_active() {
      debug!("Extension filtering is active");
      Some(filter)
    } else {
      None
    }
  };

  // Create a minimal processor for file collection
  // We need a dummy template manager since we won't actually process files
  let template_manager = TemplateManager::new();
  let license_data = LicenseData {
    year: String::new(), // Not used in plan-tree mode
  };

  let processor = Processor::new(
    template_manager,
    license_data,
    args.ignore.clone(),
    true,  // check_only
    false, // preserve_years
    args.ratchet.clone(),
    None, // diff_manager
    git_only,
    None, // license_detector
    workspace_root.clone(),
    workspace.is_git(),
    extension_filter,
  )?;

  // Collect files that would be processed
  let files = processor.collect_planned_files(&args.patterns).await?;

  // Print the tree
  let tree_output = print_tree(&files, Some(&workspace_root));
  println!("{}", tree_output);

  Ok(())
}

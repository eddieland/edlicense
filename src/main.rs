mod diff;
mod git;
mod ignore;
mod logging;
mod processor;
mod report;
mod templates;

use std::path::PathBuf;
use std::process;
use std::time::Instant;

use anyhow::{Context, Result};
use chrono::Datelike;
use clap::{Parser, ValueEnum};

use crate::diff::DiffManager;
use crate::logging::{ColorMode, set_color_mode, set_verbose};
use crate::processor::Processor;
use crate::report::{ProcessingSummary, ReportFormat, ReportGenerator};
use crate::templates::{LicenseData, TemplateManager};

/// Color mode options for output
#[derive(Debug, Clone, Copy, ValueEnum)]
enum ClapColorMode {
    /// Automatically determine whether to use colors based on TTY detection
    Auto,
    /// Never use colors
    Never,
    /// Always use colors
    Always,
}

impl From<ClapColorMode> for ColorMode {
    fn from(mode: ClapColorMode) -> Self {
        match mode {
            ClapColorMode::Auto => ColorMode::Auto,
            ClapColorMode::Never => ColorMode::Never,
            ClapColorMode::Always => ColorMode::Always,
        }
    }
}

/// A tool that ensures source code files have copyright license headers
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File or directory patterns to process. Directories are processed recursively.
    #[arg(required = true)]
    patterns: Vec<String>,

    /// Dry run mode: only check for license headers without modifying files (default)
    #[arg(long, group = "mode")]
    dry_run: bool,

    /// Modify mode: add or update license headers in files
    #[arg(long, group = "mode")]
    modify: bool,

    /// Show diff of changes in dry run mode
    #[arg(long)]
    show_diff: bool,

    /// Save diff of changes to a file in dry run mode
    #[arg(long)]
    save_diff: Option<PathBuf>,

    /// Custom license file to use
    #[arg(long, required = true)]
    license_file: PathBuf,

    /// File patterns to ignore (supports glob patterns)
    #[arg(long)]
    ignore: Vec<String>,

    /// Copyright year(s)
    #[arg(long)]
    year: Option<String>,

    /// Verbose mode: print names of modified files
    #[arg(long)]
    verbose: bool,

    /// Preserve existing years in license headers
    #[arg(long)]
    preserve_years: bool,

    /// Ratchet mode: only check and format files that have changed relative to a git reference
    #[arg(long)]
    ratchet: Option<String>,

    /// Path to a global license ignore file (overrides GLOBAL_LICENSE_IGNORE environment variable)
    #[arg(long)]
    global_ignore_file: Option<PathBuf>,

    /// Only consider files in the current git repository (defaults to false even in git repositories)
    #[arg(long)]
    git_only: Option<bool>,

    /// Control when to use colored output (auto, never, always)
    #[arg(long, value_enum, default_value = "auto")]
    colors: ClapColorMode,

    /// Generate an HTML report of license status and save to the specified path
    #[arg(long)]
    report_html: Option<PathBuf>,

    /// Generate a JSON report of license status and save to the specified path
    #[arg(long)]
    report_json: Option<PathBuf>,

    /// Generate a CSV report of license status and save to the specified path
    #[arg(long)]
    report_csv: Option<PathBuf>,
}

fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Set verbose mode
    set_verbose(args.verbose);

    // Set color mode
    set_color_mode(ColorMode::from(args.colors));

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

    // Determine the year to use
    let year = match args.year {
        Some(ref y) => y.clone(),
        None => chrono::Local::now().year().to_string(),
    };

    // Create license data
    let license_data = LicenseData { year };

    // Create and initialize template manager
    let mut template_manager = TemplateManager::new();
    template_manager
        .load_template(&args.license_file)
        .with_context(|| format!("Failed to load license template from {}", args.license_file.display()))?;

    // Determine mode (dry run is default if neither is specified or if dry_run is explicitly set)
    let check_only = args.dry_run || !args.modify;

    // Create diff manager
    let diff_manager = DiffManager::new(args.show_diff, args.save_diff);

    if args.git_only.unwrap_or(false) {
        // Note: This uses your current working directory ($CWD) to detect the git repository.
        // You should always run edlicense from inside the git repository when git detection is enabled.
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

    // Create processor
    let processor = Processor::new(
        template_manager,
        license_data,
        args.ignore,
        check_only,
        args.preserve_years,
        args.ratchet,
        Some(diff_manager),
        args.git_only,
    )?;

    // Start timing
    let start_time = Instant::now();

    // Process files
    let has_missing_license = processor.process(&args.patterns)?;

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
    let file_reports = if let Ok(reports) = processor.file_reports.lock() {
        reports.clone()
    } else {
        eprintln!("Warning: Failed to access file reports for report generation");
        Vec::new()
    };

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

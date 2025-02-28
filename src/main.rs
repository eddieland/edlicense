mod diff;
mod git;
mod ignore;
mod logging;
mod processor;
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

    /// Only consider files in the current git repository (default when in a git repository)
    #[arg(long)]
    git_only: Option<bool>,

    /// Control when to use colored output (auto, never, always)
    #[arg(long, value_enum, default_value = "auto")]
    colors: ClapColorMode,
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

    // Determine if we should only process git files
    // Default to true if we're in a git repository and git_only is not explicitly set to false
    // Note: This uses your current working directory ($CWD) to detect the git repository.
    // You should always run edlicense from inside the git repository when git detection is enabled.
    let is_git_repo = git::is_git_repository();
    let git_only = match args.git_only {
        Some(value) => value,
        None => is_git_repo, // Default to true if in a git repo
    };

    if git_only {
        if is_git_repo {
            info_log!("Git repository detected, only processing tracked files");
            verbose_log!("Using current working directory to determine git repository and tracked files");
        } else {
            info_log!("Git-only mode enabled, but not in a git repository");
            info_log!("Run edlicense from inside your git repository for correct git detection");
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
        Some(git_only),
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

    // Exit with non-zero code if in dry run mode and there are missing licenses
    if check_only && has_missing_license {
        eprintln!("Error: Some files are missing license headers");
        process::exit(1);
    }

    Ok(())
}

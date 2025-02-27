mod git;
mod ignore;
mod logging;
mod processor;
mod templates;

use std::path::PathBuf;
use std::process;

use anyhow::{Context, Result};
use chrono::Datelike;
use clap::Parser;

use crate::logging::set_verbose;
use crate::processor::Processor;
use crate::templates::{LicenseData, TemplateManager};

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
}

fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Set verbose mode
    set_verbose(args.verbose);

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

    // Create processor
    let processor = Processor::new(
        template_manager,
        license_data,
        args.ignore,
        check_only,
        args.preserve_years,
        args.ratchet,
    )?;

    // Process files
    let has_missing_license = processor.process(&args.patterns)?;

    // Exit with non-zero code if in dry run mode and there are missing licenses
    if check_only && has_missing_license {
        eprintln!("Error: Some files are missing license headers");
        process::exit(1);
    }

    Ok(())
}

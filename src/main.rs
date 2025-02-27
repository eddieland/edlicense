mod processor;
mod templates;
mod logging;

use std::path::PathBuf;
use std::process;

use anyhow::{Context, Result};
use chrono::Datelike;
use clap::Parser;

use crate::processor::Processor;
use crate::templates::{LicenseData, TemplateManager};
use crate::logging::set_verbose;

/// A tool that ensures source code files have copyright license headers
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File or directory patterns to process. Directories are processed recursively.
    #[arg(required = true)]
    patterns: Vec<String>,

    /// Check only mode: verify presence of license headers and exit with non-zero code if missing
    #[arg(long)]
    check: bool,

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
}

fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();
    
    // Set verbose mode
    set_verbose(args.verbose);

    // Determine the year to use
    let year = match args.year {
        Some(ref y) => y.clone(),
        None => chrono::Local::now().year().to_string(),
    };

    // Create license data
    let license_data = LicenseData {
        year,
    };

    // Create and initialize template manager
    let mut template_manager = TemplateManager::new();
    template_manager.load_template(&args.license_file)
        .with_context(|| format!("Failed to load license template from {}", args.license_file.display()))?;

    // Create processor
    let processor = Processor::new(
        template_manager,
        license_data,
        args.ignore,
        args.check,
        args.preserve_years,
    )?;

    // Process files
    let has_missing_license = processor.process(&args.patterns)?;

    // Exit with non-zero code if check mode is enabled and there are missing licenses
    if args.check && has_missing_license {
        eprintln!("Error: Some files are missing license headers");
        process::exit(1);
    }

    Ok(())
}

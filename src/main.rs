//! # edlicense
//!
//! A tool that ensures source code files have copyright license headers.

mod cli;
mod config;
mod diff;
mod file_filter;
mod git;
mod ignore;
mod license_detection;
mod logging;
mod output;
mod processor;
mod report;
mod templates;
mod tree;
mod workspace;

use anyhow::Result;

use crate::cli::{Cli, generate_completions, run_check};

fn main() -> Result<()> {
  let cli = Cli::parse_args();

  // Handle shell completions first
  if let Some(shell) = cli.completions {
    generate_completions(shell);
    return Ok(());
  }

  run_check(cli.check_args)
}

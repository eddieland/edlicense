//! # edlicense
//!
//! A tool that ensures source code files have copyright license headers.

mod cli;
mod diff;
mod file_filter;
mod git;
mod ignore;
mod license_detection;
mod logging;
mod processor;
mod report;
mod templates;
mod workspace;

use anyhow::Result;

use crate::cli::{Cli, Command, run_check, run_tree};

#[tokio::main]
async fn main() -> Result<()> {
  let cli = Cli::parse_args();

  match cli.command {
    Some(Command::Check(args)) => run_check(args).await,
    Some(Command::Tree(args)) => run_tree(args).await,
    // Default to check command for backward compatibility
    None => run_check(cli.check_args).await,
  }
}

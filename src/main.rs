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
mod tree;
mod workspace;

use anyhow::Result;

use crate::cli::{Cli, run_check};

#[tokio::main]
async fn main() -> Result<()> {
  let cli = Cli::parse_args();
  run_check(cli.check_args).await
}

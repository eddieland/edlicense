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

use crate::cli::{Cli, run_check};

fn main() -> Result<()> {
  // Handle shell completions via COMPLETE=<shell> environment variable
  clap_complete::CompleteEnv::with_factory(Cli::command).complete();

  let cli = Cli::parse_args();
  run_check(cli.check_args)
}

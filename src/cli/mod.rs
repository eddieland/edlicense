//! # CLI Module
//!
//! This module contains the command-line interface implementation.
//! It uses clap for argument parsing.

mod check;
mod tree;

pub use check::{CheckArgs, run_check};
pub use tree::{TreeArgs, run_tree};
use clap::{Parser, Subcommand};
use clap::builder::styling::{AnsiColor, Color, Style, Styles};

const CUSTOM_STYLES: Styles = Styles::styled()
  .header(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green))).bold())
  .usage(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green))).bold())
  .literal(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Blue))).bold())
  .placeholder(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Cyan))))
  .error(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Red))).bold())
  .valid(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green))))
  .invalid(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow))));

/// Top-level CLI arguments
#[derive(Parser, Debug)]
#[command(
  author,
  version,
  about,
  styles = CUSTOM_STYLES,
  after_help = "Examples:
  # Check license headers without modifying files
  edlicense check --license-file LICENSE.txt src/

  # Add or update license headers
  edlicense check --modify --license-file custom.txt --year 2023 include/ src/

  # Show diff of potential changes without modifying files
  edlicense check --show-diff --license-file LICENSE.txt src/**/*.rs

  # List files that would be checked
  edlicense tree src/

  # List files with verbose output showing why files are skipped
  edlicense tree -v src/

  # Only process files changed since a specific git commit
  edlicense check --ratchet=HEAD^ --license-file LICENSE.txt --modify .

  # Only process git-tracked files
  edlicense check --git-only --license-file LICENSE.txt --modify .

  # Generate an HTML report of license status
  edlicense check --report-html report.html --license-file LICENSE.txt src/

  # Ignore specific files or patterns
  edlicense check --ignore \"**/vendor/**\" --ignore \"**/*.json\" --license-file LICENSE.txt src/
",
  help_template = "{before-help}{name} v{version}
{about-section}
{usage-heading} {usage}

{all-args}{after-help}
"
)]
pub struct Cli {
  #[command(subcommand)]
  pub command: Option<Command>,

  /// For backward compatibility: flatten CheckArgs for when no subcommand is used
  #[command(flatten)]
  pub check_args: CheckArgs,
}

#[derive(Subcommand, Debug)]
pub enum Command {
  /// Check and optionally modify license headers in files (default command)
  Check(CheckArgs),

  /// List files that would be checked based on filtering rules
  Tree(TreeArgs),
}

impl Cli {
  /// Parse CLI arguments and return the Cli struct
  pub fn parse_args() -> Self {
    Self::parse()
  }
}

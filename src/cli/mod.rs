//! # CLI Module
//!
//! This module contains the command-line interface implementation.
//! It uses clap for argument parsing and supports subcommands for
//! extensibility.

mod check;

pub use check::{CheckArgs, run_check};
use clap::builder::styling::{AnsiColor, Color, Style, Styles};
use clap::{Parser, Subcommand};

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
  edlicense --license-file LICENSE.txt src/

  # Add or update license headers
  edlicense --modify --license-file custom.txt --year 2023 include/ src/

  # Show diff of potential changes without modifying files
  edlicense --show-diff --license-file LICENSE.txt src/**/*.rs

  # Save diff output to a file
  edlicense --save-diff changes.diff --license-file LICENSE.txt src/

  # Only process files changed since a specific git commit
  edlicense --ratchet=HEAD^ --license-file LICENSE.txt --modify .

  # Only process git-tracked files
  edlicense --git-only --license-file LICENSE.txt --modify .

  # Generate an HTML report of license status
  edlicense --report-html report.html --license-file LICENSE.txt src/

  # Ignore specific files or patterns
  edlicense --ignore \"**/vendor/**\" --ignore \"**/*.json\" --license-file LICENSE.txt src/
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

  #[command(flatten)]
  pub check_args: CheckArgs,
}

/// Available subcommands
#[derive(Subcommand, Debug)]
pub enum Command {
  /// Check and optionally modify license headers in source files (default)
  Check(CheckArgs),
}

impl Cli {
  /// Parse CLI arguments and return the Cli struct
  pub fn parse_args() -> Self {
    Self::parse()
  }

  /// Get the effective check arguments, whether from a subcommand or top-level
  pub fn get_check_args(self) -> CheckArgs {
    match self.command {
      Some(Command::Check(args)) => args,
      None => self.check_args,
    }
  }
}

//! # CLI Module
//!
//! This module contains the command-line interface implementation.
//! It uses clap for argument parsing.

mod check;

pub use check::{CheckArgs, run_check};
use clap::{CommandFactory, Parser};
use clap::builder::styling::{AnsiColor, Color, Style, Styles};
use clap_complete::{Generator, Shell, generate};

const CUSTOM_STYLES: Styles = Styles::styled()
  .header(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green))).bold())
  .usage(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green))).bold())
  .literal(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Blue))).bold())
  .placeholder(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Cyan))))
  .error(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Red))).bold())
  .valid(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green))))
  .invalid(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow))));

fn long_version() -> &'static str {
  static VERSION: std::sync::OnceLock<String> = std::sync::OnceLock::new();
  VERSION.get_or_init(|| {
    let version = env!("CARGO_PKG_VERSION");
    let hash = option_env!("GIT_HASH").unwrap_or_default();
    let date = option_env!("GIT_DATE").unwrap_or_default();
    if hash.is_empty() && date.is_empty() {
      version.to_string()
    } else {
      format!("{version} ({hash} {date})")
    }
  })
}

/// Top-level CLI arguments
#[derive(Parser, Debug)]
#[command(
  author,
  version = long_version(),
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
  /// Generate shell completions for the specified shell
  #[arg(long, value_name = "SHELL")]
  pub completions: Option<Shell>,

  #[command(flatten)]
  pub check_args: CheckArgs,
}

impl Cli {
  /// Parse CLI arguments and return the Cli struct
  pub fn parse_args() -> Self {
    Self::parse()
  }
}

/// Print shell completions for the given generator
fn print_completions<G: Generator>(generator: G) {
  let mut cmd = Cli::command();
  let name = cmd.get_name().to_string();
  generate(generator, &mut cmd, name, &mut std::io::stdout());
}

/// Generate and print shell completions, then exit
pub fn generate_completions(shell: Shell) {
  print_completions(shell);
}

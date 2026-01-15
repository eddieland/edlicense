use clap::ValueEnum;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;

/// Enum representing the color mode options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum ColorMode {
  /// Automatically determine whether to use colors based on TTY detection
  #[default]
  Auto = 0,
  /// Never use colors
  Never = 1,
  /// Always use colors
  Always = 2,
}

impl ColorMode {
  /// Apply the color mode using owo-colors' override mechanism.
  pub fn apply(self) {
    match self {
      ColorMode::Auto => owo_colors::unset_override(),
      ColorMode::Never => owo_colors::set_override(false),
      ColorMode::Always => owo_colors::set_override(true),
    }
  }
}

/// Initialize the tracing subscriber with the given verbosity settings.
///
/// The log level is determined by:
/// - If `quiet` is true: only ERROR level messages are shown
/// - If `verbose` is 0: WARN level (default)
/// - If `verbose` is 1: INFO level (-v)
/// - If `verbose` is 2: DEBUG level (-vv)
/// - If `verbose` is 3+: TRACE level (-vvv)
///
/// The `RUST_LOG` environment variable can override these defaults.
pub fn init_tracing(quiet: bool, verbose: u8) {
  let level = if quiet {
    LevelFilter::ERROR
  } else {
    match verbose {
      0 => LevelFilter::WARN,
      1 => LevelFilter::INFO,
      2 => LevelFilter::DEBUG,
      _ => LevelFilter::TRACE,
    }
  };

  let env_filter = EnvFilter::builder()
    .with_default_directive(level.into())
    .from_env_lossy();

  let _ = tracing_subscriber::fmt()
    .with_env_filter(env_filter)
    .with_target(false)
    .with_writer(std::io::stderr)
    .try_init();
}

use std::sync::atomic::{AtomicU8, Ordering};

use clap::ValueEnum;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;

/// Global atomic flag to control verbose logging.
///
/// This is initialized to `false` by default, meaning verbose logging is
/// disabled until explicitly enabled via [`set_verbose`].
static OUTPUT_MODE: AtomicU8 = AtomicU8::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputMode {
  Normal = 0,
  Quiet = 1,
  Verbose = 2,
}

impl OutputMode {
  /// Convert from u8 to OutputMode
  const fn from_u8(value: u8) -> Self {
    match value {
      0 => OutputMode::Normal,
      1 => OutputMode::Quiet,
      2 => OutputMode::Verbose,
      _ => OutputMode::Normal, // Default to Invalid values
    }
  }
}

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

/// Sets the global verbose logging flag.
///
/// When verbose logging is enabled, the output module shows additional details.
/// Note: For debug logging, use `tracing::debug!` instead.
pub fn set_verbose() {
  OUTPUT_MODE.store(OutputMode::Verbose as u8, Ordering::SeqCst);
}

pub fn set_quiet() {
  OUTPUT_MODE.store(OutputMode::Quiet as u8, Ordering::SeqCst);
}

/// Checks if verbose logging is currently enabled.
///
/// This function is used by the output module to determine whether to show
/// additional details in user-facing output.
///
/// # Returns
///
/// `true` if verbose logging is enabled, `false` otherwise.
pub fn is_verbose() -> bool {
  let mode_u8 = OUTPUT_MODE.load(Ordering::SeqCst);
  matches!(OutputMode::from_u8(mode_u8), OutputMode::Verbose)
}

/// Checks if quiet mode is currently enabled.
/// This function can be used to determine if output should be suppressed.
/// # Returns
///
/// `true` if quiet mode is enabled, `false` otherwise.
pub fn is_quiet() -> bool {
  let mode_u8 = OUTPUT_MODE.load(Ordering::SeqCst);
  matches!(OutputMode::from_u8(mode_u8), OutputMode::Quiet)
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

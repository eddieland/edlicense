use std::sync::atomic::{AtomicU8, Ordering};

use clap::ValueEnum;

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
/// When verbose logging is enabled, the [`verbose_log!`] macro will output
/// messages to stderr. When disabled, verbose log messages are suppressed.
pub fn set_verbose() {
  OUTPUT_MODE.store(OutputMode::Verbose as u8, Ordering::SeqCst);
}

pub fn set_quiet() {
  OUTPUT_MODE.store(OutputMode::Quiet as u8, Ordering::SeqCst);
}

/// Checks if verbose logging is currently enabled.
///
/// This function is used internally by the [`verbose_log!`] macro to determine
/// whether to output verbose log messages.
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

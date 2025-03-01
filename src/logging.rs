//! # Logging Module
//!
//! This module provides logging utilities for the edlicense tool, including:
//! - Verbose logging that can be enabled/disabled
//! - Standard info logging with color support
//!
//! The logging system is designed to be simple and efficient, with verbose logs
//! going to stderr and info logs going to stdout for better pipeline integration.
//!
//! ## Example
//!
//! ```rust
//! use edlicense::logging::{set_verbose, set_color_mode};
//! use edlicense::{verbose_log, info_log};
//! use edlicense::logging::ColorMode;
//!
//! // Enable verbose logging
//! set_verbose(true);
//!
//! // Set color mode to Auto
//! set_color_mode(ColorMode::Auto);
//!
//! // Log a verbose message (goes to stderr)
//! verbose_log!("Processing file: {}", "example.rs");
//!
//! // Log an info message (goes to stdout)
//! info_log!("License added to: {}", "example.rs");
//! ```

use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

/// Global atomic flag to control verbose logging.
///
/// This is initialized to `false` by default, meaning verbose logging is disabled
/// until explicitly enabled via [`set_verbose`].
static VERBOSE: AtomicBool = AtomicBool::new(false);

/// Global atomic value to control color mode.
///
/// This is initialized to `0` (Auto) by default.
static COLOR_MODE: AtomicU8 = AtomicU8::new(0);

/// Enum representing the color mode options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
  /// Automatically determine whether to use colors based on TTY detection
  Auto = 0,
  /// Never use colors
  Never = 1,
  /// Always use colors
  Always = 2,
}

impl ColorMode {
  /// Convert from a string to ColorMode (for potential future use)
  #[allow(dead_code)]
  fn from_str(s: &str) -> Result<Self, String> {
    match s.to_lowercase().as_str() {
      "auto" => Ok(ColorMode::Auto),
      "never" => Ok(ColorMode::Never),
      "always" => Ok(ColorMode::Always),
      _ => Err(format!("Invalid color mode: {}", s)),
    }
  }

  /// Convert from u8 to ColorMode
  fn from_u8(value: u8) -> Self {
    match value {
      0 => ColorMode::Auto,
      1 => ColorMode::Never,
      2 => ColorMode::Always,
      _ => ColorMode::Auto, // Default to Auto for invalid values
    }
  }

  /// Convert to termcolor::ColorChoice
  fn to_color_choice(self) -> ColorChoice {
    match self {
      ColorMode::Auto => {
        if atty::is(atty::Stream::Stdout) {
          ColorChoice::Auto
        } else {
          ColorChoice::Never
        }
      }
      ColorMode::Never => ColorChoice::Never,
      ColorMode::Always => ColorChoice::Always,
    }
  }
}

/// Sets the global verbose logging flag.
///
/// When verbose logging is enabled, the [`verbose_log!`] macro will output messages
/// to stderr. When disabled, verbose log messages are suppressed.
///
/// # Parameters
///
/// * `verbose` - `true` to enable verbose logging, `false` to disable it
pub fn set_verbose(verbose: bool) {
  VERBOSE.store(verbose, Ordering::SeqCst);
}

/// Sets the global color mode.
///
/// This controls whether colors are used in the output.
///
/// # Parameters
///
/// * `mode` - The color mode to use
pub fn set_color_mode(mode: ColorMode) {
  COLOR_MODE.store(mode as u8, Ordering::SeqCst);
}

/// Gets the current color mode.
///
/// # Returns
///
/// The current color mode.
pub fn get_color_mode() -> ColorMode {
  ColorMode::from_u8(COLOR_MODE.load(Ordering::SeqCst))
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
  VERBOSE.load(Ordering::SeqCst)
}

/// Logs a message to stderr if verbose mode is enabled.
///
/// This macro is used for detailed logging that is only shown when verbose mode
/// is enabled via [`set_verbose`]. It uses the same format string syntax as
/// the standard [`eprintln!`] macro.
#[macro_export]
macro_rules! verbose_log {
    ($($arg:tt)*) => {
        if $crate::logging::is_verbose() {
            eprintln!($($arg)*);
        }
    };
}

/// Logs a message to stdout regardless of verbose mode.
///
/// This macro is used for important information that should always be displayed
/// to the user. It uses the same format string syntax as the standard [`println!`] macro.
#[macro_export]
macro_rules! info_log {
    ($($arg:tt)*) => {
        $crate::logging::print_info_log(&format!($($arg)*));
    };
}

/// Internal function to print info log messages with formatting.
///
/// This function is used by the [`info_log!`] macro to format and print
/// messages with colors if enabled.
///
/// # Parameters
///
/// * `message` - The message to print
pub fn print_info_log(message: &str) {
  let color_mode = get_color_mode();
  let color_choice = color_mode.to_color_choice();

  let mut stdout = StandardStream::stdout(color_choice);

  if let Err(e) = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow))) {
    eprintln!("Error setting color: {}", e);
  }

  if let Err(e) = writeln!(&mut stdout, "{}", message) {
    eprintln!("Error writing to stdout: {}", e);
  }

  // Reset colors
  if let Err(e) = stdout.reset() {
    eprintln!("Error resetting colors: {}", e);
  }
}

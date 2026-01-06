//! # Logging Module
//!
//! This module provides logging utilities for the edlicense tool, including:
//! - Verbose logging that can be enabled/disabled
//! - Standard info logging with color support
//!
//! The logging system is designed to be simple and efficient, with verbose logs
//! going to stderr and info logs going to stdout for better pipeline
//! integration.
//!
//! ## Example
//!
//! ```rust
//! use edlicense::logging::{ColorMode, set_color_mode, set_verbose};
//! use edlicense::{info_log, verbose_log};
//!
//! // Enable verbose logging
//! set_verbose();
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

mod modes;

use std::io::Write;

pub use modes::{ColorMode, get_color_mode, is_quiet, is_verbose, set_color_mode, set_quiet, set_verbose};
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

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
/// to the user. It uses the same format string syntax as the standard
/// [`println!`] macro.
#[macro_export]
macro_rules! info_log {
    ($($arg:tt)*) => {
        if !$crate::logging::is_quiet() {
            $crate::logging::print_info_log(&format!($($arg)*));
        }
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

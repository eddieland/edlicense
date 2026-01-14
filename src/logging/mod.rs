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
//! use edlicense::logging::{ColorMode, set_verbose};
//! use edlicense::{info_log, verbose_log};
//!
//! // Enable verbose logging
//! set_verbose();
//!
//! // Set color mode to Auto (uses owo-colors' automatic TTY detection)
//! ColorMode::Auto.apply();
//!
//! // Log a verbose message (goes to stderr)
//! verbose_log!("Processing file: {}", "example.rs");
//!
//! // Log an info message (goes to stdout)
//! info_log!("License added to: {}", "example.rs");
//! ```

mod modes;

pub use modes::{ColorMode, init_tracing, is_quiet, is_verbose, set_quiet, set_verbose};
use owo_colors::{OwoColorize, Stream};

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
  println!("{}", message.if_supports_color(Stream::Stdout, |m| m.yellow()));
}

//! # Logging Module
//!
//! This module provides logging utilities for the edlicense tool, including:
//! - Tracing-based logging at multiple levels (info, debug, trace)
//! - User-facing output via `info_log!` macro (goes to stdout)
//!
//! ## Log Levels
//!
//! The tracing library provides multiple log levels controlled by verbosity
//! flags:
//! - `info!` - High-level progress (`-v`): config loading, directory scanning
//! - `debug!` - Detailed processing (`-vv`): file counts, timing, filtering
//! - `trace!` - Per-file details (`-vvv`): individual file processing, skips
//!
//! All tracing output goes to stderr. User-facing output uses `info_log!` which
//! goes to stdout.
//!
//! ## Example
//!
//! ```rust
//! use edlicense::info_log;
//! use edlicense::logging::{ColorMode, set_verbose};
//! use tracing::{debug, trace};
//!
//! // Enable verbose logging
//! set_verbose();
//!
//! // Set color mode to Auto (uses owo-colors' automatic TTY detection)
//! ColorMode::Auto.apply();
//!
//! // Log debug info (shown with -vv)
//! debug!("Found {} files", 42);
//!
//! // Log trace info (shown with -vvv)
//! trace!("Processing file: {}", "example.rs");
//!
//! // Log user-facing output (goes to stdout)
//! info_log!("License added to: {}", "example.rs");
//! ```

mod modes;

pub use modes::{ColorMode, init_tracing, is_quiet, is_verbose, set_quiet, set_verbose};
use owo_colors::{OwoColorize, Stream};

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

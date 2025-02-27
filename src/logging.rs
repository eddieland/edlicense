//! # Logging Module
//!
//! This module provides logging utilities for the edlicense tool, including:
//! - Verbose logging that can be enabled/disabled
//! - Standard info logging
//!
//! The logging system is designed to be simple and efficient, with verbose logs
//! going to stderr and info logs going to stdout for better pipeline integration.
//!
//! ## Example
//!
//! ```rust
//! use edlicense::logging::set_verbose;
//! use edlicense::{verbose_log, info_log};
//!
//! // Enable verbose logging
//! set_verbose(true);
//!
//! // Log a verbose message (goes to stderr)
//! verbose_log!("Processing file: {}", "example.rs");
//!
//! // Log an info message (goes to stdout)
//! info_log!("License added to: {}", "example.rs");
//! ```

use std::sync::atomic::{AtomicBool, Ordering};

/// Global atomic flag to control verbose logging.
///
/// This is initialized to `false` by default, meaning verbose logging is disabled
/// until explicitly enabled via [`set_verbose`].
static VERBOSE: AtomicBool = AtomicBool::new(false);

/// Sets the global verbose logging flag.
///
/// When verbose logging is enabled, the [`verbose_log!`] macro will output messages
/// to stderr. When disabled, verbose log messages are suppressed.
///
/// # Parameters
///
/// * `verbose` - `true` to enable verbose logging, `false` to disable it
///
/// # Examples
///
/// ```rust
/// use edlicense::logging::set_verbose;
///
/// // Enable verbose logging
/// set_verbose(true);
///
/// // Disable verbose logging
/// set_verbose(false);
/// ```
pub fn set_verbose(verbose: bool) {
    VERBOSE.store(verbose, Ordering::SeqCst);
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
///
/// # Examples
///
/// ```rust
/// use edlicense::logging::set_verbose;
/// use edlicense::verbose_log;
///
/// set_verbose(true);
/// verbose_log!("Processing file: {}", "example.rs");
/// ```
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
///
/// # Examples
///
/// ```rust
/// use edlicense::info_log;
///
/// info_log!("License added to: {}", "example.rs");
/// ```
#[macro_export]
macro_rules! info_log {
    ($($arg:tt)*) => {
        println!($($arg)*);
    };
}

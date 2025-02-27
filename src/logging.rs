//! Logging utilities for edlicense

use std::sync::atomic::{AtomicBool, Ordering};

/// Global verbose flag
static VERBOSE: AtomicBool = AtomicBool::new(false);

/// Set the global verbose flag
pub fn set_verbose(verbose: bool) {
    VERBOSE.store(verbose, Ordering::SeqCst);
}

/// Check if verbose logging is enabled
pub fn is_verbose() -> bool {
    VERBOSE.load(Ordering::SeqCst)
}

/// Log a message to stderr if verbose mode is enabled
#[macro_export]
macro_rules! verbose_log {
    ($($arg:tt)*) => {
        if $crate::logging::is_verbose() {
            eprintln!($($arg)*);
        }
    };
}

/// Log a message to stdout regardless of verbose mode
#[macro_export]
macro_rules! info_log {
    ($($arg:tt)*) => {
        println!($($arg)*);
    };
}

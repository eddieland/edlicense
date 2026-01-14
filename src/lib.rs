//! # edlicense
//!
//! A tool that ensures source code files have copyright license headers by
//! scanning directory patterns recursively.
//!
//! `edlicense` modifies source files in place and avoids adding a license
//! header to any file that already has one. It follows the Unix philosophy of
//! tooling where possible and is designed with modern Rust best practices for
//! high-performance CLI tools.
//!
//! ## Features
//!
//! * Recursively scan directories and add license headers to source files
//! * Automatic detection of file types and appropriate comment formatting
//! * Check-only mode to verify license headers without modifying files
//! * Ignore patterns to exclude specific files or directories
//! * Automatic year reference updates - automatically updates copyright year
//!   references when the year changes
//! * Ratchet mode - only check and format files that have changed relative to a
//!   git reference

// Re-export modules for public API
pub mod cli;
pub mod config;
pub mod diff;
pub mod file_filter;
pub mod git;
pub mod ignore;
pub mod license_detection;
pub mod logging;
pub mod processor;
pub mod report;
pub mod templates;
pub mod tree;
pub mod workspace;

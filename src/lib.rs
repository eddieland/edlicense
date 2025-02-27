//! # edlicense
//!
//! A tool that ensures source code files have copyright license headers by scanning directory patterns recursively.
//!
//! `edlicense` modifies source files in place and avoids adding a license header to any file that already has one.
//! It follows the Unix philosophy of tooling where possible and is designed with modern Rust best practices for
//! high-performance CLI tools.
//!
//! ## Features
//!
//! * Recursively scan directories and add license headers to source files
//! * Automatic detection of file types and appropriate comment formatting
//! * Check-only mode to verify license headers without modifying files
//! * Ignore patterns to exclude specific files or directories
//! * Automatic year reference updates - automatically updates copyright year references when the year changes
//! * Ratchet mode - only check and format files that have changed relative to a git reference
//!
//! ## Usage as a Library
//!
//! This crate can be used as a library in your Rust projects:
//!
//! ```rust,no_run
//! use edlicense::processor::Processor;
//! use edlicense::templates::{LicenseData, TemplateManager};
//! use std::path::Path;
//!
//! fn main() -> anyhow::Result<()> {
//!     // Create license data with the current year
//!     let license_data = LicenseData {
//!         year: "2025".to_string(),
//!     };
//!
//!     // Create and initialize template manager
//!     let mut template_manager = TemplateManager::new();
//!     template_manager.load_template(Path::new("LICENSE.txt"))?;
//!
//!     // Create processor with default settings
//!     let processor = Processor::new(
//!         template_manager,
//!         license_data,
//!         vec![], // No ignore patterns
//!         false,  // Not check-only mode
//!         false,  // Don't preserve years
//!         None,   // No ratchet reference
//!         None    // Use the default DiffManager
//!     )?;
//!
//!     // Process files in the src directory
//!     let has_missing_license = processor.process(&["src".to_string()])?;
//!
//!     if has_missing_license {
//!         println!("Some files were missing license headers");
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Modules
//!
//! * [`processor`] - Core functionality for processing files and directories
//! * [`templates`] - License template management and formatting
//! * [`logging`] - Logging utilities for verbose output
//!
//! [`processor`]: crate::processor
//! [`templates`]: crate::templates
//! [`logging`]: crate::logging

// Re-export modules for public API
pub mod diff;
pub mod git;
pub mod ignore;
pub mod logging;
pub mod processor;
pub mod templates;

// Re-export macros
// Note: We don't re-export the macros here since they're already defined in the logging module
// and would cause redefinition errors

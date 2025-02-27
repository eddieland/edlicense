//! edlicense - A tool that ensures source code files have copyright license headers
//!
//! This library provides functionality for adding, checking, and updating license headers
//! in source code files. It can be used as a library or through the command-line interface.

// Re-export modules for public API
pub mod logging;
pub mod processor;
pub mod templates;

// Re-export macros
// Note: We don't re-export the macros here since they're already defined in the logging module
// and would cause redefinition errors

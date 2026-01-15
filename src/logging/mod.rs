//! # Logging Module
//!
//! This module provides logging utilities for the edlicense tool.
//!
//! Logging is powered by the tracing crate. Use `tracing::debug!`,
//! `tracing::info!`, etc. for logging. The log level is controlled by:
//! - CLI flags: `-v` (info), `-vv` (debug), `-vvv` (trace)
//! - `RUST_LOG` environment variable
//!
//! All logs go to stderr.

mod modes;

pub use modes::{ColorMode, init_tracing};

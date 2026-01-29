//! # File I/O Module
//!
//! This module provides file reading and writing utilities for the processor.
//! It encapsulates synchronous file operations.

use std::io::Read as _;
use std::path::Path;

use anyhow::{Context, Result};

/// Maximum number of bytes to read when checking for a license header.
/// 8KB is sufficient for most license headers.
pub const LICENSE_READ_LIMIT: usize = 8 * 1024;

/// File I/O operations for the processor.
///
/// This struct provides static methods for reading and writing files.
pub struct FileIO;

impl FileIO {
  /// Reads the initial portion of a file for license checking.
  ///
  /// This method reads up to LICENSE_READ_LIMIT bytes from the start of the
  /// file. It attempts to interpret the bytes as UTF-8, handling invalid
  /// sequences by truncating at the last valid character.
  ///
  /// Returns (prefix_bytes, prefix_content, file_length) to avoid needing to keep
  /// the file handle open.
  ///
  /// # Parameters
  ///
  /// * `path` - Path to the file to read
  ///
  /// # Returns
  ///
  /// A tuple containing:
  /// - The raw bytes read
  /// - The UTF-8 string content
  /// - The total file length
  pub fn read_license_check_prefix(path: &Path) -> Result<(Vec<u8>, String, u64)> {
    let mut file = std::fs::File::open(path).with_context(|| format!("Failed to open file: {}", path.display()))?;

    let file_len = file.metadata().map(|m| m.len()).unwrap_or(0);

    let mut buf = vec![0u8; LICENSE_READ_LIMIT];
    let read_len = file
      .read(&mut buf)
      .with_context(|| format!("Failed to read file: {}", path.display()))?;
    buf.truncate(read_len);

    let prefix_content = match std::str::from_utf8(&buf) {
      Ok(prefix) => prefix.to_string(),
      Err(e) => {
        let valid_up_to = e.valid_up_to();
        if valid_up_to == 0 {
          return Err(anyhow::anyhow!("Failed to read file {}: {}", path.display(), e));
        }
        String::from_utf8_lossy(&buf[..valid_up_to]).to_string()
      }
    };

    Ok((buf, prefix_content, file_len))
  }

  /// Read full file content.
  ///
  /// # Parameters
  ///
  /// * `path` - Path to the file to read
  ///
  /// # Returns
  ///
  /// The complete file content as a String.
  pub fn read_full_content(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("Failed to read file: {}", path.display()))
  }

  /// Write file content.
  ///
  /// # Parameters
  ///
  /// * `path` - Path to the file to write
  /// * `content` - Content to write to the file
  pub fn write_file(path: &Path, content: &str) -> Result<()> {
    std::fs::write(path, content).with_context(|| format!("Failed to write file: {}", path.display()))
  }
}

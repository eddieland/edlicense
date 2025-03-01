//! # License Detection Module
//!
//! This module contains the interfaces and implementations for license detection algorithms.
//! It allows for easily replacing the license detection algorithm without modifying the processor.

/// Trait for license detectors.
///
/// Implementations of this trait are responsible for determining whether a file
/// already contains a license header based on its content.
pub trait LicenseDetector: Send + Sync {
  /// Checks if the content already has a license header.
  ///
  /// # Parameters
  ///
  /// * `content` - The file content to check
  ///
  /// # Returns
  ///
  /// `true` if the content appears to have a license header, `false` otherwise.
  fn has_license(&self, content: &str) -> bool;
}

/// Default implementation of license detection.
///
/// This detector checks for the presence of the word "copyright" in the first 1000
/// characters of the file content, case-insensitive.
///
/// This is based on addlicense's implementation of this functionality.
pub struct SimpleLicenseDetector;

impl SimpleLicenseDetector {
  /// Creates a new DefaultLicenseDetector.
  pub fn new() -> Self {
    SimpleLicenseDetector
  }
}

impl Default for SimpleLicenseDetector {
  fn default() -> Self {
    Self::new()
  }
}

impl LicenseDetector for SimpleLicenseDetector {
  /// Checks if the content already has a license header.
  ///
  /// The check is case-insensitive and only examines the first 1000 characters
  /// of the file for performance reasons, as license headers are typically
  /// at the beginning of files.
  fn has_license(&self, content: &str) -> bool {
    // Take the first 1000 characters (or less if the file is shorter)
    let check_len = std::cmp::min(content.len(), 1000);
    let check_content = &content[..check_len];

    // Convert to lowercase for case-insensitive matching
    let check_content_lower = check_content.to_lowercase();

    // Based on addlicense's implementation, we check for common license indicators
    // without requiring the specific year (which is too strict)
    check_content_lower.contains("copyright")
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_simple_license_detector() {
    let detector = SimpleLicenseDetector::new();

    // Test content with a license
    let content_with_license = "// Copyright (c) 2024 Test Company\n\nfn main() {}";
    assert!(detector.has_license(content_with_license));

    // Test content with a license in different format
    let content_with_license2 = "/* Copyright (C) 2024 Test Company */\n\nfn main() {}";
    assert!(detector.has_license(content_with_license2));

    // Test content without a license
    let content_without_license = "fn main() {\n    println!(\"No license in this code\");\n}";
    assert!(!detector.has_license(content_without_license));
  }
}

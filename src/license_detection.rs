//! # License Detection Module
//!
//! This module contains the interfaces and implementations for license detection algorithms.
//! It allows for easily replacing the license detection algorithm without modifying the processor.

use regex::Regex;
use std::sync::LazyLock;

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

/// Content-based license detector implementation.
///
/// This detector compares the actual content of the expected license text
/// with the first N characters of the file, ignoring whitespace and comment characters.
/// This is more precise than just checking for the presence of a keyword.
pub struct ContentBasedLicenseDetector {
  /// The expected license text to look for
  license_text: String,

  /// The number of characters to check at the beginning of files
  check_length: usize,
}

impl ContentBasedLicenseDetector {
  /// Creates a new ContentBasedLicenseDetector.
  ///
  /// # Parameters
  ///
  /// * `license_text` - The expected license text to look for in files
  /// * `check_length` - The number of characters to check at the beginning of files (defaults to 2000)
  ///
  /// # Returns
  ///
  /// A new `ContentBasedLicenseDetector` instance.
  #[allow(dead_code)]
  pub fn new(license_text: &str, check_length: Option<usize>) -> Self {
    ContentBasedLicenseDetector {
      license_text: license_text.to_string(),
      check_length: check_length.unwrap_or(2000),
    }
  }

  /// Normalizes text for comparison by:
  /// - Converting to lowercase
  /// - Removing common comment characters
  /// - Removing all whitespace
  ///
  /// This allows for more flexible matching regardless of formatting differences.
  ///
  /// # Parameters
  ///
  /// * `text` - The text to normalize
  ///
  /// # Returns
  ///
  /// The normalized text with comment characters and whitespace removed.
  fn normalize_text(&self, text: &str) -> String {
    let lowercase = text.to_lowercase();

    // Remove comment characters that might be present
    let without_comments = lowercase
      .replace("//", "")
      .replace("/*", "")
      .replace("*/", "")
      .replace("*", "")
      .replace("#", "")
      .replace("<!--", "")
      .replace("-->", "")
      .replace(";", ""); // For languages like Lisp

    // Replace newlines with spaces and normalize whitespace
    let with_normalized_whitespace = without_comments.replace(['\n', '\r', '\t'], " ");

    // Collapse multiple spaces into a single space
    let mut result = String::with_capacity(with_normalized_whitespace.len());
    let mut last_was_space = false;

    for c in with_normalized_whitespace.chars() {
      if c.is_whitespace() {
        if !last_was_space {
          result.push(' ');
          last_was_space = true;
        }
      } else {
        result.push(c);
        last_was_space = false;
      }
    }

    result.trim().to_string()
  }
}

impl LicenseDetector for ContentBasedLicenseDetector {
  /// Checks if the content already has a license header by comparing the
  /// normalized license text with the normalized beginning of the file.
  ///
  /// The comparison ignores:
  /// - Case differences
  /// - Whitespace differences
  /// - Comment characters
  ///
  /// # Parameters
  ///
  /// * `content` - The file content to check
  ///
  /// # Returns
  ///
  /// `true` if the file content appears to contain the license text, `false` otherwise.
  fn has_license(&self, content: &str) -> bool {
    // Take the first N characters (or less if the file is shorter)
    let check_len = std::cmp::min(content.len(), self.check_length);
    let check_content = &content[..check_len];

    // Normalize both the expected license text and the content for comparison
    let normalized_license = self.normalize_text(&self.license_text);
    let normalized_content = self.normalize_text(check_content);

    // Replace all years (4 digits surrounded by word boundaries) with "YEAR" placeholder
    // This makes the comparison year-agnostic
    static YEAR_PATTERN: LazyLock<Regex> =
      LazyLock::new(|| Regex::new(r"\b\d{4}\b").expect("year regex must compile"));
    let year_normalized_license =
      YEAR_PATTERN.replace_all(&normalized_license, "YEAR").to_string();
    let year_normalized_content =
      YEAR_PATTERN.replace_all(&normalized_content, "YEAR").to_string();

    // Check if the year-normalized content contains the year-normalized license text
    year_normalized_content.contains(&year_normalized_license)
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

  #[test]
  fn test_content_based_license_detector() {
    // Define expected license text
    let license_text = "Copyright (c) 2025 Test Company\n\nThis is a test license.";

    // Create detector with default check length
    let detector = ContentBasedLicenseDetector::new(license_text, None);

    // Test content with exact license
    let content_with_exact_license =
      "// Copyright (c) 2025 Test Company\n//\n// This is a test license.\n\nfn main() {}";
    assert!(detector.has_license(content_with_exact_license));

    // Test content with differently formatted license (different comment style)
    let content_with_different_format =
      "/* Copyright (c) 2025 Test Company\n * \n * This is a test license.\n */\n\nfn main() {}";
    assert!(detector.has_license(content_with_different_format));

    // Test content with different spacing
    let content_with_different_spacing =
      "// Copyright  (c)  2025  Test   Company\n//\n//   This  is  a  test  license.\n\nfn main() {}";
    assert!(detector.has_license(content_with_different_spacing));

    // Test content with license but in a different year (should still match)
    let content_with_different_year =
      "// Copyright (c) 2024 Test Company\n//\n// This is a test license.\n\nfn main() {}";
    assert!(detector.has_license(content_with_different_year));

    // Test content without the license
    let content_without_license = "fn main() {\n    println!(\"This code has no license header\");\n}";
    assert!(!detector.has_license(content_without_license));

    // Test with incomplete license text
    let content_with_partial_license =
      "// Copyright (c) 2025 Test Company\n//\n// This is not the complete license.\n\nfn main() {}";
    assert!(!detector.has_license(content_with_partial_license));
  }

  #[test]
  fn test_normalize_text() {
    let license_text = "Copyright (c) 2025 Test Company";
    let detector = ContentBasedLicenseDetector::new(license_text, None);

    // Test normalization of various comment styles
    let commented_text = "// Copyright (c) 2025 Test Company";
    let block_commented_text = "/* Copyright (c) 2025 Test Company */";
    let python_commented_text = "# Copyright (c) 2025 Test Company";
    let xml_commented_text = "<!-- Copyright (c) 2025 Test Company -->";
    let lisp_commented_text = ";; Copyright (c) 2025 Test Company";

    // The normalized text should be the same for all of these, but now with spaces preserved
    let expected = "copyright (c) 2025 test company";

    assert_eq!(detector.normalize_text(commented_text), expected);
    assert_eq!(detector.normalize_text(block_commented_text), expected);
    assert_eq!(detector.normalize_text(python_commented_text), expected);
    assert_eq!(detector.normalize_text(xml_commented_text), expected);
    assert_eq!(detector.normalize_text(lisp_commented_text), expected);

    // Test with extra whitespace (should be collapsed to single spaces)
    let text_with_whitespace = "  Copyright  (c)  2025  Test  Company  ";
    assert_eq!(detector.normalize_text(text_with_whitespace), expected);

    // Test with newlines (should convert to spaces)
    let text_with_newlines = "Copyright\n(c)\n2025\nTest\nCompany";
    assert_eq!(detector.normalize_text(text_with_newlines), expected);

    // Test with mixed whitespace
    let text_with_mixed_whitespace = "Copyright\t(c)\r\n2025  Test    Company";
    assert_eq!(detector.normalize_text(text_with_mixed_whitespace), expected);
  }
}

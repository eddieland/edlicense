//! # License Detection Module
//!
//! This module contains the interfaces and implementations for license
//! detection algorithms. It allows for easily replacing the license detection
//! algorithm without modifying the processor.

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
/// This detector checks for the presence of the word "copyright" in the first
/// 1000 characters of the file content, case-insensitive.
///
/// This is based on addlicense's implementation of this functionality.
pub struct SimpleLicenseDetector;

impl SimpleLicenseDetector {
  /// Number of bytes to check at the beginning of files for license headers.
  const CHECK_LENGTH: usize = 1000;

  /// Creates a new DefaultLicenseDetector.
  pub const fn new() -> Self {
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
  /// The check is case-insensitive and only examines the first 1000 bytes
  /// of the file for performance reasons, as license headers are typically
  /// at the beginning of files.
  ///
  /// This implementation uses zero-allocation byte-level comparison for
  /// optimal performance.
  fn has_license(&self, content: &str) -> bool {
    // Take the first CHECK_LENGTH bytes (or less if the file is shorter)
    let check_len = std::cmp::min(content.len(), Self::CHECK_LENGTH);
    let bytes = content.as_bytes();
    let check_bytes = &bytes[..check_len];

    // Case-insensitive search for "copyright" without allocation.
    // Uses byte-level comparison with eq_ignore_ascii_case which is O(n)
    // and avoids the String allocation from to_lowercase().
    check_bytes.windows(9).any(|w| w.eq_ignore_ascii_case(b"copyright"))
  }
}

/// Content-based license detector implementation.
///
/// This detector compares the actual content of the expected license text
/// with the first N characters of the file, ignoring whitespace and comment
/// characters. This is more precise than just checking for the presence of a
/// keyword.
pub struct ContentBasedLicenseDetector {
  /// Pre-computed normalized and year-replaced license text
  normalized_license: String,

  /// The number of bytes to check at the beginning of files
  check_length: usize,
}

impl ContentBasedLicenseDetector {
  /// Default number of bytes to check at the beginning of files.
  const DEFAULT_CHECK_LENGTH: usize = 2000;

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
  pub fn new(license_text: &str, check_length: Option<usize>) -> Self {
    let normalized = Self::normalize_and_replace_years(license_text);
    ContentBasedLicenseDetector {
      normalized_license: normalized,
      check_length: check_length.unwrap_or(Self::DEFAULT_CHECK_LENGTH),
    }
  }

  /// Single-pass normalization that:
  /// - Converts to lowercase
  /// - Skips comment characters (/, *, #, <, !, -, >, ;)
  /// - Collapses all whitespace to single spaces
  /// - Replaces 4-digit year sequences with "YEAR"
  #[inline]
  fn normalize_and_replace_years(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut last_was_space = true; // Start true to trim leading space
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
      let b = bytes[i];

      // Check for 4-digit year pattern
      if b.is_ascii_digit()
        && i + 3 < len
        && bytes[i + 1].is_ascii_digit()
        && bytes[i + 2].is_ascii_digit()
        && bytes[i + 3].is_ascii_digit()
      {
        // Check word boundaries (start of string or non-alphanumeric before)
        let at_word_start = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
        // Check word boundary after (end of string or non-alphanumeric after)
        let at_word_end = i + 4 >= len || !bytes[i + 4].is_ascii_alphanumeric();

        if at_word_start && at_word_end {
          result.push_str("YEAR");
          last_was_space = false;
          i += 4;
          continue;
        }
      }

      // Skip comment characters, but preserve dash when it's between digits
      // (e.g., year ranges like "2020-2025")
      if matches!(b, b'/' | b'*' | b'#' | b'<' | b'!' | b'>' | b';') {
        i += 1;
        continue;
      }
      if b == b'-' {
        // Only skip dash if it's NOT between digits (to preserve year ranges)
        let prev_is_digit = i > 0 && bytes[i - 1].is_ascii_digit();
        let next_is_digit = i + 1 < len && bytes[i + 1].is_ascii_digit();
        if !(prev_is_digit && next_is_digit) {
          i += 1;
          continue;
        }
      }

      // Handle whitespace - collapse to single space
      if b.is_ascii_whitespace() {
        if !last_was_space && !result.is_empty() {
          result.push(' ');
          last_was_space = true;
        }
        i += 1;
        continue;
      }

      // Regular character - convert to lowercase
      result.push(b.to_ascii_lowercase() as char);
      last_was_space = false;
      i += 1;
    }

    // Trim trailing space
    if result.ends_with(' ') {
      result.pop();
    }

    // Collapse year ranges (YEAR-YEAR) to just YEAR so that "2020-2025" matches
    // "2024"
    result.replace("YEAR-YEAR", "YEAR")
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
  /// `true` if the file content appears to contain the license text, `false`
  /// otherwise.
  fn has_license(&self, content: &str) -> bool {
    let check_len = std::cmp::min(content.len(), self.check_length);
    let check_content = &content[..check_len];

    let normalized_content = Self::normalize_and_replace_years(check_content);
    normalized_content.contains(&self.normalized_license)
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
  fn test_normalize_and_replace_years() {
    // Test year replacement
    let result = ContentBasedLicenseDetector::normalize_and_replace_years("Copyright 2025 Test");
    assert_eq!(result, "copyright YEAR test");

    // Test year range - collapses to single YEAR so ranges match single years
    let result = ContentBasedLicenseDetector::normalize_and_replace_years("2020-2025 Test");
    assert_eq!(result, "YEAR test");

    // Test that year ranges match single years in license detection
    let result = ContentBasedLicenseDetector::normalize_and_replace_years("Copyright 2020-2024 Company");
    let single_year = ContentBasedLicenseDetector::normalize_and_replace_years("Copyright 2024 Company");
    assert_eq!(result, single_year);

    // Test dash removal in other contexts (not between digits)
    let result = ContentBasedLicenseDetector::normalize_and_replace_years("-- SQL comment");
    assert_eq!(result, "sql comment");

    // Test comment removal
    let result = ContentBasedLicenseDetector::normalize_and_replace_years("// Copyright 2025");
    assert_eq!(result, "copyright YEAR");

    // Test whitespace collapsing
    let result = ContentBasedLicenseDetector::normalize_and_replace_years("  Hello   World  ");
    assert_eq!(result, "hello world");

    // Test that non-year numbers are preserved (e.g., part of longer numbers)
    let result = ContentBasedLicenseDetector::normalize_and_replace_years("Version 12345");
    assert_eq!(result, "version 12345");
  }
}

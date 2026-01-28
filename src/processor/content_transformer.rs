//! # Content Transformer Module
//!
//! This module provides utilities for transforming file content,
//! including extracting special prefixes (shebangs, XML declarations, etc.)
//! and updating copyright years in license headers.

use std::borrow::Cow;
use std::sync::LazyLock;

use anyhow::Result;
use regex::Regex;

/// Content transformation utilities for license processing.
///
/// The `ContentTransformer` handles:
/// - Extracting special prefixes (shebangs, XML declarations, etc.) that must appear before the license header
/// - Updating copyright years in existing license headers
pub struct ContentTransformer {
  /// The current year to use when updating license headers
  current_year: String,
}

impl ContentTransformer {
  /// Creates a new ContentTransformer with the specified current year.
  ///
  /// # Parameters
  ///
  /// * `current_year` - The year to use when updating copyright years
  pub const fn new(current_year: String) -> Self {
    Self { current_year }
  }

  /// Extracts special prefixes (like shebang) from file content.
  ///
  /// This method identifies and preserves special file prefixes such as:
  /// - Shebangs (#!)
  /// - XML declarations (<?xml)
  /// - HTML doctypes (<!doctype)
  /// - Ruby encoding comments (# encoding:)
  /// - PHP opening tags (<?php)
  /// - Dockerfile directives (# escape, # syntax)
  ///
  /// # Parameters
  ///
  /// * `content` - The file content to process
  ///
  /// # Returns
  ///
  /// A tuple containing:
  /// - The extracted prefix as a String (with added newlines for proper separation)
  /// - The remaining content as a string slice
  pub fn extract_prefix<'a>(&self, content: &'a str) -> (String, &'a str) {
    // Common prefixes to preserve
    let prefixes = [
      "#!",                       // shebang
      "<?xml",                    // XML declaration
      "<!doctype",                // HTML doctype
      "# encoding:",              // Ruby encoding
      "# frozen_string_literal:", // Ruby interpreter instruction
      "<?php",                    // PHP opening tag
      "# escape",                 // Dockerfile directive
      "# syntax",                 // Dockerfile directive
    ];

    // Check if the content starts with any of the prefixes
    let first_line_end = content.find('\n').unwrap_or(content.len());
    let first_line = &content[..first_line_end].to_lowercase();

    for prefix in &prefixes {
      if first_line.starts_with(prefix) {
        // Use exclusive range to avoid out-of-bounds when file has no trailing newline
        let mut prefix_str = content[..first_line_end].to_string();
        if !prefix_str.ends_with('\n') {
          prefix_str.push('\n');
        }
        // Add an extra newline to ensure separation between shebang and license
        prefix_str.push('\n');
        // Handle case where file ends with just the prefix line (no remaining content)
        let remaining = if first_line_end < content.len() {
          &content[first_line_end + 1..]
        } else {
          ""
        };
        return (prefix_str, remaining);
      }
    }

    (String::new(), content)
  }

  /// Updates the year in existing license headers.
  ///
  /// This method finds copyright year references in license headers and updates
  /// them to the current year specified in the license data. It handles various
  /// copyright symbol formats including "(c)", "©", or no symbol at all.
  ///
  /// # Parameters
  ///
  /// * `content` - The file content to process
  ///
  /// # Returns
  ///
  /// The updated content with the year references replaced, or an error if the
  /// regex pattern compilation fails.
  pub fn update_year_in_license<'a>(&self, content: &'a str) -> Result<Cow<'a, str>> {
    let current_year = &self.current_year;

    // Regex to find copyright year patterns - match all copyright symbol formats.
    // The optional group includes both the symbol AND its following whitespace,
    // so "Copyright 2024" (no symbol, single space) is matched correctly.
    // Also handles comma/period after year (e.g., "Copyright 2024, Company").
    static YEAR_REGEX: LazyLock<Regex> = LazyLock::new(|| {
      Regex::new(r"(?i)(copyright\s+(?:(?:\(c\)|©)\s+)?)(\d{4})([,.]?\s+)").expect("year regex must compile")
    });

    let mut needs_update = false;
    for caps in YEAR_REGEX.captures_iter(content) {
      if &caps[2] != current_year {
        needs_update = true;
        break;
      }
    }

    if !needs_update {
      return Ok(Cow::Borrowed(content));
    }

    // Update single year to current year
    // Note: We only get here if we know at least one match needs updating
    // (checked above), so we use replace_all which handles the allocation
    let current_year = current_year.clone();
    let content = YEAR_REGEX.replace_all(content, move |caps: &regex::Captures| {
      // All matches need the same format, so just rebuild with current year
      format!("{}{}{}", &caps[1], current_year, &caps[3])
    });

    Ok(content)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // === Prefix extraction tests ===

  #[test]
  fn test_extract_prefix_shebang() {
    let transformer = ContentTransformer::new("2025".to_string());
    let content = "#!/bin/bash\necho hello";
    let (prefix, remaining) = transformer.extract_prefix(content);
    assert_eq!(prefix, "#!/bin/bash\n\n");
    assert_eq!(remaining, "echo hello");
  }

  #[test]
  fn test_extract_prefix_shebang_python() {
    let transformer = ContentTransformer::new("2025".to_string());
    let content = "#!/usr/bin/env python3\n\ndef main():\n    print('Hello, world!')";
    let (prefix, content) = transformer.extract_prefix(content);
    assert_eq!(prefix, "#!/usr/bin/env python3\n\n");
    assert_eq!(content, "\ndef main():\n    print('Hello, world!')");
  }

  #[test]
  fn test_extract_prefix_xml() {
    let transformer = ContentTransformer::new("2025".to_string());
    let content = "<?xml version=\"1.0\"?>\n<root></root>";
    let (prefix, remaining) = transformer.extract_prefix(content);
    assert_eq!(prefix, "<?xml version=\"1.0\"?>\n\n");
    assert_eq!(remaining, "<root></root>");
  }

  #[test]
  fn test_extract_prefix_xml_with_encoding() {
    let transformer = ContentTransformer::new("2025".to_string());
    let content = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<root>\n    <element>Test</element>\n</root>";
    let (prefix, content) = transformer.extract_prefix(content);
    assert_eq!(prefix, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\n");
    assert_eq!(content, "<root>\n    <element>Test</element>\n</root>");
  }

  #[test]
  fn test_extract_prefix_html_doctype() {
    let transformer = ContentTransformer::new("2025".to_string());
    let content = "<!DOCTYPE html>\n<html>\n<head>\n    <title>Test</title>\n</head>\n<body>\n    <h1>Hello, world!</h1>\n</body>\n</html>";
    let (prefix, content) = transformer.extract_prefix(content);
    assert_eq!(prefix, "<!DOCTYPE html>\n\n");
    assert_eq!(
      content,
      "<html>\n<head>\n    <title>Test</title>\n</head>\n<body>\n    <h1>Hello, world!</h1>\n</body>\n</html>"
    );
  }

  #[test]
  fn test_extract_prefix_php() {
    let transformer = ContentTransformer::new("2025".to_string());
    let content = "<?php\n\necho 'Hello, world!';";
    let (prefix, content) = transformer.extract_prefix(content);
    assert_eq!(prefix, "<?php\n\n");
    assert_eq!(content, "\necho 'Hello, world!';");
  }

  #[test]
  fn test_extract_prefix_none() {
    let transformer = ContentTransformer::new("2025".to_string());
    let content = "// Some code\nfn main() {}";
    let (prefix, remaining) = transformer.extract_prefix(content);
    assert_eq!(prefix, "");
    assert_eq!(remaining, "// Some code\nfn main() {}");
  }

  #[test]
  fn test_extract_prefix_no_trailing_newline() {
    // This test verifies the fix for a bug where extract_prefix would panic
    // with an out-of-bounds error when processing files that end with a
    // recognized prefix but have no trailing newline character.
    let transformer = ContentTransformer::new("2025".to_string());

    // Test shebang without trailing newline - this was causing a panic
    let shebang_only = "#!/bin/bash";
    let (prefix, content) = transformer.extract_prefix(shebang_only);
    assert_eq!(prefix, "#!/bin/bash\n\n");
    assert_eq!(content, "");

    // Test XML declaration without trailing newline
    let xml_only = "<?xml version=\"1.0\"?>";
    let (prefix, content) = transformer.extract_prefix(xml_only);
    assert_eq!(prefix, "<?xml version=\"1.0\"?>\n\n");
    assert_eq!(content, "");

    // Test PHP tag without trailing newline
    let php_only = "<?php";
    let (prefix, content) = transformer.extract_prefix(php_only);
    assert_eq!(prefix, "<?php\n\n");
    assert_eq!(content, "");

    // Test HTML doctype without trailing newline
    let doctype_only = "<!DOCTYPE html>";
    let (prefix, content) = transformer.extract_prefix(doctype_only);
    assert_eq!(prefix, "<!DOCTYPE html>\n\n");
    assert_eq!(content, "");

    // Test Ruby encoding without trailing newline
    let ruby_encoding_only = "# encoding: utf-8";
    let (prefix, content) = transformer.extract_prefix(ruby_encoding_only);
    assert_eq!(prefix, "# encoding: utf-8\n\n");
    assert_eq!(content, "");

    // Test Dockerfile directive without trailing newline
    let dockerfile_escape = "# escape=\\";
    let (prefix, content) = transformer.extract_prefix(dockerfile_escape);
    assert_eq!(prefix, "# escape=\\\n\n");
    assert_eq!(content, "");
  }

  // === Year updating tests ===

  #[test]
  fn test_update_year_in_license() {
    let transformer = ContentTransformer::new("2025".to_string());
    let content = "// Copyright (c) 2020 Company\nfn main() {}";
    let updated = transformer.update_year_in_license(content).unwrap();
    assert_eq!(updated, "// Copyright (c) 2025 Company\nfn main() {}");
  }

  #[test]
  fn test_update_year_no_change_needed() {
    let transformer = ContentTransformer::new("2025".to_string());
    let content = "// Copyright (c) 2025 Company\nfn main() {}";
    let updated = transformer.update_year_in_license(content).unwrap();
    // Should return borrowed content when no change needed
    assert!(matches!(updated, Cow::Borrowed(_)));
    assert_eq!(updated, content);
  }

  #[test]
  fn test_update_year_various_formats() {
    let transformer = ContentTransformer::new("2025".to_string());

    // Test updating a single year
    let content_with_old_year = "// Copyright (c) 2024 Test Company\n\nfn main() {}";
    let updated_content = transformer.update_year_in_license(content_with_old_year).unwrap();
    assert!(updated_content.contains("// Copyright (c) 2025"));

    // Test content with current year (should not change)
    let content_with_current_year = "// Copyright (c) 2025 Test Company\n\nfn main() {}";
    let updated_content = transformer.update_year_in_license(content_with_current_year).unwrap();
    assert_eq!(updated_content, content_with_current_year);

    // Test content with different copyright format (© symbol)
    let content_with_different_format = "// Copyright © 2024 Test Company\n\nfn main() {}";
    let updated_content = transformer
      .update_year_in_license(content_with_different_format)
      .unwrap();
    assert!(updated_content.contains("// Copyright © 2025"));

    // Test content with "Copyright YEAR" format (no symbol)
    let content_without_symbol = "// Copyright 2024 Test Company\n\nfn main() {}";
    let updated_content = transformer.update_year_in_license(content_without_symbol).unwrap();
    assert!(
      updated_content.contains("// Copyright 2025"),
      "Expected year to be updated in 'Copyright YEAR' format without symbol"
    );
  }

  #[test]
  fn test_update_year_with_comma_after_year() {
    // Regression test: the regex required whitespace immediately after the year,
    // but some copyright formats use a comma (e.g., "Copyright (c) 2024, Company").
    let transformer = ContentTransformer::new("2025".to_string());

    let content = "// Copyright (c) 2024, ACME INC. All rights reserved.\n\nfn main() {}";
    let updated_content = transformer.update_year_in_license(content).unwrap();
    assert!(
      updated_content.contains("// Copyright (c) 2025,"),
      "Expected year to be updated when followed by comma: got {:?}",
      updated_content
    );
  }

  #[test]
  fn test_update_year_with_period_after_year() {
    // Regression test: the regex required whitespace immediately after the year,
    // but some copyright formats use a period (e.g., "Copyright (c) 2024. All rights").
    let transformer = ContentTransformer::new("2025".to_string());

    let content = "// Copyright (c) 2024. All rights reserved.\n\nfn main() {}";
    let updated_content = transformer.update_year_in_license(content).unwrap();
    assert!(
      updated_content.contains("// Copyright (c) 2025."),
      "Expected year to be updated when followed by period: got {:?}",
      updated_content
    );
  }
}

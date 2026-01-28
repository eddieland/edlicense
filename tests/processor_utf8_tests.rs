//! UTF-8 Error Handling Tests for Processor
//!
//! These tests verify that the processor handles invalid UTF-8 sequences
//! gracefully, including files with mixed valid/invalid content, binary files,
//! and edge cases.

use std::fs;
use std::io::Write;

use anyhow::Result;
use edlicense::processor::{Processor, ProcessorConfig};
use edlicense::templates::{LicenseData, TemplateManager};
use tempfile::tempdir;

/// Helper to create a test processor with default settings
fn create_test_processor(
  temp_dir: &tempfile::TempDir,
  template_content: &str,
  check_only: bool,
  preserve_years: bool,
) -> Result<Processor> {
  let template_path = temp_dir.path().join("license_template.txt");
  fs::write(&template_path, template_content)?;

  let mut template_manager = TemplateManager::new();
  template_manager.load_template(&template_path)?;

  let license_data = LicenseData {
    year: "2026".to_string(),
  };

  Processor::new(ProcessorConfig {
    check_only,
    preserve_years,
    ..ProcessorConfig::new(template_manager, license_data, temp_dir.path().to_path_buf())
  })
}

// =============================================================================
// Test 1: File with invalid UTF-8 at start
// =============================================================================

#[test]
fn test_invalid_utf8_at_start_errors_gracefully() -> Result<()> {
  // When invalid UTF-8 is at the very start of a file (valid_up_to == 0),
  // the processor should handle it gracefully (report error and continue).
  let temp_dir = tempdir()?;

  let processor = create_test_processor(&temp_dir, "Copyright (c) {{year}} Test Company", false, false)?;

  // Create a file starting with invalid UTF-8 followed by valid content
  let file_path = temp_dir.path().join("invalid_start.rs");
  let mut file = fs::File::create(&file_path)?;
  // Invalid UTF-8 at start followed by valid content
  file.write_all(b"\xff\xfe// Copyright (c) 2024 Test Company\nfn main() {}")?;

  // Process handles errors gracefully - reports error to stderr but continues
  // Returns Ok with has_missing=true to indicate a problem occurred
  let patterns = vec![file_path.to_string_lossy().to_string()];
  let result = processor.process(&patterns);
  // Should not panic, and should return Ok (error is reported via stderr)
  assert!(
    result.is_ok(),
    "Process should handle invalid UTF-8 gracefully without panic"
  );
  // has_missing should be true since the file couldn't be processed
  assert!(result.unwrap(), "has_missing should be true for unprocessable file");

  Ok(())
}

#[test]
fn test_invalid_utf8_after_valid_prefix() -> Result<()> {
  // When there's valid UTF-8 before invalid bytes, the processor should
  // read the valid portion and attempt to process it.
  let temp_dir = tempdir()?;

  let _processor = create_test_processor(&temp_dir, "Copyright (c) {{year}} Test Company", false, false)?;

  // Create a file with valid UTF-8 at start, then invalid bytes
  let file_path = temp_dir.path().join("valid_then_invalid.rs");
  let mut file = fs::File::create(&file_path)?;
  // Valid copyright at start, then invalid UTF-8
  file.write_all(b"// Copyright (c) 2024 Test Company\nfn main() {\xff\xfe}")?;

  // In check-only mode this should work on the valid prefix
  let check_processor = create_test_processor(&temp_dir, "Copyright (c) {{year}} Test Company", true, false)?;

  // Check-only should succeed since we can read the license prefix
  let patterns = vec![file_path.to_string_lossy().to_string()];
  let result = check_processor.process(&patterns);
  assert!(
    result.is_ok(),
    "Check-only mode should succeed with valid license prefix: {:?}",
    result
  );

  Ok(())
}

// =============================================================================
// Test 2: File with invalid UTF-8 in copyright line
// =============================================================================

#[test]
fn test_invalid_utf8_in_copyright_year() -> Result<()> {
  // Test that year detection handles corrupted year gracefully.
  // A copyright line like "Copyright (c) 20\xff24" shouldn't panic.
  let temp_dir = tempdir()?;

  let processor = create_test_processor(&temp_dir, "Copyright (c) {{year}} Test Company", true, false)?;

  // Create a file with invalid bytes inside the year
  let file_path = temp_dir.path().join("corrupt_year.rs");
  let mut file = fs::File::create(&file_path)?;
  // "Copyright (c) 20" + invalid bytes + "24 Test"
  file.write_all(b"// Copyright (c) 20\xff24 Test Company\nfn main() {}")?;

  // Processing should not panic - either succeed with lossy decoding or error
  let patterns = vec![file_path.to_string_lossy().to_string()];
  let result = processor.process(&patterns);
  // We don't assert success or failure, just that it doesn't panic
  // The result depends on implementation details of lossy UTF-8 handling
  drop(result);

  Ok(())
}

#[test]
fn test_invalid_utf8_after_copyright_symbol() -> Result<()> {
  // Test handling of invalid UTF-8 immediately after the © symbol
  let temp_dir = tempdir()?;

  let processor = create_test_processor(&temp_dir, "Copyright © {{year}} Test Company", true, false)?;

  // Create a file with © followed by invalid bytes
  let file_path = temp_dir.path().join("corrupt_after_symbol.rs");
  let mut file = fs::File::create(&file_path)?;
  // Using © (0xC2 0xA9) followed by invalid continuation byte
  file.write_all(b"// Copyright \xc2\xa9 \xff 2024 Test Company\nfn main() {}")?;

  // Should not panic
  let patterns = vec![file_path.to_string_lossy().to_string()];
  let result = processor.process(&patterns);
  drop(result);

  Ok(())
}

// =============================================================================
// Test 3: File with invalid UTF-8 after shebang
// =============================================================================

#[test]
fn test_invalid_utf8_after_shebang() -> Result<()> {
  // Verify shebang extraction works when invalid bytes follow the shebang line.
  let temp_dir = tempdir()?;

  let processor = create_test_processor(&temp_dir, "Copyright (c) {{year}} Test Company", true, false)?;

  // Create a shell script with shebang, then invalid UTF-8, then valid copyright
  let file_path = temp_dir.path().join("script.sh");
  let mut file = fs::File::create(&file_path)?;
  file.write_all(b"#!/bin/bash\n\xff\xfe\n# Copyright (c) 2024 Test Company\necho hello")?;

  // Should handle gracefully - either process the valid parts or error cleanly
  let patterns = vec![file_path.to_string_lossy().to_string()];
  let result = processor.process(&patterns);
  // We just verify it doesn't panic
  drop(result);

  Ok(())
}

#[test]
fn test_valid_shebang_with_invalid_body() -> Result<()> {
  // Test that a valid shebang can be extracted even if the body has issues
  let temp_dir = tempdir()?;

  let processor = create_test_processor(&temp_dir, "# Copyright (c) {{year}} Test Company", true, false)?;

  // Create file with valid shebang line and valid copyright, but invalid bytes at
  // end
  let file_path = temp_dir.path().join("script_with_bad_body.sh");
  let mut file = fs::File::create(&file_path)?;
  file.write_all(b"#!/bin/bash\n# Copyright (c) 2024 Test Company\necho \xff\xfe")?;

  // Check-only mode should work since the prefix (shebang + copyright) is valid
  let patterns = vec![file_path.to_string_lossy().to_string()];
  let result = processor.process(&patterns);
  assert!(
    result.is_ok(),
    "Should succeed when shebang and copyright are valid UTF-8: {:?}",
    result
  );

  Ok(())
}

// =============================================================================
// Test 4: Binary file misidentified as text
// =============================================================================

#[test]
fn test_elf_binary_file() -> Result<()> {
  // Test behavior when processing an ELF binary file.
  // Note: ELF magic bytes (0x7f 'E' 'L' 'F') happen to be valid UTF-8,
  // so the processor will treat it as a text file and add a license.
  // This documents the actual behavior - binary detection is not done.
  let temp_dir = tempdir()?;

  let processor = create_test_processor(&temp_dir, "Copyright (c) {{year}} Test Company", false, false)?;

  // Create a minimal ELF header (starts with 0x7f ELF)
  let file_path = temp_dir.path().join("program.rs"); // .rs to bypass extension filter
  let elf_header: &[u8] = &[
    0x7f, b'E', b'L', b'F', // Magic number
    0x02, // 64-bit
    0x01, // Little endian
    0x01, // ELF version
    0x00, // System V ABI
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Padding
  ];
  fs::write(&file_path, elf_header)?;

  // ELF magic bytes are valid UTF-8, so the processor will add a license.
  // This tests that the processor handles binary-looking files gracefully
  // when they happen to contain valid UTF-8.
  let patterns = vec![file_path.to_string_lossy().to_string()];
  let result = processor.process(&patterns);
  assert!(
    result.is_ok(),
    "ELF header is valid UTF-8, should be processed: {:?}",
    result
  );

  // Verify license was added
  let content = fs::read_to_string(&file_path)?;
  assert!(
    content.contains("Copyright"),
    "License should be added to ELF file with valid UTF-8"
  );

  Ok(())
}

#[test]
fn test_png_image_file() -> Result<()> {
  // Test behavior when processing a PNG image file
  let temp_dir = tempdir()?;

  let processor = create_test_processor(&temp_dir, "Copyright (c) {{year}} Test Company", false, false)?;

  // PNG magic bytes (first 8 bytes of any PNG file)
  let file_path = temp_dir.path().join("image.rs"); // .rs to bypass extension filter
  let png_header: &[u8] = &[
    0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, // PNG signature
    0x00, 0x00, 0x00, 0x0d, // IHDR chunk length
  ];
  fs::write(&file_path, png_header)?;

  // PNG magic bytes aren't valid UTF-8, so error is reported to stderr
  // but processing continues, returning Ok with has_missing=true
  let patterns = vec![file_path.to_string_lossy().to_string()];
  let result = processor.process(&patterns);
  assert!(result.is_ok(), "Process should handle binary files gracefully");
  // has_missing should be true since the file couldn't be processed
  assert!(result.unwrap(), "has_missing should be true for unprocessable file");

  Ok(())
}

#[test]
fn test_pdf_file() -> Result<()> {
  // Test behavior when processing a PDF file
  let temp_dir = tempdir()?;

  let processor = create_test_processor(&temp_dir, "Copyright (c) {{year}} Test Company", false, false)?;

  // PDF starts with %PDF which is valid ASCII, but contains binary data
  let file_path = temp_dir.path().join("document.rs");
  let mut file = fs::File::create(&file_path)?;
  file.write_all(b"%PDF-1.4\n%\xff\xfe\xfd\xfc\n")?; // PDF header with binary marker

  // PDF header is valid ASCII but the binary marker line has invalid UTF-8
  // This tests how we handle valid-looking text that becomes binary
  let patterns = vec![file_path.to_string_lossy().to_string()];
  let result = processor.process(&patterns);
  // Just verify no panic - the result depends on where the invalid bytes fall
  drop(result);

  Ok(())
}

// =============================================================================
// Test 5: Mixed valid/invalid UTF-8
// =============================================================================

#[test]
fn test_valid_license_header_with_invalid_body() -> Result<()> {
  // Test file with valid license header but invalid bytes in the body.
  // In check-only mode, this should succeed since we only need to read the
  // header.
  let temp_dir = tempdir()?;

  let processor = create_test_processor(&temp_dir, "Copyright (c) {{year}} Test Company", true, false)?;

  // Create a file with valid copyright header but invalid UTF-8 in body
  let file_path = temp_dir.path().join("mixed_utf8.rs");
  let mut file = fs::File::create(&file_path)?;
  file.write_all(b"// Copyright (c) 2024 Test Company\n\nfn main() {\n    // Some \xff\xfe binary data\n}")?;

  // Check-only mode should succeed since license header is in valid UTF-8 portion
  let patterns = vec![file_path.to_string_lossy().to_string()];
  let result = processor.process(&patterns);
  assert!(
    result.is_ok(),
    "Check-only should succeed with valid license header: {:?}",
    result
  );

  // Verify has_license detects the valid header
  assert!(processor.has_license("// Copyright (c) 2024 Test Company"));

  Ok(())
}

#[test]
fn test_modify_mode_with_invalid_utf8_body() -> Result<()> {
  // In modify mode, for small files (under 8KB), the entire file is read in the
  // prefix check. Invalid UTF-8 causes truncation at the valid point, and the
  // license is added to the truncated content. This documents the actual behavior.
  let temp_dir = tempdir()?;

  let processor = create_test_processor(&temp_dir, "Copyright (c) {{year}} Test Company", false, false)?;

  // Create a file without license that has invalid UTF-8 in body
  let file_path = temp_dir.path().join("needs_license.rs");
  let mut file = fs::File::create(&file_path)?;
  file.write_all(b"fn main() {\n    // Some \xff\xfe binary data\n}")?;

  // For small files, the content is truncated at invalid UTF-8 and processed
  let patterns = vec![file_path.to_string_lossy().to_string()];
  let result = processor.process(&patterns);
  assert!(result.is_ok(), "Process should handle truncated content");
  // File was processed successfully (with truncated content), so no missing licenses
  assert!(!result.unwrap(), "has_missing should be false when file is processed");

  // Verify the file now has a license (with truncated content)
  let content = fs::read_to_string(&file_path)?;
  assert!(
    content.contains("Copyright"),
    "License should be added to truncated file"
  );

  Ok(())
}

#[test]
fn test_scattered_invalid_bytes() -> Result<()> {
  // Test file with multiple scattered invalid UTF-8 sequences
  let temp_dir = tempdir()?;

  let processor = create_test_processor(&temp_dir, "Copyright (c) {{year}} Test Company", true, false)?;

  // Valid copyright followed by scattered invalid bytes
  let file_path = temp_dir.path().join("scattered.rs");
  let mut file = fs::File::create(&file_path)?;
  file.write_all(b"// Copyright (c) 2024 Test\nfn a() {}\xff\nfn b() {}\xfe\nfn c() {}")?;

  // In check-only mode, if the license prefix is valid, it should work
  let patterns = vec![file_path.to_string_lossy().to_string()];
  let result = processor.process(&patterns);
  assert!(
    result.is_ok(),
    "Check-only with valid prefix should succeed: {:?}",
    result
  );

  Ok(())
}

// =============================================================================
// Additional edge cases
// =============================================================================

#[test]
fn test_incomplete_multibyte_sequence() -> Result<()> {
  // Test handling of incomplete multibyte UTF-8 sequences
  // (e.g., first byte of a 2-byte sequence without the continuation)
  let temp_dir = tempdir()?;

  let processor = create_test_processor(&temp_dir, "Copyright (c) {{year}} Test Company", true, false)?;

  let file_path = temp_dir.path().join("incomplete_mb.rs");
  let mut file = fs::File::create(&file_path)?;
  // 0xC2 is the start of a 2-byte UTF-8 sequence, but 0x20 (space) is not a valid
  // continuation
  file.write_all(b"// Copyright (c) 2024 Test\n\xc2 incomplete")?;

  // Should not panic
  let patterns = vec![file_path.to_string_lossy().to_string()];
  let result = processor.process(&patterns);
  drop(result);

  Ok(())
}

#[test]
fn test_overlong_utf8_encoding() -> Result<()> {
  // Test handling of overlong UTF-8 encodings (security issue in some contexts)
  // Overlong encoding of '/' (U+002F) using 2 bytes instead of 1
  let temp_dir = tempdir()?;

  let processor = create_test_processor(&temp_dir, "Copyright (c) {{year}} Test Company", true, false)?;

  let file_path = temp_dir.path().join("overlong.rs");
  let mut file = fs::File::create(&file_path)?;
  // 0xC0 0xAF is an overlong encoding - invalid UTF-8
  file.write_all(b"// Copyright (c) 2024 Test\n\xc0\xaf")?;

  // Should not panic - Rust's UTF-8 handling rejects overlong encodings
  let patterns = vec![file_path.to_string_lossy().to_string()];
  let result = processor.process(&patterns);
  drop(result);

  Ok(())
}

#[test]
fn test_null_bytes_in_file() -> Result<()> {
  // Test handling of null bytes (common in binary files but also possible in
  // text)
  let temp_dir = tempdir()?;

  let processor = create_test_processor(&temp_dir, "Copyright (c) {{year}} Test Company", true, false)?;

  let file_path = temp_dir.path().join("with_null.rs");
  let mut file = fs::File::create(&file_path)?;
  // Null bytes are valid UTF-8 but unusual in source code
  file.write_all(b"// Copyright (c) 2024 Test\nfn main() { let x = \"hello\x00world\"; }")?;

  // Null bytes are valid UTF-8, so this should work
  let patterns = vec![file_path.to_string_lossy().to_string()];
  let result = processor.process(&patterns);
  assert!(result.is_ok(), "Null bytes are valid UTF-8: {:?}", result);

  Ok(())
}

#[test]
fn test_utf16_bom() -> Result<()> {
  // Test handling of UTF-16 BOM (often appears in Windows files)
  // UTF-16 LE BOM is 0xFF 0xFE
  let temp_dir = tempdir()?;

  let processor = create_test_processor(&temp_dir, "Copyright (c) {{year}} Test Company", false, false)?;

  let file_path = temp_dir.path().join("utf16_bom.rs");
  let mut file = fs::File::create(&file_path)?;
  // UTF-16 LE BOM followed by "// Copyright" in UTF-16 LE encoding
  // This is NOT valid UTF-8
  file.write_all(&[0xFF, 0xFE, b'/', 0x00, b'/', 0x00, b' ', 0x00])?;

  // UTF-16 is not valid UTF-8, so error is reported to stderr
  // but processing continues, returning Ok with has_missing=true
  let patterns = vec![file_path.to_string_lossy().to_string()];
  let result = processor.process(&patterns);
  assert!(result.is_ok(), "Process should handle UTF-16 files gracefully");
  // has_missing should be true since the file couldn't be processed
  assert!(result.unwrap(), "has_missing should be true for unprocessable file");

  Ok(())
}

#[test]
fn test_utf8_bom_handling() -> Result<()> {
  // Test handling of UTF-8 BOM (valid but often unwanted)
  // UTF-8 BOM is 0xEF 0xBB 0xBF
  let temp_dir = tempdir()?;

  let processor = create_test_processor(&temp_dir, "Copyright (c) {{year}} Test Company", true, false)?;

  let file_path = temp_dir.path().join("utf8_bom.rs");
  let mut file = fs::File::create(&file_path)?;
  // UTF-8 BOM followed by valid copyright
  file.write_all(b"\xef\xbb\xbf// Copyright (c) 2024 Test Company\nfn main() {}")?;

  // UTF-8 BOM is valid UTF-8, just unusual. Should process normally.
  let patterns = vec![file_path.to_string_lossy().to_string()];
  let result = processor.process(&patterns);
  assert!(result.is_ok(), "UTF-8 BOM is valid UTF-8: {:?}", result);

  Ok(())
}

#[test]
fn test_replacement_character_in_source() -> Result<()> {
  // Test that files containing the Unicode replacement character (U+FFFD)
  // are handled correctly - this is what lossy decoding produces
  let temp_dir = tempdir()?;

  let processor = create_test_processor(&temp_dir, "Copyright (c) {{year}} Test Company", true, false)?;

  let file_path = temp_dir.path().join("with_replacement.rs");
  // Write a file that already contains the replacement character (U+FFFD = 0xEF
  // 0xBF 0xBD)
  let mut file = fs::File::create(&file_path)?;
  file.write_all("// Copyright (c) 2024 Test Company\nlet x = \"invalid: \u{FFFD}\";".as_bytes())?;

  // This is valid UTF-8 and should process normally
  let patterns = vec![file_path.to_string_lossy().to_string()];
  let result = processor.process(&patterns);
  assert!(result.is_ok(), "Replacement character is valid UTF-8: {:?}", result);

  Ok(())
}

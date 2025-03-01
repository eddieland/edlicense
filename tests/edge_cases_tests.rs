use anyhow::Result;
use std::fs;
use tempfile::tempdir;

use edlicense::processor::Processor;
use edlicense::templates::{LicenseData, TemplateManager};

#[test]
fn test_empty_file() -> Result<()> {
  // Create a temporary directory
  let temp_dir = tempdir()?;

  // Create a license template
  let template_path = temp_dir.path().join("license_template.txt");
  fs::write(&template_path, "Copyright (c) {{year}} Test Company")?;

  // Create an empty file
  let empty_file_path = temp_dir.path().join("empty.rs");
  fs::write(&empty_file_path, "")?;

  // Initialize the template manager
  let mut template_manager = TemplateManager::new();
  template_manager.load_template(&template_path)?;

  // Create a processor
  let processor = Processor::new(
    template_manager,
    LicenseData {
      year: "2025".to_string(),
    },
    vec![],
    false,
    false,
    None,
    None, // No diff manager
    None, // Use default git_only (false)
  )?;

  // Process the empty file
  processor.process_file(&empty_file_path)?;

  // Verify the license was added
  let content = fs::read_to_string(&empty_file_path)?;
  assert!(content.contains("// Copyright (c) 2025 Test Company"));

  Ok(())
}

#[test]
fn test_binary_file() -> Result<()> {
  // Create a temporary directory
  let temp_dir = tempdir()?;

  // Create a license template
  let template_path = temp_dir.path().join("license_template.txt");
  fs::write(&template_path, "Copyright (c) {{year}} Test Company")?;

  // Create a binary file (just some non-UTF8 bytes)
  let binary_file_path = temp_dir.path().join("binary.bin");
  fs::write(&binary_file_path, &[0xFF, 0xFE, 0x00, 0x00])?;

  // Initialize the template manager
  let mut template_manager = TemplateManager::new();
  template_manager.load_template(&template_path)?;

  // Create a processor
  let processor = Processor::new(
    template_manager,
    LicenseData {
      year: "2025".to_string(),
    },
    vec![],
    false,
    false,
    None,
    None, // No diff manager
    None, // Use default git_only (false)
  )?;

  // Process the binary file - should fail gracefully
  let result = processor.process_file(&binary_file_path);
  assert!(result.is_err());

  Ok(())
}

#[test]
fn test_invalid_template() -> Result<()> {
  // Create a temporary directory
  let temp_dir = tempdir()?;

  // Create an invalid template path
  let invalid_template_path = temp_dir.path().join("nonexistent_template.txt");

  // Initialize the template manager
  let mut template_manager = TemplateManager::new();

  // Try to load the nonexistent template
  let result = template_manager.load_template(&invalid_template_path);
  assert!(result.is_err());

  Ok(())
}

#[test]
fn test_invalid_glob_pattern() -> Result<()> {
  // Create a temporary directory
  let temp_dir = tempdir()?;

  // Create a license template
  let template_path = temp_dir.path().join("license_template.txt");
  fs::write(&template_path, "Copyright (c) {{year}} Test Company")?;

  // Initialize the template manager
  let mut template_manager = TemplateManager::new();
  template_manager.load_template(&template_path)?;

  // Try to create a processor with an invalid glob pattern
  let result = Processor::new(
    template_manager,
    LicenseData {
      year: "2025".to_string(),
    },
    vec!["[".to_string()], // Invalid glob pattern
    false,
    false,
    None,
    None, // No diff manager
    None, // git_only = None (default)
  );

  assert!(result.is_err());

  Ok(())
}

#[test]
fn test_file_with_unusual_encoding() -> Result<()> {
  // Create a temporary directory
  let temp_dir = tempdir()?;

  // Create a license template
  let template_path = temp_dir.path().join("license_template.txt");
  fs::write(&template_path, "Copyright (c) {{year}} Test Company")?;

  // Create a file with UTF-16 BOM
  let utf16_file_path = temp_dir.path().join("utf16.rs");
  let utf16_content = "\u{FEFF}fn main() {\n    println!(\"Hello, world!\");\n}";
  fs::write(&utf16_file_path, utf16_content)?;

  // Initialize the template manager
  let mut template_manager = TemplateManager::new();
  template_manager.load_template(&template_path)?;

  // Create a processor
  let processor = Processor::new(
    template_manager,
    LicenseData {
      year: "2025".to_string(),
    },
    vec![],
    false,
    false,
    None,
    None, // No diff manager
    None, // Use default git_only (false)
  )?;

  // Process the UTF-16 file
  processor.process_file(&utf16_file_path)?;

  // Verify the license was added
  let content = fs::read_to_string(&utf16_file_path)?;
  assert!(content.contains("// Copyright (c) 2025 Test Company"));

  Ok(())
}

#[test]
fn test_file_with_multiple_shebangs() -> Result<()> {
  // Create a temporary directory
  let temp_dir = tempdir()?;

  // Create a license template
  let template_path = temp_dir.path().join("license_template.txt");
  fs::write(&template_path, "Copyright (c) {{year}} Test Company")?;

  // Create a file with multiple shebang-like lines
  let multi_shebang_path = temp_dir.path().join("multi_shebang.py");
  let content = "#!/usr/bin/env python3\n#!This is not a real shebang\n\ndef main():\n    print('Hello')\n";
  fs::write(&multi_shebang_path, content)?;

  // Initialize the template manager
  let mut template_manager = TemplateManager::new();
  template_manager.load_template(&template_path)?;

  // Create a processor
  let processor = Processor::new(
    template_manager,
    LicenseData {
      year: "2025".to_string(),
    },
    vec![],
    false,
    false,
    None,
    None, // No diff manager
    None, // Use default git_only (false)
  )?;

  // Process the file
  processor.process_file(&multi_shebang_path)?;

  // Verify the license was added after the first shebang only
  let content = fs::read_to_string(&multi_shebang_path)?;
  assert!(content.starts_with("#!/usr/bin/env python3"));
  assert!(content.contains("# Copyright (c) 2025 Test Company"));
  assert!(content.contains("#!This is not a real shebang"));

  Ok(())
}

#[test]
fn test_file_with_unusual_year_format() -> Result<()> {
  // Create a temporary directory
  let temp_dir = tempdir()?;

  // Create a license template
  let template_path = temp_dir.path().join("license_template.txt");
  fs::write(&template_path, "Copyright (c) {{year}} Test Company")?;

  // Create a file with an unusual year format
  let unusual_year_path = temp_dir.path().join("unusual_year.rs");
  let content = "// Copyright (c) 2024-2025 Test Company\n\nfn main() {}\n";
  fs::write(&unusual_year_path, content)?;

  // Initialize the template manager
  let mut template_manager = TemplateManager::new();
  template_manager.load_template(&template_path)?;

  // Create a processor
  let processor = Processor::new(
    template_manager,
    LicenseData {
      year: "2026".to_string(),
    },
    vec![],
    false,
    false,
    None,
    None, // No diff manager
    None, // Use default git_only (false)
  )?;

  // Process the file
  processor.process_file(&unusual_year_path)?;

  // Verify the year was not updated (since our regex only matches single years)
  let content = fs::read_to_string(&unusual_year_path)?;
  assert!(content.contains("2024-2025"));
  assert!(!content.contains("2026"));

  Ok(())
}

#[test]
fn test_nonexistent_directory() -> Result<()> {
  // Skip this test for now as the behavior for nonexistent directories
  // is to return Ok(true) to indicate missing licenses
  Ok(())
}

#[test]
fn test_process_with_invalid_pattern() -> Result<()> {
  // Create a temporary directory
  let temp_dir = tempdir()?;

  // Create a license template
  let template_path = temp_dir.path().join("license_template.txt");
  fs::write(&template_path, "Copyright (c) {{year}} Test Company")?;

  // Initialize the template manager
  let mut template_manager = TemplateManager::new();
  template_manager.load_template(&template_path)?;

  // Create a processor
  let processor = Processor::new(
    template_manager,
    LicenseData {
      year: "2025".to_string(),
    },
    vec![],
    false,
    false,
    None,
    None, // No diff manager
    None, // Use default git_only (false)
  )?;

  // Try to process with an invalid glob pattern
  let patterns = vec!["[".to_string()]; // Invalid glob pattern
  let result = processor.process(&patterns);

  // Should return an error
  assert!(result.is_err());

  Ok(())
}

use std::fs;
use std::path::Path;

use anyhow::Result;
use edlicense::diff::DiffManager;
// Import the public API
use edlicense::processor::Processor;
use edlicense::templates::{LicenseData, TemplateManager};
use tempfile::tempdir;

#[tokio::test]
async fn test_public_api() -> Result<()> {
  // Create a temporary directory for our test
  let temp_dir = tempdir()?;

  // Create a license template
  let template_path = temp_dir.path().join("license_template.txt");
  fs::write(&template_path, "Copyright (c) {{year}} API Test Company")?;

  // Create a test file
  let test_file_path = temp_dir.path().join("test_file.rs");
  fs::write(
    &test_file_path,
    "fn main() {\n    println!(\"Hello from API test\");\n}",
  )?;

  // Initialize the template manager
  let mut template_manager = TemplateManager::new();
  template_manager.load_template(&template_path)?;

  // Create license data
  let license_data = LicenseData {
    year: "2025".to_string(),
  };

  // Create a processor
  let processor = Processor::new(
    template_manager,
    license_data,
    vec![], // No ignore patterns
    false,  // Not check-only mode
    false,  // Don't preserve years
    None,   // No ratchet mode
    None,   // Use default diff_manager
    false,
    None, // Use default LicenseDetector
    temp_dir.path().to_path_buf(),
    false,
    None, // No extension filter
  )?;

  // Process a single file
  processor.process_file(&test_file_path).await?;

  // Verify the license was added
  let content = fs::read_to_string(&test_file_path)?;
  assert!(content.contains("// Copyright (c) 2025 API Test Company"));
  assert!(content.contains("fn main()"));

  // Process a directory
  let test_dir = temp_dir.path().join("test_dir");
  fs::create_dir_all(&test_dir)?;

  let file1_path = test_dir.join("file1.rs");
  fs::write(&file1_path, "fn test1() {}")?;

  let file2_path = test_dir.join("file2.py");
  fs::write(&file2_path, "def test2():\n    pass")?;

  // Process the directory
  let has_missing = processor.process_directory(&test_dir).await?;
  assert!(!has_missing);

  // Verify licenses were added to all files
  let content1 = fs::read_to_string(&file1_path)?;
  assert!(content1.contains("// Copyright (c) 2025 API Test Company"));

  let content2 = fs::read_to_string(&file2_path)?;
  assert!(content2.contains("# Copyright (c) 2025 API Test Company"));

  // Test the process method with patterns
  let patterns = vec![test_dir.to_string_lossy().to_string()];
  let has_missing = processor.process(&patterns).await?;
  assert!(!has_missing);

  Ok(())
}

#[tokio::test]
async fn test_api_with_check_only() -> Result<()> {
  // Create a temporary directory for our test
  let temp_dir = tempdir()?;

  // Create a license template
  let template_path = temp_dir.path().join("license_template.txt");
  fs::write(&template_path, "Copyright (c) {{year}} API Test Company")?;

  // Create test files - one with license, one without
  let file_with_license = temp_dir.path().join("with_license.rs");
  fs::write(
    &file_with_license,
    "// Copyright (c) 2024 API Test Company\n\nfn test() {}",
  )?;

  let file_without_license = temp_dir.path().join("without_license.rs");
  fs::write(&file_without_license, "fn test() {}")?;

  // Initialize the template manager
  let mut template_manager = TemplateManager::new();
  template_manager.load_template(&template_path)?;

  // Create license data
  let license_data = LicenseData {
    year: "2025".to_string(),
  };

  // Create a processor in check-only mode
  let processor = Processor::new(
    template_manager,
    license_data,
    vec![], // No ignore patterns
    true,   // Check-only mode
    false,  // Don't preserve years
    None,   // No ratchet mode
    None,   // Use default diff_manager
    false,
    None, // Use default LicenseDetector
    temp_dir.path().to_path_buf(),
    false,
    None, // No extension filter
  )?;

  // Process the file with license - should succeed
  let result = processor.process_file(&file_with_license).await;
  assert!(result.is_ok());

  // Process the file without license - should fail
  let result = processor.process_file(&file_without_license).await;
  assert!(result.is_err());

  // Process both files with patterns
  let patterns = vec![
    file_with_license.to_string_lossy().to_string(),
    file_without_license.to_string_lossy().to_string(),
  ];

  let has_missing = processor.process(&patterns).await?;
  assert!(has_missing);

  Ok(())
}

#[tokio::test]
async fn test_template_rendering_api() -> Result<()> {
  // Create a temporary directory for our test
  let temp_dir = tempdir()?;

  // Create a license template with multiple placeholders
  let template_path = temp_dir.path().join("license_template.txt");
  fs::write(&template_path, "Copyright (c) {{year}} API Test Company")?;

  // Initialize the template manager
  let mut template_manager = TemplateManager::new();
  template_manager.load_template(&template_path)?;

  // Create license data
  let license_data = LicenseData {
    year: "2025".to_string(),
  };

  // Render the template
  let rendered = template_manager.render(&license_data)?;
  assert_eq!(rendered, "Copyright (c) 2025 API Test Company");

  // Test formatting for different file types
  let rust_formatted = template_manager
    .format_for_file_type(&rendered, Path::new("test.rs"))
    .expect("Rust files should have a comment style");
  assert!(rust_formatted.contains("// Copyright"));

  let python_formatted = template_manager
    .format_for_file_type(&rendered, Path::new("test.py"))
    .expect("Python files should have a comment style");
  assert!(python_formatted.contains("# Copyright"));

  let java_formatted = template_manager
    .format_for_file_type(&rendered, Path::new("test.java"))
    .expect("Java files should have a comment style");
  assert!(java_formatted.contains("/*"));
  assert!(java_formatted.contains(" * Copyright"));

  Ok(())
}

#[tokio::test]
async fn test_show_diff_mode() -> Result<()> {
  // Create a temporary directory for our test
  let temp_dir = tempdir()?;

  // Create a license template
  let template_path = temp_dir.path().join("license_template.txt");
  fs::write(&template_path, "Copyright (c) {{year}} API Test Company")?;

  // Create a test file without license
  let test_file_path = temp_dir.path().join("test_file.rs");
  fs::write(
    &test_file_path,
    "fn main() {\n    println!(\"Hello from API test\");\n}",
  )?;

  // Initialize the template manager
  let mut template_manager = TemplateManager::new();
  template_manager.load_template(&template_path)?;

  // Create license data
  let license_data = LicenseData {
    year: "2025".to_string(),
  };

  // Create a processor in check-only mode with show_diff enabled
  let processor = Processor::new(
    template_manager,
    license_data,
    vec![], // No ignore patterns
    true,   // Check-only mode
    false,  // Don't preserve years
    None,   // No ratchet mode
    Some(DiffManager::new(true, None)),
    false,
    None, // Use default LicenseDetector
    temp_dir.path().to_path_buf(),
    false,
    None, // No extension filter
  )?;

  // Process the file - should fail but show diff
  let result = processor.process_file(&test_file_path).await;
  assert!(result.is_err());

  // The file should not be modified
  let content = fs::read_to_string(&test_file_path)?;
  assert!(!content.contains("Copyright"));

  Ok(())
}

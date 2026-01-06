use std::fs;
use std::path::Path;

use anyhow::Result;
use edlicense::templates::{LicenseData, TemplateManager};
use tempfile::tempdir;

#[test]
fn test_template_loading() -> Result<()> {
  let temp_dir = tempdir()?;
  let template_path = temp_dir.path().join("test_template.txt");

  // Create a test template
  fs::write(&template_path, "Copyright (c) {{year}} Test Company")?;

  let mut template_manager = TemplateManager::new();
  template_manager.load_template(&template_path)?;

  // Render the template
  let license_data = LicenseData {
    year: "2025".to_string(),
  };

  let rendered = template_manager.render(&license_data)?;
  assert_eq!(rendered, "Copyright (c) 2025 Test Company");

  Ok(())
}

#[test]
fn test_comment_formatting() -> Result<()> {
  let temp_dir = tempdir()?;
  let template_path = temp_dir.path().join("test_template.txt");

  // Create a test template
  fs::write(
    &template_path,
    "Copyright (c) {{year}} Test Company\nAll rights reserved.",
  )?;

  let mut template_manager = TemplateManager::new();
  template_manager.load_template(&template_path)?;

  // Render the template
  let license_data = LicenseData {
    year: "2025".to_string(),
  };

  let rendered = template_manager.render(&license_data)?;

  // Test formatting for different file types
  let rust_file_path = Path::new("test.rs");
  let rust_formatted = template_manager.format_for_file_type(&rendered, rust_file_path);
  assert!(rust_formatted.contains("// Copyright"));
  assert!(rust_formatted.contains("// All rights"));

  let python_file_path = Path::new("test.py");
  let python_formatted = template_manager.format_for_file_type(&rendered, python_file_path);
  assert!(python_formatted.contains("# Copyright"));
  assert!(python_formatted.contains("# All rights"));

  let java_file_path = Path::new("test.java");
  let java_formatted = template_manager.format_for_file_type(&rendered, java_file_path);
  assert!(java_formatted.contains("/*"));
  assert!(java_formatted.contains(" * Copyright"));
  assert!(java_formatted.contains(" * All rights"));
  assert!(java_formatted.contains(" */"));

  let js_file_path = Path::new("test.js");
  let js_formatted = template_manager.format_for_file_type(&rendered, js_file_path);
  assert!(js_formatted.contains("/**"));
  assert!(js_formatted.contains(" * Copyright"));
  assert!(js_formatted.contains(" * All rights"));
  assert!(js_formatted.contains(" */"));

  let html_file_path = Path::new("test.html");
  let html_formatted = template_manager.format_for_file_type(&rendered, html_file_path);
  assert!(html_formatted.contains("<!--"));
  assert!(html_formatted.contains(" Copyright"));
  assert!(html_formatted.contains(" All rights"));
  assert!(html_formatted.contains("-->"));

  Ok(())
}

#[test]
fn test_special_filenames() -> Result<()> {
  let temp_dir = tempdir()?;
  let template_path = temp_dir.path().join("test_template.txt");

  // Create a test template
  fs::write(&template_path, "Copyright (c) {{year}} Test Company")?;

  let mut template_manager = TemplateManager::new();
  template_manager.load_template(&template_path)?;

  // Render the template
  let license_data = LicenseData {
    year: "2025".to_string(),
  };

  let rendered = template_manager.render(&license_data)?;

  // Test special filenames
  let cmake_file_path = Path::new("CMakeLists.txt");
  let cmake_formatted = template_manager.format_for_file_type(&rendered, cmake_file_path);
  assert!(cmake_formatted.contains("# Copyright"));

  let dockerfile_path = Path::new("Dockerfile");
  let dockerfile_formatted = template_manager.format_for_file_type(&rendered, dockerfile_path);
  assert!(dockerfile_formatted.contains("# Copyright"));

  let custom_dockerfile_path = Path::new("custom.dockerfile");
  let custom_dockerfile_formatted = template_manager.format_for_file_type(&rendered, custom_dockerfile_path);
  assert!(custom_dockerfile_formatted.contains("# Copyright"));

  Ok(())
}

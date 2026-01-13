use std::fs;
use std::path::Path;

use anyhow::Result;
use edlicense::templates::{get_comment_style_for_file, LicenseData, TemplateManager};
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
  let rust_formatted = template_manager.format_for_file_type(&rendered, rust_file_path).unwrap();
  assert!(rust_formatted.contains("// Copyright"));
  assert!(rust_formatted.contains("// All rights"));

  let python_file_path = Path::new("test.py");
  let python_formatted = template_manager.format_for_file_type(&rendered, python_file_path).unwrap();
  assert!(python_formatted.contains("# Copyright"));
  assert!(python_formatted.contains("# All rights"));

  let java_file_path = Path::new("test.java");
  let java_formatted = template_manager.format_for_file_type(&rendered, java_file_path).unwrap();
  assert!(java_formatted.contains("/*"));
  assert!(java_formatted.contains(" * Copyright"));
  assert!(java_formatted.contains(" * All rights"));
  assert!(java_formatted.contains(" */"));

  let js_file_path = Path::new("test.js");
  let js_formatted = template_manager.format_for_file_type(&rendered, js_file_path).unwrap();
  assert!(js_formatted.contains("/**"));
  assert!(js_formatted.contains(" * Copyright"));
  assert!(js_formatted.contains(" * All rights"));
  assert!(js_formatted.contains(" */"));

  let html_file_path = Path::new("test.html");
  let html_formatted = template_manager.format_for_file_type(&rendered, html_file_path).unwrap();
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
  let cmake_formatted = template_manager.format_for_file_type(&rendered, cmake_file_path).unwrap();
  assert!(cmake_formatted.contains("# Copyright"));

  let dockerfile_path = Path::new("Dockerfile");
  let dockerfile_formatted = template_manager.format_for_file_type(&rendered, dockerfile_path).unwrap();
  assert!(dockerfile_formatted.contains("# Copyright"));

  let custom_dockerfile_path = Path::new("custom.dockerfile");
  let custom_dockerfile_formatted = template_manager.format_for_file_type(&rendered, custom_dockerfile_path).unwrap();
  assert!(custom_dockerfile_formatted.contains("# Copyright"));

  Ok(())
}

#[test]
fn test_unknown_extensions_return_none() -> Result<()> {
  // Binary image files
  assert!(get_comment_style_for_file(Path::new("image.png")).is_none());
  assert!(get_comment_style_for_file(Path::new("image.jpg")).is_none());
  assert!(get_comment_style_for_file(Path::new("image.jpeg")).is_none());
  assert!(get_comment_style_for_file(Path::new("image.gif")).is_none());
  assert!(get_comment_style_for_file(Path::new("image.bmp")).is_none());
  assert!(get_comment_style_for_file(Path::new("image.ico")).is_none());
  assert!(get_comment_style_for_file(Path::new("image.svg")).is_none());
  assert!(get_comment_style_for_file(Path::new("image.webp")).is_none());

  // Binary executable/library files
  assert!(get_comment_style_for_file(Path::new("program.exe")).is_none());
  assert!(get_comment_style_for_file(Path::new("library.dll")).is_none());
  assert!(get_comment_style_for_file(Path::new("library.so")).is_none());
  assert!(get_comment_style_for_file(Path::new("library.dylib")).is_none());
  assert!(get_comment_style_for_file(Path::new("object.o")).is_none());
  assert!(get_comment_style_for_file(Path::new("archive.a")).is_none());

  // Archive files
  assert!(get_comment_style_for_file(Path::new("archive.zip")).is_none());
  assert!(get_comment_style_for_file(Path::new("archive.tar")).is_none());
  assert!(get_comment_style_for_file(Path::new("archive.gz")).is_none());
  assert!(get_comment_style_for_file(Path::new("archive.rar")).is_none());

  // Document/media files
  assert!(get_comment_style_for_file(Path::new("document.pdf")).is_none());
  assert!(get_comment_style_for_file(Path::new("document.doc")).is_none());
  assert!(get_comment_style_for_file(Path::new("document.docx")).is_none());
  assert!(get_comment_style_for_file(Path::new("audio.mp3")).is_none());
  assert!(get_comment_style_for_file(Path::new("video.mp4")).is_none());

  // Generic binary files
  assert!(get_comment_style_for_file(Path::new("data.bin")).is_none());
  assert!(get_comment_style_for_file(Path::new("data.dat")).is_none());

  // Files with no extension
  assert!(get_comment_style_for_file(Path::new("NOEXTENSION")).is_none());

  Ok(())
}

#[test]
fn test_format_for_file_type_returns_none_for_unknown() -> Result<()> {
  let temp_dir = tempdir()?;
  let template_path = temp_dir.path().join("test_template.txt");

  // Create a test template
  fs::write(&template_path, "Copyright (c) {{year}} Test Company")?;

  let mut template_manager = TemplateManager::new();
  template_manager.load_template(&template_path)?;

  let license_data = LicenseData {
    year: "2025".to_string(),
  };

  let rendered = template_manager.render(&license_data)?;

  // These should all return None for unknown extensions
  assert!(template_manager.format_for_file_type(&rendered, Path::new("image.png")).is_none());
  assert!(template_manager.format_for_file_type(&rendered, Path::new("binary.exe")).is_none());
  assert!(template_manager.format_for_file_type(&rendered, Path::new("archive.zip")).is_none());
  assert!(template_manager.format_for_file_type(&rendered, Path::new("document.pdf")).is_none());

  // These should return Some for known extensions
  assert!(template_manager.format_for_file_type(&rendered, Path::new("main.rs")).is_some());
  assert!(template_manager.format_for_file_type(&rendered, Path::new("script.py")).is_some());
  assert!(template_manager.format_for_file_type(&rendered, Path::new("app.js")).is_some());

  Ok(())
}

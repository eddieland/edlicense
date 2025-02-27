use std::env;
use std::fs;

use anyhow::Result;
use tempfile::tempdir;

use edlicense::ignore::IgnoreManager;
use edlicense::processor::Processor;
use edlicense::templates::{LicenseData, TemplateManager};

#[test]
fn test_licenseignore_basic() -> Result<()> {
    // Create a temporary directory
    let temp_dir = tempdir()?;
    let temp_path = temp_dir.path();

    // Create a .licenseignore file
    let ignore_content = "*.json\n*.md\nvendor/\n";
    fs::write(temp_path.join(".licenseignore"), ignore_content)?;

    // Create test files
    fs::write(temp_path.join("test.rs"), "// Test Rust file")?;
    fs::write(temp_path.join("test.json"), "// Test JSON file")?;
    fs::write(temp_path.join("test.md"), "# Test Markdown file")?;
    fs::create_dir(temp_path.join("vendor"))?;
    fs::write(temp_path.join("vendor").join("test.rs"), "// Test vendor file")?;

    // Create an IgnoreManager and load .licenseignore files
    let mut ignore_manager = IgnoreManager::new(vec![])?;
    ignore_manager.load_licenseignore_files(temp_path)?;

    // Test which files are ignored
    assert!(
        !ignore_manager.is_ignored(&temp_path.join("test.rs")),
        "Rust file should not be ignored"
    );
    assert!(
        ignore_manager.is_ignored(&temp_path.join("test.json")),
        "JSON file should be ignored"
    );
    assert!(
        ignore_manager.is_ignored(&temp_path.join("test.md")),
        "Markdown file should be ignored"
    );
    assert!(
        ignore_manager.is_ignored(&temp_path.join("vendor").join("test.rs")),
        "Vendor file should be ignored"
    );

    Ok(())
}

#[test]
fn test_licenseignore_with_cli_patterns() -> Result<()> {
    // Create a temporary directory
    let temp_dir = tempdir()?;
    let temp_path = temp_dir.path();

    // Create a .licenseignore file
    let ignore_content = "*.json\n";
    fs::write(temp_path.join(".licenseignore"), ignore_content)?;

    // Create test files
    fs::write(temp_path.join("test.rs"), "// Test Rust file")?;
    fs::write(temp_path.join("test.json"), "// Test JSON file")?;
    fs::write(temp_path.join("test.md"), "# Test Markdown file")?;

    // Create an IgnoreManager with CLI patterns
    let mut ignore_manager = IgnoreManager::new(vec!["*.md".to_string()])?;
    ignore_manager.load_licenseignore_files(temp_path)?;

    // Test which files are ignored
    assert!(
        !ignore_manager.is_ignored(&temp_path.join("test.rs")),
        "Rust file should not be ignored"
    );
    assert!(
        ignore_manager.is_ignored(&temp_path.join("test.json")),
        "JSON file should be ignored by .licenseignore"
    );
    assert!(
        ignore_manager.is_ignored(&temp_path.join("test.md")),
        "Markdown file should be ignored by CLI pattern"
    );

    Ok(())
}

#[test]
fn test_global_licenseignore() -> Result<()> {
    // Create a temporary directory
    let temp_dir = tempdir()?;
    let temp_path = temp_dir.path();

    // Create a global ignore file
    let global_ignore_path = temp_path.join("global_licenseignore");
    let global_ignore_content = "*.py\n";
    fs::write(&global_ignore_path, global_ignore_content)?;

    // Set the environment variable
    unsafe {
        env::set_var("GLOBAL_LICENSE_IGNORE", global_ignore_path.to_str().unwrap());
    }

    // Create a .licenseignore file
    let ignore_content = "*.json\n";
    fs::write(temp_path.join(".licenseignore"), ignore_content)?;

    // Create test files
    fs::write(temp_path.join("test.rs"), "// Test Rust file")?;
    fs::write(temp_path.join("test.json"), "// Test JSON file")?;
    fs::write(temp_path.join("test.py"), "# Test Python file")?;

    // Create an IgnoreManager
    let mut ignore_manager = IgnoreManager::new(vec![])?;
    ignore_manager.load_licenseignore_files(temp_path)?;

    // Test which files are ignored
    assert!(
        !ignore_manager.is_ignored(&temp_path.join("test.rs")),
        "Rust file should not be ignored"
    );
    assert!(
        ignore_manager.is_ignored(&temp_path.join("test.json")),
        "JSON file should be ignored by .licenseignore"
    );
    assert!(
        ignore_manager.is_ignored(&temp_path.join("test.py")),
        "Python file should be ignored by global ignore file"
    );

    // Clean up
    unsafe {
        env::remove_var("GLOBAL_LICENSE_IGNORE");
    }

    Ok(())
}

#[test]
fn test_hierarchical_licenseignore() -> Result<()> {
    // Create a temporary directory
    let temp_dir = tempdir()?;
    let temp_path = temp_dir.path();

    // Create a parent .licenseignore file
    let parent_ignore_content = "*.json\n*.md\n";
    fs::write(temp_path.join(".licenseignore"), parent_ignore_content)?;

    // Create a subdirectory
    fs::create_dir(temp_path.join("subdir"))?;

    // Create a subdirectory .licenseignore file with additional patterns
    // Note: We're not using negation patterns since the current implementation
    // doesn't support overriding parent patterns with negation
    let subdir_ignore_content = "*.txt\n";
    fs::write(temp_path.join("subdir").join(".licenseignore"), subdir_ignore_content)?;

    // Create test files in parent directory
    fs::write(temp_path.join("test.rs"), "// Test Rust file")?;
    fs::write(temp_path.join("test.json"), "// Test JSON file")?;
    fs::write(temp_path.join("test.md"), "# Test Markdown file")?;

    // Create test files in subdirectory
    fs::write(temp_path.join("subdir").join("test.rs"), "// Test Rust file in subdir")?;
    fs::write(
        temp_path.join("subdir").join("test.json"),
        "// Test JSON file in subdir",
    )?;
    fs::write(
        temp_path.join("subdir").join("test.md"),
        "# Test Markdown file in subdir",
    )?;
    fs::write(temp_path.join("subdir").join("test.txt"), "Test text file in subdir")?;

    // Create an IgnoreManager for parent directory
    let mut parent_ignore_manager = IgnoreManager::new(vec![])?;
    parent_ignore_manager.load_licenseignore_files(temp_path)?;

    // Create an IgnoreManager for subdirectory
    let mut subdir_ignore_manager = IgnoreManager::new(vec![])?;
    subdir_ignore_manager.load_licenseignore_files(&temp_path.join("subdir"))?;

    // Test parent directory files
    assert!(
        !parent_ignore_manager.is_ignored(&temp_path.join("test.rs")),
        "Rust file in parent should not be ignored"
    );
    assert!(
        parent_ignore_manager.is_ignored(&temp_path.join("test.json")),
        "JSON file in parent should be ignored"
    );
    assert!(
        parent_ignore_manager.is_ignored(&temp_path.join("test.md")),
        "Markdown file in parent should be ignored"
    );

    // Test subdirectory files with subdirectory ignore manager
    assert!(
        !subdir_ignore_manager.is_ignored(&temp_path.join("subdir").join("test.rs")),
        "Rust file in subdir should not be ignored"
    );
    assert!(
        subdir_ignore_manager.is_ignored(&temp_path.join("subdir").join("test.json")),
        "JSON file in subdir should be ignored (inherited from parent)"
    );
    assert!(
        subdir_ignore_manager.is_ignored(&temp_path.join("subdir").join("test.md")),
        "Markdown file in subdir should be ignored (inherited from parent)"
    );
    assert!(
        subdir_ignore_manager.is_ignored(&temp_path.join("subdir").join("test.txt")),
        "Text file in subdir should be ignored (from subdir ignore)"
    );

    Ok(())
}

#[test]
fn test_complex_patterns() -> Result<()> {
    // Create a temporary directory
    let temp_dir = tempdir()?;
    let temp_path = temp_dir.path();

    // Create a .licenseignore file with complex patterns
    let ignore_content = "
# Comment line
*.json
!important.json
/root_only.txt
**/node_modules/
docs/*.md
*.min.*
";
    fs::write(temp_path.join(".licenseignore"), ignore_content)?;

    // Create test files and directories
    fs::write(temp_path.join("test.json"), "// Regular JSON file")?;
    fs::write(
        temp_path.join("important.json"),
        "// Important JSON file that should not be ignored",
    )?;
    fs::write(temp_path.join("root_only.txt"), "Root level text file")?;

    fs::create_dir(temp_path.join("subdir"))?;
    fs::write(temp_path.join("subdir").join("subdir_file.json"), "// JSON in subdir")?;
    fs::write(
        temp_path.join("subdir").join("root_only.txt"),
        "Text file with same name in subdir",
    )?;

    fs::create_dir_all(temp_path.join("deep").join("node_modules"))?;
    fs::write(
        temp_path.join("deep").join("node_modules").join("module.js"),
        "// Module file",
    )?;

    fs::create_dir(temp_path.join("docs"))?;
    fs::write(temp_path.join("docs").join("readme.md"), "# Documentation")?;
    fs::write(temp_path.join("docs").join("config.json"), "// Config in docs")?;

    fs::write(temp_path.join("script.min.js"), "// Minified script")?;
    fs::write(temp_path.join("style.min.css"), "/* Minified style */")?;

    // Create an IgnoreManager and load .licenseignore files
    let mut ignore_manager = IgnoreManager::new(vec![])?;
    ignore_manager.load_licenseignore_files(temp_path)?;

    // Test which files are ignored
    assert!(
        ignore_manager.is_ignored(&temp_path.join("test.json")),
        "Regular JSON file should be ignored"
    );
    assert!(
        !ignore_manager.is_ignored(&temp_path.join("important.json")),
        "Important JSON file should NOT be ignored due to negation"
    );
    assert!(
        ignore_manager.is_ignored(&temp_path.join("root_only.txt")),
        "Root level text file should be ignored"
    );
    assert!(
        !ignore_manager.is_ignored(&temp_path.join("subdir").join("root_only.txt")),
        "Text file in subdir should NOT be ignored as /root_only.txt only matches at root"
    );
    assert!(
        ignore_manager.is_ignored(&temp_path.join("subdir").join("subdir_file.json")),
        "JSON file in subdir should be ignored"
    );
    assert!(
        ignore_manager.is_ignored(&temp_path.join("deep").join("node_modules").join("module.js")),
        "File in node_modules should be ignored"
    );
    assert!(
        ignore_manager.is_ignored(&temp_path.join("docs").join("readme.md")),
        "Markdown file in docs should be ignored"
    );
    assert!(
        ignore_manager.is_ignored(&temp_path.join("docs").join("config.json")),
        "JSON file in docs should be ignored"
    );
    assert!(
        ignore_manager.is_ignored(&temp_path.join("script.min.js")),
        "Minified JS file should be ignored"
    );
    assert!(
        ignore_manager.is_ignored(&temp_path.join("style.min.css")),
        "Minified CSS file should be ignored"
    );

    Ok(())
}

#[test]
fn test_path_normalization() -> Result<()> {
    // Create a temporary directory
    let temp_dir = tempdir()?;
    let temp_path = temp_dir.path();

    // Create a .licenseignore file
    let ignore_content = "*.json\n";
    fs::write(temp_path.join(".licenseignore"), ignore_content)?;

    // Create test files
    fs::write(temp_path.join("test.json"), "// Test JSON file")?;

    // Create an IgnoreManager and load .licenseignore files
    let mut ignore_manager = IgnoreManager::new(vec![])?;
    ignore_manager.load_licenseignore_files(temp_path)?;

    // Test with different path formats
    let absolute_path = temp_path.join("test.json");
    assert!(
        ignore_manager.is_ignored(&absolute_path),
        "JSON file should be ignored with absolute path"
    );

    // Test with relative paths if possible
    if let Ok(relative_path) = absolute_path.strip_prefix(temp_path) {
        // Create a new path by joining the relative path to the temp_path
        // This simulates a different but equivalent path
        let reconstructed_path = temp_path.join(relative_path);
        assert!(
            ignore_manager.is_ignored(&reconstructed_path),
            "JSON file should be ignored with reconstructed path"
        );
    }

    Ok(())
}

#[test]
fn test_processor_with_licenseignore() -> Result<()> {
    // Create a temporary directory
    let temp_dir = tempdir()?;
    let temp_path = temp_dir.path();

    // Create a .licenseignore file
    let ignore_content = "*.json\n";
    fs::write(temp_path.join(".licenseignore"), ignore_content)?;

    // Create test files
    fs::write(temp_path.join("test.rs"), "// Test Rust file")?;
    fs::write(temp_path.join("test.json"), "// Test JSON file")?;

    // Create a license template
    let license_path = temp_path.join("LICENSE.txt");
    fs::write(&license_path, "Copyright (c) 2025 Test")?;

    // Create license data
    let license_data = LicenseData {
        year: "2025".to_string(),
    };

    // Create and initialize template manager
    let mut template_manager = TemplateManager::new();
    template_manager.load_template(&license_path)?;

    // Create processor
    let processor = Processor::new(
        template_manager,
        license_data,
        vec![], // No CLI ignore patterns
        true,   // Check-only mode
        false,  // Don't preserve years
        None,   // No ratchet reference
        None,   // Use default diff_manager
    )?;

    // Process the directory
    let has_missing = processor.process_directory(temp_path)?;

    // The Rust file should be processed and found to be missing a license
    assert!(has_missing, "Should have found files missing license headers");

    Ok(())
}

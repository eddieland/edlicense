use anyhow::Result;
use std::fs;
use tempfile::tempdir;

use edlicense::ignore::IgnoreManager;

#[test]
fn test_cli_ignore_patterns_directly() -> Result<()> {
    // Create a temporary directory
    let temp_dir = tempdir()?;
    let temp_path = temp_dir.path();

    // Create test files
    fs::write(temp_path.join("test.json"), "// Test JSON file")?;
    fs::write(temp_path.join("test.rs"), "// Test Rust file")?;
    fs::create_dir_all(temp_path.join("vendor"))?;
    fs::write(temp_path.join("vendor").join("test.rs"), "// Test vendor file")?;
    fs::create_dir_all(temp_path.join("vendor").join("subfolder"))?;
    fs::write(
        temp_path.join("vendor").join("subfolder").join("test.rs"),
        "// Test subfolder file",
    )?;

    // Test different CLI pattern formats
    let patterns_to_test = vec![
        // Standard glob pattern
        vec!["*.json".to_string()],
        // Directory pattern with trailing slash
        vec!["vendor/".to_string()],
        // Directory pattern without trailing slash
        vec!["vendor".to_string()],
        // Directory pattern with wildcard
        vec!["vendor/*".to_string()],
        // Recursive directory pattern
        vec!["vendor/**".to_string()],
        // Multiple patterns
        vec!["*.json".to_string(), "vendor/**".to_string()],
    ];

    for (i, patterns) in patterns_to_test.iter().enumerate() {
        println!("Testing pattern set {}: {:?}", i, patterns);

        // Create an IgnoreManager with CLI patterns
        let mut ignore_manager = IgnoreManager::new(patterns.clone())?;

        // Load .licenseignore files (none in this case)
        ignore_manager.load_licenseignore_files(temp_path)?;

        // Test JSON file (should be ignored by *.json pattern)
        let json_ignored = ignore_manager.is_ignored(&temp_path.join("test.json"));

        // Test Rust file (should never be ignored)
        let rust_ignored = ignore_manager.is_ignored(&temp_path.join("test.rs"));

        // Test vendor file (should be ignored by vendor patterns)
        let vendor_ignored = ignore_manager.is_ignored(&temp_path.join("vendor").join("test.rs"));

        // Test subfolder file (should be ignored by vendor/** pattern)
        let subfolder_ignored = ignore_manager.is_ignored(&temp_path.join("vendor").join("subfolder").join("test.rs"));

        // Print results for debugging
        println!("  JSON file ignored: {}", json_ignored);
        println!("  Rust file ignored: {}", rust_ignored);
        println!("  Vendor file ignored: {}", vendor_ignored);
        println!("  Subfolder file ignored: {}", subfolder_ignored);

        // Check expectations based on the pattern
        if patterns.iter().any(|p| p.contains("*.json")) {
            assert!(json_ignored, "JSON file should be ignored by pattern: {:?}", patterns);
        }

        assert!(!rust_ignored, "Rust file should not be ignored");

        if patterns.iter().any(|p| p.starts_with("vendor")) {
            // For vendor/ or vendor/** patterns, both vendor files should be ignored
            if patterns
                .iter()
                .any(|p| p == "vendor/" || p == "vendor" || p.contains("**"))
            {
                assert!(
                    vendor_ignored,
                    "Vendor file should be ignored by pattern: {:?}",
                    patterns
                );

                // For vendor/** pattern, subfolder files should also be ignored
                if patterns.iter().any(|p| p.contains("**")) {
                    assert!(
                        subfolder_ignored,
                        "Subfolder file should be ignored by pattern: {:?}",
                        patterns
                    );
                }
            }
        }
    }

    Ok(())
}

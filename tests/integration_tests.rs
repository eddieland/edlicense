use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;
use anyhow::Result;

// Helper function to create a test environment
fn setup_test_environment() -> Result<tempfile::TempDir> {
    let temp_dir = tempdir()?;
    
    // Create a license template
    let template_content = "Copyright (c) {{Year}} Test Company\nAll rights reserved.";
    fs::write(temp_dir.path().join("license_template.txt"), template_content)?;
    
    // Create a test directory structure
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir)?;
    
    // Create some test files
    fs::write(src_dir.join("main.rs"), "fn main() {\n    println!(\"Hello, world!\");\n}")?;
    fs::write(src_dir.join("lib.rs"), "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}")?;
    
    // Create a file with an existing license
    let licensed_content = "// Copyright (c) 2024 Test Company\n// All rights reserved.\n\npub fn subtract(a: i32, b: i32) -> i32 {\n    a - b\n}";
    fs::write(src_dir.join("licensed.rs"), licensed_content)?;
    
    // Create a file with a shebang
    let shebang_content = "#!/usr/bin/env python3\n\ndef main():\n    print('Hello, world!')";
    fs::write(temp_dir.path().join("script.py"), shebang_content)?;
    
    // Create a directory to be ignored
    let vendor_dir = temp_dir.path().join("vendor");
    fs::create_dir_all(&vendor_dir)?;
    fs::write(vendor_dir.join("external.rs"), "fn external() {}")?;
    
    Ok(temp_dir)
}

// Helper function to run edlicense with the given arguments
fn run_edlicense(args: &[&str], current_dir: &Path) -> Result<(i32, String, String)> {
    let output = Command::new("cargo")
        .arg("run")
        .arg("--")
        .args(args)
        .current_dir(current_dir)
        .output()?;
    
    let status = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    
    Ok((status, stdout, stderr))
}

#[test]
#[ignore] // This test requires the binary to be built, so we'll ignore it by default
fn test_add_license() -> Result<()> {
    let temp_dir = setup_test_environment()?;
    
    // Run edlicense to add licenses
    let args = &[
        "--license-file", 
        "license_template.txt",
        "--verbose",
        "src"
    ];
    
    let (status, _stdout, stderr) = run_edlicense(args, temp_dir.path())?;
    
    // Check that the command succeeded
    assert_eq!(status, 0, "Command failed with stderr: {}", stderr);
    
    // Check that the files were processed
    assert!(stderr.contains("Processing file: "));
    
    // Check that the licenses were added
    let main_content = fs::read_to_string(temp_dir.path().join("src/main.rs"))?;
    assert!(main_content.contains("// Copyright (c)"));
    assert!(main_content.contains("Test Company"));
    
    let lib_content = fs::read_to_string(temp_dir.path().join("src/lib.rs"))?;
    assert!(lib_content.contains("// Copyright (c)"));
    
    // Check that the year was updated in the licensed file
    let licensed_content = fs::read_to_string(temp_dir.path().join("src/licensed.rs"))?;
    assert!(!licensed_content.contains("2024"));
    assert!(licensed_content.contains("2025"));
    
    // Check that the shebang was preserved
    let script_content = fs::read_to_string(temp_dir.path().join("script.py"))?;
    assert!(script_content.starts_with("#!/usr/bin/env python3"));
    assert!(script_content.contains("# Copyright (c)"));
    
    Ok(())
}

#[test]
#[ignore] // This test requires the binary to be built, so we'll ignore it by default
fn test_check_only_mode() -> Result<()> {
    let temp_dir = setup_test_environment()?;
    
    // Run edlicense in check-only mode
    let args = &[
        "--check",
        "--license-file", 
        "license_template.txt",
        "src"
    ];
    
    let (status, _stdout, stderr) = run_edlicense(args, temp_dir.path())?;
    
    // Check that the command failed (because some files don't have licenses)
    assert_ne!(status, 0, "Command should have failed but succeeded");
    assert!(stderr.contains("Error: Some files are missing license headers"));
    
    // Check that the files were not modified
    let main_content = fs::read_to_string(temp_dir.path().join("src/main.rs"))?;
    assert!(!main_content.contains("Copyright"));
    
    // Now add licenses to all files
    let add_args = &[
        "--license-file", 
        "license_template.txt",
        "src"
    ];
    
    let (add_status, _, _) = run_edlicense(add_args, temp_dir.path())?;
    assert_eq!(add_status, 0);
    
    // Run check-only mode again
    let (check_status, _, check_stderr) = run_edlicense(args, temp_dir.path())?;
    
    // This time it should succeed
    assert_eq!(check_status, 0, "Check command failed with stderr: {}", check_stderr);
    
    Ok(())
}

#[test]
#[ignore] // This test requires the binary to be built, so we'll ignore it by default
fn test_ignore_patterns() -> Result<()> {
    let temp_dir = setup_test_environment()?;
    
    // Run edlicense with ignore patterns
    let args = &[
        "--license-file", 
        "license_template.txt",
        "--ignore", "vendor/**",
        "--verbose",
        "."
    ];
    
    let (status, _stdout, stderr) = run_edlicense(args, temp_dir.path())?;
    
    // Check that the command succeeded
    assert_eq!(status, 0, "Command failed with stderr: {}", stderr);
    
    // Check that the vendor directory was ignored
    assert!(stderr.contains("Skipping: ") && stderr.contains("vendor"));
    
    // Check that the vendor file was not modified
    let vendor_content = fs::read_to_string(temp_dir.path().join("vendor/external.rs"))?;
    assert!(!vendor_content.contains("Copyright"));
    
    Ok(())
}

#[test]
#[ignore] // This test requires the binary to be built, so we'll ignore it by default
fn test_preserve_years() -> Result<()> {
    let temp_dir = setup_test_environment()?;
    
    // Run edlicense with preserve-years flag
    let args = &[
        "--license-file", 
        "license_template.txt",
        "--preserve-years",
        "src"
    ];
    
    let (status, _stdout, stderr) = run_edlicense(args, temp_dir.path())?;
    
    // Check that the command succeeded
    assert_eq!(status, 0, "Command failed with stderr: {}", stderr);
    
    // Check that the year was preserved in the licensed file
    let licensed_content = fs::read_to_string(temp_dir.path().join("src/licensed.rs"))?;
    assert!(licensed_content.contains("2024"));
    assert!(!licensed_content.contains("2025"));
    
    Ok(())
}

#[test]
#[ignore] // This test requires the binary to be built, so we'll ignore it by default
fn test_custom_year() -> Result<()> {
    let temp_dir = setup_test_environment()?;
    
    // Run edlicense with a custom year
    let args = &[
        "--license-file", 
        "license_template.txt",
        "--year", "2030",
        "src"
    ];
    
    let (status, _stdout, stderr) = run_edlicense(args, temp_dir.path())?;
    
    // Check that the command succeeded
    assert_eq!(status, 0, "Command failed with stderr: {}", stderr);
    
    // Check that the custom year was used
    let main_content = fs::read_to_string(temp_dir.path().join("src/main.rs"))?;
    assert!(main_content.contains("2030"));
    
    // Check that the year was updated in the licensed file
    let licensed_content = fs::read_to_string(temp_dir.path().join("src/licensed.rs"))?;
    assert!(!licensed_content.contains("2024"));
    assert!(licensed_content.contains("2030"));
    
    Ok(())
}
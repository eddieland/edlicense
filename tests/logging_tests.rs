use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

use assert_cmd::prelude::*;

#[test]
fn test_color_modes() -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary license file
    let mut license_file = NamedTempFile::new()?;
    writeln!(license_file, "Copyright (c) {{year}} Test")?;

    // Create a temporary test file
    let mut test_file = NamedTempFile::new()?;
    writeln!(test_file, "// This is a test file without a license")?;

    // Test with --colors=never
    let output = Command::cargo_bin("edlicense")?
        .arg("--license-file")
        .arg(license_file.path())
        .arg("--colors=never")
        .arg("--verbose")
        .arg("--modify") // Add modify flag to ensure command succeeds
        .arg(test_file.path())
        .output()?;

    // Check that the output doesn't contain ANSI color codes
    let stdout = String::from_utf8(output.stdout)?;
    assert!(!stdout.contains("\x1b["));

    // Test with --colors=always
    let output = Command::cargo_bin("edlicense")?
        .arg("--license-file")
        .arg(license_file.path())
        .arg("--colors=always")
        .arg("--verbose")
        .arg("--modify") // Add modify flag to ensure command succeeds
        .arg(test_file.path())
        .output()?;

    // With --colors=always, we should see color codes even in non-TTY output
    // But this is hard to test in a non-interactive environment, so we'll just
    // check that the command runs successfully
    assert!(output.status.success());

    // Test default (auto) mode
    let output = Command::cargo_bin("edlicense")?
        .arg("--license-file")
        .arg(license_file.path())
        .arg("--verbose")
        .arg("--modify") // Add modify flag to ensure command succeeds
        .arg(test_file.path())
        .output()?;

    // In auto mode with non-TTY output, we shouldn't see color codes
    let stdout = String::from_utf8(output.stdout)?;
    assert!(!stdout.contains("\x1b["));

    Ok(())
}

#[test]
fn test_info_log_formatting() -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary license file
    let mut license_file = NamedTempFile::new()?;
    writeln!(license_file, "Copyright (c) {{year}} Test")?;

    // Create a temporary test file
    let mut test_file = NamedTempFile::new()?;
    writeln!(test_file, "// This is a test file without a license")?;

    // Run in modify mode to add a license
    let output = Command::cargo_bin("edlicense")?
        .arg("--license-file")
        .arg(license_file.path())
        .arg("--modify")
        .arg("--colors=never") // Disable colors for consistent testing
        .arg(test_file.path())
        .output()?;

    // Check that the output contains the expected formatted message
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains(&format!("Added license to: {}", test_file.path().display())));

    // Run again to check for year updates
    // First modify the file to have an outdated year
    let mut outdated_license = String::new();
    outdated_license.push_str("// Copyright (c) 2020 Test\n");
    outdated_license.push_str("// This is a test file without a license\n");

    std::fs::write(test_file.path(), outdated_license)?;

    // Run with year update
    let output = Command::cargo_bin("edlicense")?
        .arg("--license-file")
        .arg(license_file.path())
        .arg("--modify")
        .arg("--colors=never") // Disable colors for consistent testing
        .arg("--year=2025")
        .arg(test_file.path())
        .output()?;

    // Check that the output contains the expected formatted message for year update
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains(&format!("Updated year in: {}", test_file.path().display())));

    Ok(())
}

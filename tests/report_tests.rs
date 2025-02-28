//! # Report generation tests
//!
//! This module tests the report generation functionality.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use edlicense::report::{FileAction, FileReport, ProcessingSummary, ReportFormat, ReportGenerator};

#[test]
fn test_html_report_generation() {
    // Create test data
    let temp_dir = tempfile::tempdir().unwrap();
    let output_path = temp_dir.path().join("report.html");

    let file_reports = vec![
        FileReport {
            path: PathBuf::from("src/main.rs"),
            has_license: true,
            action_taken: Some(FileAction::NoActionNeeded),
            ignored: false,
            ignored_reason: None,
        },
        FileReport {
            path: PathBuf::from("src/lib.rs"),
            has_license: true,
            action_taken: Some(FileAction::YearUpdated),
            ignored: false,
            ignored_reason: None,
        },
        FileReport {
            path: PathBuf::from("tests/test.rs"),
            has_license: false,
            action_taken: Some(FileAction::Added),
            ignored: false,
            ignored_reason: None,
        },
        FileReport {
            path: PathBuf::from("target/debug/build.rs"),
            has_license: false,
            action_taken: Some(FileAction::Skipped),
            ignored: true,
            ignored_reason: Some("Matches ignore pattern".to_string()),
        },
    ];

    let summary = ProcessingSummary::from_reports(&file_reports, Duration::from_secs(1));

    // Generate report
    let report_generator = ReportGenerator::new(ReportFormat::Html, output_path.clone());
    let result = report_generator.generate(&file_reports, &summary);

    // Verify report was generated
    assert!(result.is_ok());
    assert!(output_path.exists());

    // Read report content
    let content = fs::read_to_string(&output_path).unwrap();

    // Verify content contains expected elements
    assert!(content.contains("<html"));
    assert!(content.contains("</html>"));
    assert!(content.contains("src/main.rs"));
    assert!(content.contains("src/lib.rs"));
    assert!(content.contains("tests/test.rs"));
    assert!(content.contains("target/debug/build.rs"));
    assert!(content.contains("Has license"));
    assert!(content.contains("No license"));
    assert!(content.contains("Added license"));
    assert!(content.contains("Updated year"));
    assert!(content.contains("Matches ignore pattern"));
}

#[test]
fn test_json_report_generation() {
    use serde_json::{Value, from_str};

    // Create test data
    let temp_dir = tempfile::tempdir().unwrap();
    let output_path = temp_dir.path().join("report.json");

    let file_reports = vec![
        FileReport {
            path: PathBuf::from("src/main.rs"),
            has_license: true,
            action_taken: Some(FileAction::NoActionNeeded),
            ignored: false,
            ignored_reason: None,
        },
        FileReport {
            path: PathBuf::from("src/lib.rs"),
            has_license: false,
            action_taken: Some(FileAction::Added),
            ignored: false,
            ignored_reason: None,
        },
    ];

    let summary = ProcessingSummary::from_reports(&file_reports, Duration::from_secs(1));

    // Generate report
    let report_generator = ReportGenerator::new(ReportFormat::Json, output_path.clone());
    let result = report_generator.generate(&file_reports, &summary);

    // Verify report was generated
    assert!(result.is_ok());
    assert!(output_path.exists());

    // Read report content
    let content = fs::read_to_string(&output_path).unwrap();

    // Parse the JSON content
    let json: Value = from_str(&content).expect("Should be valid JSON");

    // Verify JSON structure
    assert!(json.is_object(), "JSON should be an object");
    assert!(json.get("summary").is_some(), "JSON should have a summary field");
    assert!(json.get("files").is_some(), "JSON should have a files field");

    // Verify summary data
    let summary = json.get("summary").unwrap();
    assert_eq!(summary.get("total_files").unwrap().as_u64(), Some(2));
    assert_eq!(summary.get("files_with_license").unwrap().as_u64(), Some(1));
    assert_eq!(summary.get("files_without_license").unwrap().as_u64(), Some(1));

    // Verify files data
    let files = json.get("files").unwrap().as_array().unwrap();
    assert_eq!(files.len(), 2);

    // Find the main.rs file entry
    let main_rs = files
        .iter()
        .find(|f| f.get("path").unwrap().as_str().unwrap().contains("src/main.rs"))
        .unwrap();

    // Verify main.rs data
    assert_eq!(main_rs.get("has_license").unwrap().as_bool(), Some(true));
    assert_eq!(main_rs.get("action").unwrap().as_str(), Some("none"));

    // Find the lib.rs file entry
    let lib_rs = files
        .iter()
        .find(|f| f.get("path").unwrap().as_str().unwrap().contains("src/lib.rs"))
        .unwrap();

    // Verify lib.rs data
    assert_eq!(lib_rs.get("has_license").unwrap().as_bool(), Some(false));
    assert_eq!(lib_rs.get("action").unwrap().as_str(), Some("added"));
}

#[test]
fn test_csv_report_generation() {
    // Create test data
    let temp_dir = tempfile::tempdir().unwrap();
    let output_path = temp_dir.path().join("report.csv");

    let file_reports = vec![
        FileReport {
            path: PathBuf::from("src/main.rs"),
            has_license: true,
            action_taken: Some(FileAction::NoActionNeeded),
            ignored: false,
            ignored_reason: None,
        },
        FileReport {
            path: PathBuf::from("src/lib.rs"),
            has_license: false,
            action_taken: Some(FileAction::Added),
            ignored: false,
            ignored_reason: None,
        },
    ];

    let summary = ProcessingSummary::from_reports(&file_reports, Duration::from_secs(1));

    // Generate report
    let report_generator = ReportGenerator::new(ReportFormat::Csv, output_path.clone());
    let result = report_generator.generate(&file_reports, &summary);

    // Verify report was generated
    assert!(result.is_ok());
    assert!(output_path.exists());

    // Read report content
    let content = fs::read_to_string(&output_path).unwrap();

    // Verify CSV format and content
    let lines: Vec<&str> = content.lines().collect();
    assert!(lines.len() >= 4); // Header, 2 data rows, and at least 1 summary row
    assert_eq!(lines[0], "file_path,has_license,action_taken,notes");
    assert!(lines.iter().any(|line| line.starts_with("src/main.rs")));
    assert!(lines.iter().any(|line| line.starts_with("src/lib.rs")));
    assert!(content.contains("true,None,"));
    assert!(content.contains("false,Added,"));
    assert!(content.contains("# Summary"));
}

#[test]
fn test_process_summary_calculation() {
    let file_reports = vec![
        // Files with license
        FileReport {
            path: PathBuf::from("file1.rs"),
            has_license: true,
            action_taken: Some(FileAction::NoActionNeeded),
            ignored: false,
            ignored_reason: None,
        },
        FileReport {
            path: PathBuf::from("file2.rs"),
            has_license: true,
            action_taken: Some(FileAction::YearUpdated),
            ignored: false,
            ignored_reason: None,
        },
        // Files without license
        FileReport {
            path: PathBuf::from("file3.rs"),
            has_license: false,
            action_taken: Some(FileAction::Added),
            ignored: false,
            ignored_reason: None,
        },
        // Ignored files
        FileReport {
            path: PathBuf::from("file4.rs"),
            has_license: false,
            action_taken: Some(FileAction::Skipped),
            ignored: true,
            ignored_reason: Some("Reason".to_string()),
        },
    ];

    let duration = Duration::from_secs(5);
    let summary = ProcessingSummary::from_reports(&file_reports, duration);

    assert_eq!(summary.total_files, 4);
    assert_eq!(summary.files_with_license, 2);
    assert_eq!(summary.files_without_license, 1);
    assert_eq!(summary.files_ignored, 1);
    assert_eq!(summary.licenses_added, 1);
    assert_eq!(summary.licenses_updated, 1);
    assert_eq!(summary.processing_time, duration);
}

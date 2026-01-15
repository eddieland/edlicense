//! # Report Module
//!
//! This module provides functionality for generating reports of license
//! processing in various formats (HTML, JSON, CSV).
//!
//! It captures information about each processed file, including its license
//! status and any actions taken, and can output this information in the
//! requested format.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};

/// Information about a processed file for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReport {
  /// Path to the file
  #[serde(with = "path_serialization")]
  pub path: PathBuf,
  /// Whether the file has a license header
  pub has_license: bool,
  /// Action taken on the file, if any
  pub action_taken: Option<FileAction>,
  /// Whether the file was ignored
  pub ignored: bool,
  /// Reason the file was ignored, if applicable
  pub ignored_reason: Option<String>,
}

/// Possible actions taken on a file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileAction {
  /// License header was added to the file
  Added,
  /// License year was updated (or needs updating in check mode)
  #[serde(rename = "updated")]
  YearUpdated,
  /// No action was needed (file already had correct license)
  #[serde(rename = "none")]
  NoActionNeeded,
  /// File was skipped for some other reason
  Skipped,
}

/// Helper module for serializing/deserializing PathBuf
mod path_serialization {
  use std::path::PathBuf;

  use serde::{Deserialize, Deserializer, Serializer};

  pub fn serialize<S>(path: &std::path::Path, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.serialize_str(&path.to_string_lossy())
  }

  pub fn deserialize<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
  where
    D: Deserializer<'de>,
  {
    let s = String::deserialize(deserializer)?;
    Ok(PathBuf::from(s))
  }
}

/// Supported report formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
  /// HTML format with styled output
  Html,
  /// JSON format for machine readability
  Json,
  /// CSV format for spreadsheet compatibility
  Csv,
}

impl std::fmt::Display for ReportFormat {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ReportFormat::Html => write!(f, "HTML"),
      ReportFormat::Json => write!(f, "JSON"),
      ReportFormat::Csv => write!(f, "CSV"),
    }
  }
}

/// Error returned when parsing a string into a ReportFormat fails
#[derive(Debug, thiserror::Error)]
#[error("Invalid report format: {0}")]
pub struct ParseReportFormatError(pub String);

impl std::str::FromStr for ReportFormat {
  type Err = ParseReportFormatError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_lowercase().as_str() {
      "html" => Ok(ReportFormat::Html),
      "json" => Ok(ReportFormat::Json),
      "csv" => Ok(ReportFormat::Csv),
      _ => Err(ParseReportFormatError(s.to_string())),
    }
  }
}

/// Report Generator for creating license reports
pub struct ReportGenerator<'a> {
  /// Format of the report to generate
  format: ReportFormat,
  /// Path where the report will be saved
  output_path: &'a std::path::Path,
}

impl<'a> ReportGenerator<'a> {
  /// Create a new report generator
  ///
  /// # Parameters
  ///
  /// * `format` - The format to use for the report
  /// * `output_path` - The path where the report will be saved
  pub const fn new(format: ReportFormat, output_path: &'a std::path::Path) -> Self {
    Self { format, output_path }
  }

  /// Generate a report from a collection of file reports
  ///
  /// # Parameters
  ///
  /// * `files` - List of file reports to include
  /// * `summary` - Processing summary information
  ///
  /// # Returns
  ///
  /// `Ok(())` if the report was generated successfully, or an error if the
  /// report couldn't be generated or written to disk.
  pub fn generate(&self, files: &[FileReport], summary: &ProcessingSummary) -> Result<()> {
    let content = match self.format {
      ReportFormat::Html => self.generate_html(files, summary)?,
      ReportFormat::Json => self.generate_json(files, summary)?,
      ReportFormat::Csv => self.generate_csv(files, summary)?,
    };

    fs::write(self.output_path, content)
      .with_context(|| format!("Failed to write report to {}", self.output_path.display()))
  }

  /// Generate HTML report content
  fn generate_html(&self, files: &[FileReport], summary: &ProcessingSummary) -> Result<String> {
    let date = Local::now().format("%Y-%m-%d %H:%M:%S");

    let mut html = format!(
      r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>edlicense Report</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
        }}
        h1, h2 {{
            color: #2c3e50;
        }}
        .summary {{
            background-color: #f8f9fa;
            border-radius: 4px;
            padding: 15px;
            margin-bottom: 20px;
        }}
        .summary-item {{
            margin-bottom: 8px;
        }}
        table {{
            width: 100%;
            border-collapse: collapse;
            margin-bottom: 20px;
        }}
        th, td {{
            border: 1px solid #dee2e6;
            padding: 8px 12px;
            text-align: left;
        }}
        th {{
            background-color: #f2f2f2;
        }}
        tr:nth-child(even) {{
            background-color: #f8f9fa;
        }}
        .status-with {{
            color: #28a745;
        }}
        .status-without {{
            color: #dc3545;
        }}
        .action-added {{
            color: #28a745;
            font-weight: bold;
        }}
        .action-updated {{
            color: #17a2b8;
        }}
        .action-none {{
            color: #6c757d;
        }}
        .ignored {{
            color: #6c757d;
            font-style: italic;
        }}
    </style>
</head>
<body>
    <h1>edlicense Report</h1>
    <p>Generated on {date}</p>

    <div class="summary">
        <h2>Summary</h2>
        <div class="summary-item">Total files processed: {}</div>
        <div class="summary-item">Files with license: {}</div>
        <div class="summary-item">Files without license: {}</div>
        <div class="summary-item">Files ignored: {}</div>
        <div class="summary-item">License headers added: {}</div>
        <div class="summary-item">License headers updated: {}</div>
        <div class="summary-item">Processing time: {:.2} seconds</div>
    </div>

    <h2>File Details</h2>
    <table>
        <tr>
            <th>File Path</th>
            <th>License Status</th>
            <th>Action Taken</th>
            <th>Notes</th>
        </tr>
"#,
      summary.total_files,
      summary.files_with_license,
      summary.files_without_license,
      summary.files_ignored,
      summary.licenses_added,
      summary.licenses_updated,
      summary.processing_time.as_secs_f64()
    );

    for file in files {
      let path_display = file.path.display();

      let status_class = if file.has_license {
        "status-with"
      } else {
        "status-without"
      };

      let status_text = if file.has_license { "Has license" } else { "No license" };

      let (action_class, action_text) = if file.ignored {
        ("ignored", "Ignored")
      } else if let Some(action) = &file.action_taken {
        match action {
          FileAction::Added => ("action-added", "Added license"),
          FileAction::YearUpdated => ("action-updated", "Updated year"),
          FileAction::NoActionNeeded => ("action-none", "None needed"),
          FileAction::Skipped => ("ignored", "Skipped"),
        }
      } else {
        ("action-none", "None")
      };

      let note = if file.ignored {
        if let Some(reason) = &file.ignored_reason {
          reason
        } else {
          "Matched ignore pattern"
        }
      } else {
        ""
      };

      html.push_str(&format!(
                "        <tr>\n            <td>{}</td>\n            <td class=\"{}\">{}</td>\n            <td class=\"{}\">{}</td>\n            <td>{}</td>\n        </tr>\n",
                path_display, status_class, status_text, action_class, action_text, note
            ));
    }

    html.push_str("    </table>\n</body>\n</html>");

    Ok(html)
  }

  /// Generate JSON report content
  fn generate_json(&self, files: &[FileReport], summary: &ProcessingSummary) -> Result<String> {
    use serde_json::{Map, Value, json, to_string_pretty};

    // Manually build files to ensure correct key format
    let mut files_array = Vec::new();
    for file in files {
      let mut file_map = Map::new();
      file_map.insert(
        "path".to_string(),
        Value::String(file.path.to_string_lossy().to_string()),
      );
      file_map.insert("has_license".to_string(), Value::Bool(file.has_license));

      // Handle action
      let action_str = if file.ignored {
        "ignored".to_string()
      } else if let Some(action) = &file.action_taken {
        match action {
          FileAction::Added => "added".to_string(),
          FileAction::YearUpdated => "updated".to_string(),
          FileAction::NoActionNeeded => "none".to_string(),
          FileAction::Skipped => "skipped".to_string(),
        }
      } else {
        "none".to_string()
      };
      file_map.insert("action".to_string(), Value::String(action_str));

      // Add ignore reason if applicable
      if let Some(ref reason) = file.ignored_reason
        && file.ignored
      {
        file_map.insert("ignored_reason".to_string(), Value::String(reason.clone()));
      }

      files_array.push(Value::Object(file_map));
    }

    // Create summary object
    let mut summary_map = Map::new();
    summary_map.insert("total_files".to_string(), Value::Number(summary.total_files.into()));
    summary_map.insert(
      "files_with_license".to_string(),
      Value::Number(summary.files_with_license.into()),
    );
    summary_map.insert(
      "files_without_license".to_string(),
      Value::Number(summary.files_without_license.into()),
    );
    summary_map.insert("files_ignored".to_string(), Value::Number(summary.files_ignored.into()));
    summary_map.insert(
      "licenses_added".to_string(),
      Value::Number(summary.licenses_added.into()),
    );
    summary_map.insert(
      "licenses_updated".to_string(),
      Value::Number(summary.licenses_updated.into()),
    );
    summary_map.insert(
      "processing_time_seconds".to_string(),
      Value::Number(serde_json::Number::from_f64(summary.processing_time.as_secs_f64()).expect("Valid f64")),
    );

    // Create the final report
    let report = json!({
        "summary": summary_map,
        "files": files_array
    });

    // Format the JSON with pretty-printing
    Ok(to_string_pretty(&report)?)
  }

  /// Generate CSV report content
  fn generate_csv(&self, files: &[FileReport], summary: &ProcessingSummary) -> Result<String> {
    let mut csv = String::new();

    // Add header
    csv.push_str("file_path,has_license,action_taken,notes\n");

    // Add file details
    for file in files {
      let path = file.path.to_string_lossy().replace(',', "%2C"); // Escape commas in path

      let action = if file.ignored {
        "Ignored"
      } else if let Some(action) = &file.action_taken {
        match action {
          FileAction::Added => "Added",
          FileAction::YearUpdated => "Updated",
          FileAction::NoActionNeeded => "None",
          FileAction::Skipped => "Skipped",
        }
      } else {
        "None"
      };

      let note = if file.ignored {
        if let Some(reason) = &file.ignored_reason {
          reason.replace(',', "%2C") // Escape commas in note
        } else {
          "Matched ignore pattern".to_string()
        }
      } else {
        String::new()
      };

      csv.push_str(&format!("{},{},{},{}\n", path, file.has_license, action, note));
    }

    // Add summary at the end
    csv.push_str("\n# Summary\n");
    csv.push_str(&format!("Total files processed,{}\n", summary.total_files));
    csv.push_str(&format!("Files with license,{}\n", summary.files_with_license));
    csv.push_str(&format!("Files without license,{}\n", summary.files_without_license));
    csv.push_str(&format!("Files ignored,{}\n", summary.files_ignored));
    csv.push_str(&format!("License headers added,{}\n", summary.licenses_added));
    csv.push_str(&format!("License headers updated,{}\n", summary.licenses_updated));
    csv.push_str(&format!(
      "Processing time (seconds),{:.2}\n",
      summary.processing_time.as_secs_f64()
    ));
    csv.push_str(&format!("Generated on,{}\n", Local::now().format("%Y-%m-%d %H:%M:%S")));

    Ok(csv)
  }
}

/// Summary of the processing results
#[derive(Debug, Clone, Serialize)]
pub struct ProcessingSummary {
  /// Total number of files processed
  pub total_files: usize,
  /// Number of files with license headers
  pub files_with_license: usize,
  /// Number of files without license headers
  pub files_without_license: usize,
  /// Number of files ignored
  pub files_ignored: usize,
  /// Number of license headers added
  pub licenses_added: usize,
  /// Number of license headers updated
  pub licenses_updated: usize,
  /// Total processing time
  #[serde(skip_serializing)]
  pub processing_time: std::time::Duration,
  /// Processing time in seconds for serialization
  #[serde(rename = "processing_time_seconds")]
  pub processing_time_secs: f64,
  /// Timestamp when the report was generated
  #[serde(skip_serializing_if = "Option::is_none")]
  pub timestamp: Option<i64>,
}

impl ProcessingSummary {
  /// Create a new ProcessingSummary initialized to zero
  pub fn new(processing_time: std::time::Duration) -> Self {
    Self {
      total_files: 0,
      files_with_license: 0,
      files_without_license: 0,
      files_ignored: 0,
      licenses_added: 0,
      licenses_updated: 0,
      processing_time,
      processing_time_secs: processing_time.as_secs_f64(),
      timestamp: Some(Local::now().timestamp()),
    }
  }

  /// Create a ProcessingSummary from a collection of FileReports
  pub fn from_reports(files: &[FileReport], processing_time: std::time::Duration) -> Self {
    let mut summary = Self::new(processing_time);

    summary.total_files = files.len();

    for file in files {
      if file.ignored {
        summary.files_ignored += 1;
        continue;
      }

      if file.has_license {
        summary.files_with_license += 1;
      } else {
        summary.files_without_license += 1;
      }

      if let Some(action) = &file.action_taken {
        match action {
          FileAction::Added => summary.licenses_added += 1,
          FileAction::YearUpdated => summary.licenses_updated += 1,
          _ => {}
        }
      }
    }

    summary
  }
}

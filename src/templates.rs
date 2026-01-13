//! # Templates Module
//!
//! This module provides functionality for managing license templates, rendering
//! them with specific data (like the copyright year), and formatting them
//! appropriately for different file types.
//!
//! The module includes:
//! - [`TemplateManager`] for loading and rendering license templates
//! - [`LicenseData`] for providing data to fill in templates
//! - [`CommentStyle`] for defining how comments should be formatted in
//!   different file types
//!
//! ## Example
//!
//! ```rust,no_run
//! use std::path::Path;
//!
//! use edlicense::templates::{LicenseData, TemplateManager};
//!
//! # fn main() -> anyhow::Result<()> {
//! // Create license data with the current year
//! let license_data = LicenseData {
//!   year: "2025".to_string(),
//! };
//!
//! // Create and initialize template manager
//! let mut template_manager = TemplateManager::new();
//! template_manager.load_template(Path::new("LICENSE.txt"))?;
//!
//! // Render the template with the license data
//! let license_text = template_manager.render(&license_data)?;
//!
//! // Format the license for a specific file type
//! let formatted_license =
//!   template_manager.format_for_file_type(&license_text, Path::new("main.rs"));
//! # Ok(())
//! # }
//! ```

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::verbose_log;

/// Data used to fill out a license template.
///
/// # Fields
///
/// * `year` - The copyright year to use in the license
pub struct LicenseData {
  /// The copyright year to use in the license
  pub year: String,
}

/// Manager for loading, rendering, and formatting license templates.
///
/// The `TemplateManager` is responsible for:
/// - Loading license templates from files
/// - Rendering templates with specific data (like the year)
/// - Formatting license text with appropriate comment styles for different file
///   types
///
/// # Examples
///
/// ```rust,no_run
/// use std::path::Path;
///
/// use edlicense::templates::{LicenseData, TemplateManager};
///
/// # fn main() -> anyhow::Result<()> {
/// let mut manager = TemplateManager::new();
/// manager.load_template(Path::new("LICENSE.txt"))?;
///
/// let data = LicenseData {
///   year: "2025".to_string(),
/// };
/// let license = manager.render(&data)?;
///
/// // Format for a Rust file
/// let formatted = manager.format_for_file_type(&license, Path::new("main.rs"));
/// # Ok(())
/// # }
/// ```
pub struct TemplateManager {
  /// The loaded license template content
  template: String,
}

impl Default for TemplateManager {
  fn default() -> Self {
    Self::new()
  }
}

impl TemplateManager {
  /// Creates a new empty template manager.
  ///
  /// The manager is initialized with an empty template string.
  /// You must call [`load_template`](Self::load_template) before using it.
  ///
  /// # Returns
  ///
  /// A new `TemplateManager` instance with an empty template.
  pub const fn new() -> Self {
    Self {
      template: String::new(),
    }
  }

  /// Loads a custom license template from a file.
  ///
  /// This method reads the template content from the specified file path
  /// and stores it in the template manager for later use.
  ///
  /// # Parameters
  ///
  /// * `path` - Path to the license template file
  ///
  /// # Returns
  ///
  /// `Ok(())` if the template was loaded successfully, or an error if the file
  /// could not be read.
  ///
  /// # Errors
  ///
  /// Returns an error if:
  /// - The file does not exist
  /// - The file cannot be read
  /// - The file content is not valid UTF-8
  pub fn load_template(&mut self, path: &Path) -> Result<()> {
    verbose_log!("Loading template from: {}", path.display());

    let template_content =
      fs::read_to_string(path).with_context(|| format!("Failed to read license template file: {}", path.display()))?;

    verbose_log!("Template content:\n{}", template_content);

    self.template = template_content;

    Ok(())
  }

  /// Renders a license template with the given data.
  ///
  /// This method replaces template variables with actual values from the
  /// provided license data. Currently, it supports the `{{year}}` variable
  /// which is replaced with the year from the license data.
  ///
  /// # Parameters
  ///
  /// * `data` - License data containing values to substitute into the template
  ///
  /// # Returns
  ///
  /// The rendered license text with variables replaced, or an error if
  /// rendering fails.
  pub fn render(&self, data: &LicenseData) -> Result<String> {
    verbose_log!("Rendering template with year: {}", data.year);

    // Simple string replacement
    let rendered = self.template.replace("{{year}}", &data.year);

    Ok(rendered)
  }

  /// Formats the license text with the appropriate comment style for the given
  /// file type.
  ///
  /// This method determines the appropriate comment style based on the file
  /// extension and formats the license text accordingly.
  ///
  /// # Parameters
  ///
  /// * `license_text` - The rendered license text to format
  /// * `file_path` - Path to the file, used to determine the appropriate
  ///   comment style
  ///
  /// # Returns
  ///
  /// `Some(String)` with the formatted license text if the file type is supported,
  /// `None` if the file type is not recognized (e.g., binary files like .png).
  pub fn format_for_file_type(&self, license_text: &str, file_path: &Path) -> Option<String> {
    let comment_style = get_comment_style_for_file(file_path)?;
    Some(format_with_comment_style(license_text, &comment_style))
  }
}

/// Defines the comment style for different file types.
///
/// This struct represents how comments should be formatted for different
/// programming languages and file types. It includes markers for the top,
/// middle, and bottom of a comment block.
///
/// # Fields
///
/// * `top` - The string to use at the top of a comment block (e.g., "/*")
/// * `middle` - The string to use at the beginning of each line in the comment
///   block (e.g., " * ")
/// * `bottom` - The string to use at the bottom of a comment block (e.g., "
///   */")
#[derive(Debug)]
pub struct CommentStyle {
  /// The string to use at the top of a comment block
  pub top: &'static str,

  /// The string to use at the beginning of each line in the comment block
  pub middle: &'static str,

  /// The string to use at the bottom of a comment block
  pub bottom: &'static str,
}

/// Determines the appropriate comment style for a file based on its extension.
///
/// This function examines the file extension (and in some cases the filename)
/// to determine the appropriate comment style for the given file type.
///
/// # Parameters
///
/// * `path` - Path to the file
///
/// # Returns
///
/// `Some(CommentStyle)` if the file type is supported, `None` if the file type
/// is not recognized and should be skipped (e.g., binary files like .png, .jpg).
///
/// # Supported File Types
///
/// The function supports many common file types including:
/// - C/C++/C#/Go/Rust/Swift/Dart: `// comment style`
/// - Java/Scala/Kotlin: `/* comment style */`
/// - JavaScript/TypeScript/CSS: `/** comment style */`
/// - Python/Shell/YAML/Ruby: `# comment style`
/// - HTML/XML/Vue: `<!-- comment style -->`
/// - And many more...
///
/// Files with unknown extensions are skipped to avoid corrupting binary files.
pub fn get_comment_style_for_file(path: &Path) -> Option<CommentStyle> {
  let file_name = path
    .file_name()
    .and_then(|name| name.to_str())
    .unwrap_or("")
    .to_lowercase();

  let extension = path
    .extension()
    .and_then(|ext| ext.to_str())
    .unwrap_or("")
    .to_lowercase();

  match extension.as_str() {
    "c" | "h" | "gv" | "java" | "scala" | "kt" | "kts" => Some(CommentStyle {
      top: "/*",
      middle: " * ",
      bottom: " */",
    }),
    "js" | "mjs" | "cjs" | "jsx" | "tsx" | "css" | "scss" | "sass" | "ts" => Some(CommentStyle {
      top: "/**",
      middle: " * ",
      bottom: " */",
    }),
    "cc" | "cpp" | "cs" | "go" | "hcl" | "hh" | "hpp" | "m" | "mm" | "proto" | "rs" | "swift" | "dart" | "groovy"
    | "v" | "sv" => Some(CommentStyle {
      top: "",
      middle: "// ",
      bottom: "",
    }),
    "py" | "sh" | "yaml" | "yml" | "rb" | "tcl" | "tf" | "bzl" | "pl" | "pp" | "toml" => Some(CommentStyle {
      top: "",
      middle: "# ",
      bottom: "",
    }),
    "el" | "lisp" => Some(CommentStyle {
      top: "",
      middle: ";; ",
      bottom: "",
    }),
    "erl" => Some(CommentStyle {
      top: "",
      middle: "% ",
      bottom: "",
    }),
    "hs" | "sql" | "sdl" => Some(CommentStyle {
      top: "",
      middle: "-- ",
      bottom: "",
    }),
    "html" | "xml" | "vue" | "wxi" | "wxl" | "wxs" => Some(CommentStyle {
      top: "<!--",
      middle: " ",
      bottom: "-->",
    }),
    "php" => Some(CommentStyle {
      top: "",
      middle: "// ",
      bottom: "",
    }),
    "j2" => Some(CommentStyle {
      top: "{#",
      middle: "",
      bottom: "#}",
    }),
    "ml" | "mli" | "mll" | "mly" => Some(CommentStyle {
      top: "(**",
      middle: "   ",
      bottom: "*)",
    }),
    _ => {
      // Handle special cases based on filename
      if file_name == "cmakelists.txt"
        || file_name.ends_with(".cmake.in")
        || file_name.ends_with(".cmake")
        || file_name == "dockerfile"
        || file_name.ends_with(".dockerfile")
      {
        Some(CommentStyle {
          top: "",
          middle: "# ",
          bottom: "",
        })
      } else {
        // Unknown file type - skip to avoid corrupting binary files
        None
      }
    }
  }
}

/// Formats license text with the given comment style.
///
/// This function takes a license text and formats it with the appropriate
/// comment markers based on the provided comment style. It handles:
/// - Adding top comment markers (if any)
/// - Prefixing each line with the middle comment marker
/// - Adding bottom comment markers (if any)
/// - Ensuring proper spacing and newlines
///
/// # Parameters
///
/// * `license_text` - The license text to format
/// * `style` - The comment style to use for formatting
///
/// # Returns
///
/// The formatted license text with appropriate comment markers.
pub fn format_with_comment_style(license_text: &str, style: &CommentStyle) -> String {
  let mut result = String::new();

  // Add top comment marker if present
  if !style.top.is_empty() {
    result.push_str(style.top);
    result.push('\n');
  }

  // Add each line with the middle comment marker
  for line in license_text.lines() {
    if line.is_empty() {
      result.push_str(style.middle.trim_end());
    } else {
      result.push_str(style.middle);
      result.push_str(line);
    }
    result.push('\n');
  }

  // Add bottom comment marker if present
  if !style.bottom.is_empty() {
    result.push_str(style.bottom);
    result.push('\n');
  }

  // Add an extra newline at the end
  result.push('\n');

  result
}

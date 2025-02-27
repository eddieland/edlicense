use std::fs;
use std::path::Path;
use anyhow::{Context, Result};

use crate::verbose_log;

/// Data used to fill out a license template
pub struct LicenseData {
    pub year: String,
}

/// License template manager
pub struct TemplateManager {
    template: String,
}

impl TemplateManager {
    /// Create a new template manager
    pub fn new() -> Self {
        Self {
            template: String::new(),
        }
    }
    
    /// Load a custom template from a file
    pub fn load_template(&mut self, path: &Path) -> Result<()> {
        verbose_log!("Loading template from: {}", path.display());
        
        let template_content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read license template file: {}", path.display()))?;
        
        verbose_log!("Template content:\n{}", template_content);
        
        self.template = template_content;
        
        Ok(())
    }
    
    /// Render a license template with the given data
    pub fn render(&self, data: &LicenseData) -> Result<String> {
        verbose_log!("Rendering template with year: {}", data.year);
        
        // Simple string replacement
        let rendered = self.template
            .replace("{{Year}}", &data.year);
        
        Ok(rendered)
    }
    
    /// Format the license text with the appropriate comment style for the given file type
    pub fn format_for_file_type(&self, license_text: &str, file_path: &Path) -> String {
        let comment_style = get_comment_style_for_file(file_path);
        format_with_comment_style(license_text, &comment_style)
    }
}

/// Comment style for different file types
#[derive(Debug)]
pub struct CommentStyle {
    pub top: &'static str,
    pub middle: &'static str,
    pub bottom: &'static str,
}

/// Get the appropriate comment style for a file based on its extension
fn get_comment_style_for_file(path: &Path) -> CommentStyle {
    let file_name = path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    match extension.as_str() {
        "c" | "h" | "gv" | "java" | "scala" | "kt" | "kts" => {
            CommentStyle {
                top: "/*",
                middle: " * ",
                bottom: " */",
            }
        },
        "js" | "mjs" | "cjs" | "jsx" | "tsx" | "css" | "scss" | "sass" | "ts" => {
            CommentStyle {
                top: "/**",
                middle: " * ",
                bottom: " */",
            }
        },
        "cc" | "cpp" | "cs" | "go" | "hcl" | "hh" | "hpp" | "m" | "mm" | "proto" | "rs" | "swift" | "dart" | "groovy" | "v" | "sv" => {
            CommentStyle {
                top: "",
                middle: "// ",
                bottom: "",
            }
        },
        "py" | "sh" | "yaml" | "yml" | "rb" | "tcl" | "tf" | "bzl" | "pl" | "pp" | "toml" => {
            CommentStyle {
                top: "",
                middle: "# ",
                bottom: "",
            }
        },
        "el" | "lisp" => {
            CommentStyle {
                top: "",
                middle: ";; ",
                bottom: "",
            }
        },
        "erl" => {
            CommentStyle {
                top: "",
                middle: "% ",
                bottom: "",
            }
        },
        "hs" | "sql" | "sdl" => {
            CommentStyle {
                top: "",
                middle: "-- ",
                bottom: "",
            }
        },
        "html" | "xml" | "vue" | "wxi" | "wxl" | "wxs" => {
            CommentStyle {
                top: "<!--",
                middle: " ",
                bottom: "-->",
            }
        },
        "php" => {
            CommentStyle {
                top: "",
                middle: "// ",
                bottom: "",
            }
        },
        "j2" => {
            CommentStyle {
                top: "{#",
                middle: "",
                bottom: "#}",
            }
        },
        "ml" | "mli" | "mll" | "mly" => {
            CommentStyle {
                top: "(**",
                middle: "   ",
                bottom: "*)",
            }
        },
        _ => {
            // Handle special cases based on filename
            if file_name == "cmakelists.txt" || file_name.ends_with(".cmake.in") || file_name.ends_with(".cmake") {
                CommentStyle {
                    top: "",
                    middle: "# ",
                    bottom: "",
                }
            } else if file_name == "dockerfile" || file_name.ends_with(".dockerfile") {
                CommentStyle {
                    top: "",
                    middle: "# ",
                    bottom: "",
                }
            } else {
                // Default to C-style comments if we can't determine the file type
                CommentStyle {
                    top: "",
                    middle: "// ",
                    bottom: "",
                }
            }
        }
    }
}

/// Format license text with the given comment style
fn format_with_comment_style(license_text: &str, style: &CommentStyle) -> String {
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
// crates/substring_marker_snippet_extractor/src/processor/file_processor.rs

use std::path::Path;
use anyhow::{Result, anyhow};
use std::fs;

use crate::{filter_substring_markers_with_placeholder, file_uses_markers, extract_enclosing_block};

/// Trait that abstracts file processing.
pub trait FileProcessor {
    /// Processes the file at the given path, optionally using the expected file basename.
    fn process_file(&self, file_path: &Path, todo_file_basename: Option<&str>) -> Result<String>;
}

/// Default implementation of the `FileProcessor` trait.
pub struct DefaultFileProcessor;

impl FileProcessor for DefaultFileProcessor {
    fn process_file(&self, file_path: &Path, todo_file_basename: Option<&str>) -> Result<String> {
        let file_path_str = file_path.to_str().ok_or_else(|| anyhow!("Invalid file path"))?;
        let file_content = fs::read_to_string(file_path)?;
        
        // Use marker filtering if markers are present.
        // Here we pass in the placeholder "// ..." for display purposes.
        let processed_content = if file_content.lines().any(|line| line.trim() == "// v") {
            filter_substring_markers_with_placeholder(&file_content, "// ...")
        } else {
            file_content.clone()
        };

        let file_basename = file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let mut combined_content = processed_content;

        // Append the enclosing context if markers are used and the basename matches.
        if file_uses_markers(&file_content) {
            if let Some(expected_basename) = todo_file_basename {
                if file_basename == expected_basename {
                    if let Some(context) = extract_enclosing_block(file_path_str) {
                        combined_content.push_str("\n\n// Enclosing function context:\n");
                        combined_content.push_str(&context);
                    }
                }
            }
        }
        
        Ok(combined_content)
    }
}

/// Public API function to process a file using a provided `FileProcessor` implementation.
pub fn process_file_with_processor<P: AsRef<Path>>(
    processor: &dyn FileProcessor,
    file_path: P,
    todo_file_basename: Option<&str>
) -> Result<String> {
    processor.process_file(file_path.as_ref(), todo_file_basename)
}

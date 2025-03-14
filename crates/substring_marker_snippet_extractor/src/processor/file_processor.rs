// crates/substring_marker_snippet_extractor/src/processor/file_processor.rs

use std::path::Path;
use anyhow::{Result, anyhow};
use std::fs;

use crate::utils::marker_utils::{filter_substring_markers, file_uses_markers, extract_enclosing_block};

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
        
        // Use marker filtering if the SLIM_MODE is enabled or if the file naturally contains markers.
        let processed_content = if std::env::var("SLIM_MODE").is_ok() || file_content.lines().any(|line| line.trim() == "// v") {
            filter_substring_markers(&file_content, "// ...")
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    /// Dummy processor that always returns an error.
    pub struct FailingProcessor;

    impl FileProcessor for FailingProcessor {
        fn process_file(&self, _file_path: &Path, _todo_file_basename: Option<&str>) -> Result<String> {
            Err(anyhow!("Simulated processing failure"))
        }
    }

    #[test]
    fn test_default_processor_success() {
        // When there are no markers, the processor should simply return the raw content.
        let raw_content = "fn main() {\n    println!(\"Hello, world!\");\n}\n";
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", raw_content).unwrap();
        let processor = DefaultFileProcessor;
        let result = processor.process_file(
            temp_file.path(),
            Some(temp_file.path().file_name().unwrap().to_str().unwrap())
        ).unwrap();
        assert_eq!(result, raw_content);
    }

    #[test]
    fn test_failing_processor() {
        let processor = FailingProcessor;
        let temp_file = NamedTempFile::new().unwrap();
        let result = processor.process_file(temp_file.path(), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_default_processor_with_markers() {
        // Create a temporary file with a candidate declaration (using Swift syntax),
        // markers, and a TODO. Using `concat!` ensures that the literal is not affected
        // by source code indentation.
        let content = concat!(
            "Some preamble text\n",
            "func myFunction() {\n",
            "let x = 10;\n",
            "}\n",
            "Other text\n",
            "// v\n",
            "ignored text\n",
            "// ^\n",
            "Trailing text\n",
            "// TODO: - Do something"
        );
        // Generate the expected filtered output using the same function.
        let expected_filtered = filter_substring_markers(content, "// ...");
        // The extract_enclosing_block function should extract the candidate declaration
        // exactly as it appears in the file:
        let expected_context = "func myFunction() {\nlet x = 10;\n}";
        // The implementation appends the context with two newlines before the header.
        let expected_context_appended = format!("\n\n// Enclosing function context:\n{}", expected_context);
        let expected = format!("{}{}", expected_filtered, expected_context_appended);
    
        // Create an isolated temporary file for this test.
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", content).unwrap();
        let file_basename = temp_file
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
    
        let processor = DefaultFileProcessor;
        let result = processor
            .process_file(temp_file.path(), Some(&file_basename))
            .unwrap();
    
        // Compare the outputs by splitting into tokens, ignoring whitespace differences.
        let result_tokens: Vec<_> = result.split_whitespace().collect();
        let expected_tokens: Vec<_> = expected.split_whitespace().collect();
    
        assert_eq!(
            result_tokens, expected_tokens,
            "\n\nTokenized output did not match expected output."
        );
    }

    #[test]
    fn test_slim_mode_no_markers() {
        // This test verifies that when SLIM_MODE is enabled,
        // even a file with no markers is processed as if it uses markers.
        std::env::set_var("SLIM_MODE", "true");
    
        let raw_content = "fn main() {\n    println!(\"Hello, slim world!\");\n}\n";
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", raw_content).unwrap();
        let processor = DefaultFileProcessor;
        let result = processor.process_file(
            temp_file.path(),
            Some(temp_file.path().file_name().unwrap().to_str().unwrap())
        ).unwrap();
        // In slim mode, the file should be processed as if it has markers.
        let expected = filter_substring_markers(raw_content, "// ...");
        assert_eq!(result, expected);
    
        std::env::remove_var("SLIM_MODE");
    }
}

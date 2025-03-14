// src/processor/file_processor.rs
use std::path::Path;
use anyhow::{Result, anyhow};
use std::fs;

use crate::utils::marker_utils::{filter_substring_markers, file_uses_markers, extract_enclosing_block};

/// Trait that abstracts file processing.
/// Now accepts a `slim_mode` flag to force treating the file as if it uses substring markers.
pub trait FileProcessor {
    fn process_file(&self, file_path: &Path, todo_file_basename: Option<&str>, slim_mode: bool) -> Result<String>;
}

/// Default implementation of the `FileProcessor` trait.
pub struct DefaultFileProcessor;

impl FileProcessor for DefaultFileProcessor {
    fn process_file(&self, file_path: &Path, todo_file_basename: Option<&str>, slim_mode: bool) -> Result<String> {
        let file_path_str = file_path.to_str().ok_or_else(|| anyhow!("Invalid file path"))?;
        let file_content = fs::read_to_string(file_path)?;
        
        // Use substring marker filtering if slim_mode is enabled
        // or if the file already contains the expected marker ("// v").
        let processed_content = if slim_mode || file_content.lines().any(|line| line.trim() == "// v") {
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
/// The `slim_mode` flag is passed along to the processor.
pub fn process_file_with_processor<P: AsRef<Path>>(
    processor: &dyn FileProcessor,
    file_path: P,
    todo_file_basename: Option<&str>,
    slim_mode: bool,
) -> Result<String> {
    processor.process_file(file_path.as_ref(), todo_file_basename, slim_mode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    /// Dummy processor that always returns an error.
    pub struct FailingProcessor;

    impl FileProcessor for FailingProcessor {
        fn process_file(
            &self,
            _file_path: &Path,
            _todo_file_basename: Option<&str>,
            _slim_mode: bool,
        ) -> Result<String> {
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
            Some(temp_file.path().file_name().unwrap().to_str().unwrap()),
            false, // slim_mode is false
        ).unwrap();
        assert_eq!(result, raw_content);
    }

    #[test]
    fn test_failing_processor() {
        let processor = FailingProcessor;
        let temp_file = NamedTempFile::new().unwrap();
        let result = processor.process_file(temp_file.path(), None, false);
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
        // Expected behavior:
        // 1. The marker filtering produces the following output:
        let expected_filtered = "\n\n// ...\n\nignored text\n\n\n// ...\n\n\n\n";
        // 2. The extract_enclosing_block function should extract the candidate declaration
        //    exactly as it appears in the file:
        let expected_context = "func myFunction() {\nlet x = 10;\n}";
        // 3. The processor appends the context, prefixed by the header.
        let expected_context_appended = format!("// Enclosing function context:\n{}", expected_context);
        let expected = format!("{}{}", expected_filtered, expected_context_appended);

        // Create an isolated temporary file for this test.
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
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
            .process_file(temp_file.path(), Some(&file_basename), false)
            .unwrap();

        assert_eq!(result, expected);
    }
    
    #[test]
    fn test_default_processor_slim_mode_without_markers() {
        // When there are no markers in the file, but slim_mode is enabled,
        // the processor should apply the filter_substring_markers logic.
        let raw_content = "fn main() {\n    println!(\"Hello, world!\");\n}\n";
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", raw_content).unwrap();
        let processor = DefaultFileProcessor;

        // Enable slim mode.
        let result = processor
            .process_file(
                temp_file.path(),
                Some(temp_file.path().file_name().unwrap().to_str().unwrap()),
                true, // slim_mode enabled
            )
            .unwrap();

        // The expected output is what the filter_substring_markers produces for raw_content.
        // Note: This behavior depends on how filter_substring_markers handles content without markers.
        // If it simply returns a placeholder, adjust the expected string accordingly.
        let expected = crate::utils::marker_utils::filter_substring_markers(raw_content, "// ...");

        assert_eq!(result, expected);
    }
}

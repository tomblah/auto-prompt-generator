// crates/assemble_prompt/src/file_processor.rs

use anyhow::Result;
use std::fs;
use std::path::Path;

use substring_marker_snippet_extractor::{filter_substring_markers, FileAnalysis};

/// Trait that abstracts file processing.
pub trait FileProcessor {
    /// Processes the file at the given path, optionally using the expected file basename.
    fn process_file(&self, file_path: &Path, todo_file_basename: Option<&str>) -> Result<String>;
}

/// Default implementation of the `FileProcessor` trait.
pub struct DefaultFileProcessor;

impl FileProcessor for DefaultFileProcessor {
    fn process_file(&self, file_path: &Path, todo_file_basename: Option<&str>) -> Result<String> {
        let file_content = fs::read_to_string(file_path)?;
        let analysis = FileAnalysis::new(&file_content);

        let processed_content = if file_content.lines().any(|line| line.trim() == "// v") {
            filter_substring_markers(&file_content, "// ...")
        } else {
            file_content.clone()
        };

        let file_basename = file_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let mut combined_content = processed_content;

        if analysis.has_markers() {
            if let Some(expected_basename) = todo_file_basename {
                if file_basename == expected_basename {
                    if let Some(context) = analysis.enclosing_block(None) {
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
    todo_file_basename: Option<&str>,
) -> Result<String> {
    processor.process_file(file_path.as_ref(), todo_file_basename)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Dummy processor that always returns an error.
    pub struct FailingProcessor;

    impl FileProcessor for FailingProcessor {
        fn process_file(
            &self,
            _file_path: &Path,
            _todo_file_basename: Option<&str>,
        ) -> Result<String> {
            Err(anyhow!("Simulated processing failure"))
        }
    }

    #[test]
    fn test_default_processor_success() {
        let raw_content = "fn main() {\n    println!(\"Hello, world!\");\n}\n";
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", raw_content).unwrap();
        let processor = DefaultFileProcessor;
        let result = processor
            .process_file(
                temp_file.path(),
                Some(temp_file.path().file_name().unwrap().to_str().unwrap()),
            )
            .unwrap();
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
        let expected_filtered = "\n\n// ...\n\nignored text\n\n\n// ...\n\n\n\n";
        let expected_context = "func myFunction() {\nlet x = 10;\n}";
        let expected_context_appended =
            format!("// Enclosing function context:\n{}", expected_context);
        let expected = format!("{}{}", expected_filtered, expected_context_appended);

        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        use std::io::Write;
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

        assert_eq!(result, expected);
    }
}

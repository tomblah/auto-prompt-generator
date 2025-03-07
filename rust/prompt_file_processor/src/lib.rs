use std::fs;
use std::path::Path;
use anyhow::{Result, anyhow};

// Import the external crate for filtering substring markers.
use filter_substring_markers::filter_substring_markers;
// Import the extract_enclosing_block function from extract_enclosing_function,
// renaming it for clarity.
use extract_enclosing_function::extract_enclosing_block as extract_block_from_content;

/// Checks if the file uses both markers ("// v" and "// ^").
pub fn file_uses_markers(content: &str) -> bool {
    let has_open = content.lines().any(|line| line.trim() == "// v");
    let has_close = content.lines().any(|line| line.trim() == "// ^");
    has_open && has_close
}

/// Returns the index (zero-based) of the first line that contains "// TODO: - ".
pub fn todo_index(content: &str) -> Option<usize> {
    content.lines().position(|line| line.contains("// TODO: - "))
}

/// Determines whether the TODO is already inside a marker block by counting marker boundaries
/// from the start of the file up to the TODO line.
pub fn is_todo_inside_markers(content: &str, todo_idx: usize) -> bool {
    let lines: Vec<&str> = content.lines().collect();
    let mut marker_depth = 0;
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed == "// v" {
            marker_depth += 1;
        } else if trimmed == "// ^" {
            if marker_depth > 0 {
                marker_depth -= 1;
            }
        }
        if i == todo_idx {
            break;
        }
    }
    marker_depth > 0
}

/// Public API: processes the file at `file_path` by filtering its content based on markers
/// and, if applicable, appending an enclosing context block extracted via candidate heuristics.
/// The optional parameter `todo_file_basename` is used so that context is only appended if
/// the file's basename matches.
/// Returns the processed content as a `String`.
pub fn process_file<P: AsRef<Path>>(file_path: P, todo_file_basename: Option<&str>) -> Result<String> {
    let file_path_ref = file_path.as_ref();
    let _file_path_str = file_path_ref.to_str().ok_or_else(|| anyhow!("Invalid file path"))?;
    let file_content = fs::read_to_string(file_path_ref)?;
    
    // If the file contains the marker, use the filtered content; otherwise, use the raw content.
    let processed_content = if file_content.lines().any(|line| line.trim() == "// v") {
        filter_substring_markers(&file_content)
    } else {
        file_content.clone()
    };
    
    // Determine the file's basename.
    let file_basename = file_path_ref
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    
    let mut combined_content = processed_content;
    
    // Append the enclosing block context if the file uses markers and the basename matches.
    if file_uses_markers(&file_content) {
        if let Some(expected_basename) = todo_file_basename {
            if file_basename == expected_basename {
                if let Some(context) = extract_block_from_content(&file_content) {
                    combined_content.push_str("\n\n// Enclosing function context:\n");
                    combined_content.push_str(&context);
                }
            }
        }
    }
    
    Ok(combined_content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;
    use std::fs;

    // --- Tests for filter_substring_markers ---

    const INPUT_WITH_MARKERS: &str = r#"
Some initial content that should not appear.
// v
line 1 inside marker
line 2 inside marker
// ^
Some trailing content that should not appear.
"#;

    #[test]
    fn test_filter_substring_markers() {
        let expected = "\n// ...\nline 1 inside marker\nline 2 inside marker\n\n// ...\n";
        let result = filter_substring_markers(INPUT_WITH_MARKERS);
        assert_eq!(result, expected);
    }

    // --- Tests for marker detection functions ---

    #[test]
    fn test_file_uses_markers_true() {
        let input = r#"
code line 1
// v
inside marker
// ^
code line 2
"#;
        assert!(file_uses_markers(input));
    }

    #[test]
    fn test_file_uses_markers_false() {
        let input = r#"
code line 1
// v
inside marker
code line 2
"#;
        assert!(!file_uses_markers(input));
    }

    #[test]
    fn test_todo_index() {
        let input = r#"
line one
// TODO: - do something
line three
"#;
        let idx = todo_index(input);
        assert!(idx.is_some());
        let idx_val = idx.unwrap();
        let line = input.lines().nth(idx_val).unwrap();
        assert!(line.contains("// TODO: -"));
    }

    #[test]
    fn test_is_todo_inside_markers_false() {
        let content = r#"
func example() {
    // code
}

// v
// some extra context
// ^
 // TODO: - do something
"#;
        let todo_idx = todo_index(content).unwrap();
        assert!(!is_todo_inside_markers(content, todo_idx));
    }

    #[test]
    fn test_is_todo_inside_markers_true() {
        let content = r#"
func example() {
    // v
    // TODO: - do something
    // ^
}
"#;
        let todo_idx = todo_index(content).unwrap();
        assert!(is_todo_inside_markers(content, todo_idx));
    }

    // --- Integration tests for process_file ---

    // Test that if the file does not contain any markers, process_file returns the raw file content.
    #[test]
    fn test_process_file_no_markers() {
        let raw_content = "fn main() {\n    println!(\"Hello, world!\");\n}\n";
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        write!(temp_file, "{}", raw_content).expect("Failed to write to temp file");
        // Pass an expected basename (which is irrelevant when there are no markers).
        let result = process_file(temp_file.path(), Some("irrelevant.txt"))
            .expect("process_file should succeed for file without markers");
        // Without markers, process_file should return the raw content.
        assert_eq!(result, raw_content);
    }

    // Test that if markers are present but the provided expected basename does not match,
    // process_file returns the filtered content without appending the enclosing context.
    #[test]
    fn test_process_file_markers_basename_mismatch() {
        let content_with_markers = r#"
func sampleFunction() {
    println!("Start");
}

// v
// Marker block content line 1
// Marker block content line 2
// ^

 // TODO: - perform a task
"#;
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        write!(temp_file, "{}", content_with_markers).expect("Failed to write to temp file");
        let file_path = temp_file.path();
        // Use an expected basename that does NOT match the file's actual basename.
        let wrong_basename = "mismatch.txt";
        let result = process_file(file_path, Some(wrong_basename))
            .expect("process_file should succeed");
        let expected_filtered = filter_substring_markers(content_with_markers);
        assert_eq!(result, expected_filtered, "When expected basename does not match, no context should be appended");
    }

    // Test that if markers are present and the provided expected basename matches,
    // process_file returns the filtered content with the enclosing context appended.
    #[test]
    fn test_process_file_markers_basename_match() {
        let content_with_markers = r#"
func myFunction() {
    println!("Hello");
}

// v
// Extra context that is not part of the function block.
// ^
 // TODO: - perform important task
"#;
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        write!(temp_file, "{}", content_with_markers).expect("Failed to write to temp file");
        let file_path = temp_file.path();
        // Provide the expected basename that exactly matches the file's basename.
        let expected_basename = file_path.file_name().unwrap().to_str().unwrap();
        let result = process_file(file_path, Some(expected_basename))
            .expect("process_file should succeed");

        // The expected result is the filtered content plus the extracted enclosing context.
        let filtered = filter_substring_markers(content_with_markers);
        let expected_context = extract_block_from_content(content_with_markers)
            .unwrap_or_else(|| "".to_string());
        let expected = if !expected_context.is_empty() {
            format!("{}\n\n// Enclosing function context:\n{}", filtered, expected_context)
        } else {
            filtered
        };
        assert_eq!(result, expected, "When expected basename matches, the enclosing block should be appended");
    }

    // Test that process_file returns an error when the file does not exist.
    #[test]
    fn test_process_file_file_not_found() {
        // Create a temporary file and then remove it.
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let file_path = temp_file.into_temp_path().keep().expect("Failed to persist temp file");
        fs::remove_file(&file_path).expect("Failed to delete temporary file");

        let result = process_file(&file_path, Some("dummy.txt"));
        assert!(result.is_err(), "Expected process_file to error when file does not exist");
    }
}

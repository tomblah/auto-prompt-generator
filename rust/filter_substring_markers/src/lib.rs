// rust/filter_substring_markers/src/lib.rs

/// Processes file content by outputting only the text between substring markers.
///
/// The markers are defined as:
///   - An opening marker: a line that, when trimmed, is exactly "// v"
///   - A closing marker: a line that, when trimmed, is exactly "// ^"
/// Lines outside these markers are omitted (replaced by a placeholder).
pub fn filter_substring_markers(content: &str) -> String {
    let mut output = String::new();
    let mut in_block = false;
    let mut last_was_placeholder = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "// v" {
            if !last_was_placeholder {
                output.push_str("\n// ...\n");
                last_was_placeholder = true;
            }
            in_block = true;
            continue;
        }
        if trimmed == "// ^" {
            in_block = false;
            if !last_was_placeholder {
                output.push_str("\n// ...\n");
                last_was_placeholder = true;
            }
            continue;
        }
        if in_block {
            output.push_str(line);
            output.push('\n');
            last_was_placeholder = false;
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::filter_substring_markers;

    #[test]
    fn test_filter_substring_markers_basic() {
        let input = "\
Line before
// v
Inside block line 1
Inside block line 2
// ^
Line after";
        let expected = "\n// ...\nInside block line 1\nInside block line 2\n\n// ...\n";
        assert_eq!(filter_substring_markers(input), expected);
    }

    #[test]
    fn test_no_markers() {
        // When there are no markers, the function should return an empty string.
        let input = "This is a file with no markers.\nAnother line.";
        let expected = "";
        assert_eq!(filter_substring_markers(input), expected);
    }

    #[test]
    fn test_empty_markers() {
        // Test when markers are present but with no content between them.
        let input = "\
Header text
// v
// ^
Footer text";
        // Only the placeholder for the opening marker is added (the closing marker doesn't add another because last_was_placeholder remains true).
        let expected = "\n// ...\n";
        assert_eq!(filter_substring_markers(input), expected);
    }

    #[test]
    fn test_consecutive_markers() {
        // Test with consecutive opening and closing markers.
        let input = "\
Line before
// v
// v
Inside block content
// ^
 // ^
Line after";
        // The first "// v" adds the placeholder, the second is ignored (due to last_was_placeholder),
        // then "Inside block content" is output, and the first "// ^" adds the placeholder while the second is ignored.
        let expected = "\n// ...\nInside block content\n\n// ...\n";
        assert_eq!(filter_substring_markers(input), expected);
    }
}

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
    fn test_filter_substring_markers() {
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
}

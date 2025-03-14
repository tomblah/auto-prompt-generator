// crates/post_processing/src/lib.rs

/// Scrubs extra TODO markers from the given prompt if diff mode is not enabled.
///
/// This function takes a `primary_marker` parameter that must exactly match one of the lines in the prompt.
/// It removes all extra marker lines except:
///   - The first occurrence of a line exactly matching the supplied primary marker (if present), and
///   - The very last line that contains the marker substring.
/// If the primary marker isn’t found, an error is returned.
///
/// # Arguments
///
/// * `prompt` - The full prompt string to be processed.
/// * `diff_enabled` - Whether diff mode is active (in which case no scrubbing is done).
/// * `primary_marker` - The exact text of the primary TODO marker to preserve.
///
/// # Returns
///
/// A `Result` with the processed prompt as a `String` on success, or an error message if
/// the primary marker is not found.
pub fn scrub_extra_todo_markers(prompt: &str, diff_enabled: bool, primary_marker: &str) -> Result<String, String> {
    // If diff mode is enabled, do nothing.
    if diff_enabled {
        return Ok(prompt.to_string());
    }

    let marker = "// TODO: -";
    let lines: Vec<&str> = prompt.lines().collect();

    // Ensure that the primary marker exists in the prompt.
    let primary_found = lines.iter().any(|line| line.trim() == primary_marker);
    if !primary_found {
        return Err(format!("Primary marker '{}' not found in prompt", primary_marker));
    }

    // Find the index of the last line that contains the marker substring.
    let last_marker_index = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains(marker))
        .map(|(i, _)| i)
        .last()
        .ok_or_else(|| "No marker lines found in prompt".to_string())?;

    let mut output_lines = Vec::with_capacity(lines.len());
    let mut primary_marker_included = false;

    for (i, line) in lines.iter().enumerate() {
        if line.contains(marker) {
            if i == last_marker_index {
                // Always include the last marker line.
                output_lines.push(*line);
            } else if line.trim() == primary_marker && !primary_marker_included {
                // Include the first occurrence of the primary marker.
                output_lines.push(*line);
                primary_marker_included = true;
            }
            // Otherwise, skip this marker line.
        } else {
            output_lines.push(*line);
        }
    }

    Ok(output_lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preserves_last_marker() {
        let primary_marker = "// TODO: - Primary Marker";
        let input = r#"Line one
// TODO: - Primary Marker
Middle code
// TODO: - Extra Marker
End code
// TODO: - CTA Marker"#;
        let expected = r#"Line one
// TODO: - Primary Marker
Middle code
End code
// TODO: - CTA Marker"#;
        let output = scrub_extra_todo_markers(input, false, primary_marker).unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn test_error_when_primary_missing() {
        let primary_marker = "// TODO: - Primary Marker";
        let input = r#"Line one
// TODO: - Extra Marker
Middle code
// TODO: - CTA Marker"#;
        let result = scrub_extra_todo_markers(input, false, primary_marker);
        assert!(result.is_err());
    }

    #[test]
    fn test_diff_mode_no_scrub() {
        let primary_marker = "// TODO: - Primary Marker";
        let input = r#"Line one
// TODO: - Primary Marker
Middle code
// TODO: - Extra Marker
End code
// TODO: - CTA Marker"#;
        let output = scrub_extra_todo_markers(input, true, primary_marker).unwrap();
        // In diff mode the prompt is returned unmodified.
        assert_eq!(output, input);
    }
}

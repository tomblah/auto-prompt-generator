// crates/generate_prompt/src/prompt_validation.rs

use todo_marker::TODO_MARKER;

/// Validates that the given prompt contains the correct number of markers.
///
/// * When `diff_enabled` is **true**, the prompt must have **2 or 3** marker lines.
/// * When `diff_enabled` is **false**, the prompt must have **exactly 2** marker lines.
///
/// # Arguments
///
/// * `prompt`       – The final prompt as a string slice.
/// * `diff_enabled` – Whether diff mode is enabled.
///
/// # Returns
///
/// * `Ok(())` if the marker count is as expected.
/// * `Err(String)` with an explanatory message if the count is wrong.
pub fn validate_marker_count(prompt: &str, diff_enabled: bool) -> Result<(), String> {
    let marker = TODO_MARKER;

    // Collect every line containing the marker substring.
    let marker_lines: Vec<&str> = prompt
        .lines()
        .filter(|line| line.contains(marker))
        .collect();

    let count = marker_lines.len();

    if diff_enabled {
        if count != 2 && count != 3 {
            return Err(format!(
                "Expected 2 or 3 {} markers (with diff enabled), but found {}.",
                marker, count
            ));
        }
    } else if count != 2 {
        return Err(format!(
            "Expected exactly 2 {} markers, but found {}.",
            marker, count
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_marker_count_normal_valid() {
        let prompt = "// TODO: -\nSome code here\n// TODO: -";
        assert!(validate_marker_count(prompt, false).is_ok());
    }

    #[test]
    fn test_validate_marker_count_normal_invalid() {
        let prompt = "// TODO: -\nSome code here";
        assert!(validate_marker_count(prompt, false).is_err());
    }

    #[test]
    fn test_validate_marker_count_diff_enabled_valid_two() {
        let prompt = "// TODO: -\nSome code here\n// TODO: -";
        assert!(validate_marker_count(prompt, true).is_ok());
    }

    #[test]
    fn test_validate_marker_count_diff_enabled_valid_three() {
        let prompt = "// TODO: -\nSome code here\n// TODO: -\nExtra diff marker\n// TODO: -";
        assert!(validate_marker_count(prompt, true).is_ok());
    }

    #[test]
    fn test_validate_marker_count_diff_enabled_invalid() {
        let prompt = "// TODO: -\nSome code here";
        assert!(validate_marker_count(prompt, true).is_err());
    }
}

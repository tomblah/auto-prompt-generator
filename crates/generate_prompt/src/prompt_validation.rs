// crates/generate_prompt/src/prompt_validation.rs

/// Validates that the given prompt contains the correct number of markers.
/// - If diff mode is enabled, expects exactly 2 or 3 markers.
/// - Otherwise, expects exactly 2 markers.
///
/// # Arguments
///
/// * `prompt` - The final prompt as a string slice.
/// * `diff_enabled` - Whether diff mode is enabled.
///
/// # Returns
///
/// * `Ok(())` if the marker count is as expected.
/// * `Err(String)` with an error message if the marker count is incorrect.
pub fn validate_marker_count(prompt: &str, diff_enabled: bool) -> Result<(), String> {
    let marker = "// TODO: -";
    let marker_lines: Vec<&str> = prompt
        .lines()
        .filter(|line| line.contains(marker))
        .collect();

    if diff_enabled {
        if marker_lines.len() != 2 && marker_lines.len() != 3 {
            return Err(format!(
                "Expected 2 or 3 {} markers (with diff enabled), but found {}.",
                marker,
                marker_lines.len()
            ));
        }
    } else {
        if marker_lines.len() != 2 {
            return Err(format!(
                "Expected exactly 2 {} markers, but found {}.",
                marker,
                marker_lines.len()
            ));
        }
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

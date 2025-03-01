use std::path::Path;

/// Filters a list of file paths by excluding those whose basenames exactly match any of the provided exclusion patterns.
/// Also, lines that end with a slash (i.e. directories) are excluded.
///
/// # Arguments
///
/// * `lines` - A vector of file paths (each as a `String`).
/// * `exclusions` - A slice of strings representing exclusion patterns.
///
/// # Returns
///
/// A vector of file paths that are not excluded.
pub fn filter_excluded_files_lines(lines: Vec<String>, exclusions: &[String]) -> Vec<String> {
    lines
        .into_iter()
        .filter(|line| {
            let trimmed_line = line.trim();
            // Skip empty lines or lines ending with a slash.
            if trimmed_line.is_empty() || trimmed_line.ends_with('/') {
                return false;
            }
            let path = Path::new(trimmed_line);
            // Extract the basename.
            let basename = match path.file_name() {
                Some(name) => name.to_string_lossy().trim().to_string(),
                None => return false,
            };
            // Exclude if the basename exactly matches any provided pattern.
            !exclusions.iter().any(|pattern| &basename == pattern)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_exclusions() {
        let lines = vec![
            "/path/to/FileA.swift".to_string(),
            "/path/to/FileB.swift".to_string(),
            "/path/to/FileC.swift".to_string(),
        ];
        let exclusions: Vec<String> = vec![];
        let filtered = filter_excluded_files_lines(lines.clone(), &exclusions);
        assert_eq!(filtered, lines);
    }

    #[test]
    fn test_exclude_one() {
        let lines = vec![
            "/path/to/FileA.swift".to_string(),
            "/path/to/FileB.swift".to_string(),
            "/path/to/FileC.swift".to_string(),
        ];
        let exclusions = vec!["FileB.swift".to_string()];
        let filtered = filter_excluded_files_lines(lines, &exclusions);
        assert_eq!(
            filtered,
            vec![
                "/path/to/FileA.swift".to_string(),
                "/path/to/FileC.swift".to_string(),
            ]
        );
    }

    #[test]
    fn test_exclude_multiple() {
        let lines = vec![
            "/path/to/FileA.swift".to_string(),
            "/another/path/FileB.swift".to_string(),
            "/yet/another/path/FileC.swift".to_string(),
            "/different/FileD.swift".to_string(),
        ];
        let exclusions = vec!["FileB.swift".to_string(), "FileD.swift".to_string()];
        let filtered = filter_excluded_files_lines(lines, &exclusions);
        assert_eq!(
            filtered,
            vec![
                "/path/to/FileA.swift".to_string(),
                "/yet/another/path/FileC.swift".to_string(),
            ]
        );
    }

    #[test]
    fn test_empty_lines() {
        let lines = vec![
            "".to_string(),
            "   ".to_string(),
            "/path/to/FileA.swift".to_string(),
        ];
        let exclusions: Vec<String> = vec![];
        let filtered = filter_excluded_files_lines(lines, &exclusions);
        assert_eq!(filtered, vec!["/path/to/FileA.swift".to_string()]);
    }

    #[test]
    fn test_no_basename() {
        // Test lines that are directories or root, which should be excluded.
        let lines = vec![
            "/".to_string(),             // root, should be excluded
            "/path/to/".to_string(),       // directory with trailing slash, should be excluded
            "/path/to/FileA.swift".to_string(),
        ];
        let exclusions: Vec<String> = vec![];
        let filtered = filter_excluded_files_lines(lines, &exclusions);
        // Only the file with a valid basename should be included.
        assert_eq!(filtered, vec!["/path/to/FileA.swift".to_string()]);
    }
}

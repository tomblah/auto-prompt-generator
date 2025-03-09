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
        .filter_map(|line| {
            let trimmed_line = line.trim();
            if trimmed_line.is_empty() || trimmed_line.ends_with('/') {
                return None;
            }
            let path = std::path::Path::new(trimmed_line);
            let basename = match path.file_name() {
                Some(name) => name.to_string_lossy().trim().to_string(),
                None => return None,
            };
            if exclusions.iter().any(|pattern| &basename == pattern) {
                None
            } else {
                // Return the trimmed line.
                Some(trimmed_line.to_string())
            }
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
    
    #[test]
    fn test_whitespace_trimming() {
        let lines = vec![
            "   /path/to/FileA.swift  ".to_string(),
            "\t/path/to/FileB.swift\n".to_string(),
        ];
        let exclusions: Vec<String> = vec![];
        let filtered = filter_excluded_files_lines(lines, &exclusions);
        assert_eq!(
            filtered,
            vec![
                "/path/to/FileA.swift".to_string(),
                "/path/to/FileB.swift".to_string(),
            ]
        );
    }

    #[test]
    fn test_case_sensitivity() {
        // Expect exclusion to be case sensitive.
        let lines = vec![
            "/path/to/FileA.swift".to_string(),
            "/path/to/fileA.swift".to_string(),
        ];
        let exclusions = vec!["FileA.swift".to_string()];
        let filtered = filter_excluded_files_lines(lines, &exclusions);
        // Only the exact match ("FileA.swift") should be excluded.
        assert_eq!(
            filtered,
            vec!["/path/to/fileA.swift".to_string()]
        );
    }

    #[test]
    fn test_partial_match_not_excluded() {
        let lines = vec![
            "/path/to/FileA.swift".to_string(),
            "/path/to/FileB.swift".to_string(),
        ];
        let exclusions = vec!["File".to_string()]; // Should not match "FileA.swift" exactly.
        let filtered = filter_excluded_files_lines(lines.clone(), &exclusions);
        assert_eq!(filtered, lines);
    }

    #[test]
    fn test_multiple_trailing_slashes() {
        let lines = vec![
            "/path/to/FileA.swift///".to_string(),
            "/path/to/FileB.swift".to_string(),
        ];
        let exclusions: Vec<String> = vec![];
        // The first path ends with slashes so it should be filtered out.
        let filtered = filter_excluded_files_lines(lines, &exclusions);
        assert_eq!(filtered, vec!["/path/to/FileB.swift".to_string()]);
    }
    
    #[test]
    fn test_only_slash_after_trim() {
        // This will cover the branch where the trimmed line ends with '/'
        // and returns None immediately.
        let lines = vec![
            "   /path/to/directory/  ".to_string(),  // after trim(), still ends with '/'
        ];
        let exclusions: Vec<String> = vec![];
        let filtered = filter_excluded_files_lines(lines, &exclusions);
        // We expect an empty result because it ends with '/'
        assert_eq!(filtered, Vec::<String>::new());
    }

    #[test]
    fn test_no_file_name_at_all() {
        // This will cover the branch where path.file_name() is None.
        // On Unix-like paths, a trailing slash is one way to cause file_name() to return None,
        // but using just "/" also triggers it.
        let lines = vec![
            "/".to_string(),         // root directory => file_name() = None
            "/path/to/".to_string(), // trailing slash => file_name() = None
        ];
        let exclusions: Vec<String> = vec![];
        let filtered = filter_excluded_files_lines(lines, &exclusions);
        // Both should be excluded because they have no basename
        assert_eq!(filtered, Vec::<String>::new());
    }
}

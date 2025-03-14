use std::path::{Path, PathBuf};
use get_search_roots::get_search_roots;

/// Determines the search root directory for the prompt generation.
///
/// This function calls `get_search_roots` on the provided base directory and then
/// picks the candidate that is the deepest prefix of the TODO file path (i.e. the one with the most path components).
///
/// # Arguments
///
/// * `base_dir` - The base directory (typically the Git root).
/// * `file_path` - The path to the TODO instruction file.
///
/// # Returns
///
/// The chosen search root as a `PathBuf`.
pub fn determine_search_root(base_dir: &Path, file_path: &str) -> PathBuf {
    let candidate_roots = get_search_roots(base_dir)
        .unwrap_or_else(|_| vec![base_dir.to_path_buf()]);
    if candidate_roots.len() == 1 {
        candidate_roots[0].clone()
    } else {
        let todo_path = PathBuf::from(file_path);
        candidate_roots.into_iter()
            .filter(|p| todo_path.starts_with(p))
            // Choose the candidate with the maximum number of path components (i.e. the deepest one)
            .max_by_key(|p| p.components().count())
            .unwrap_or_else(|| base_dir.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_base_dir_is_swift_package() {
        // Create a temporary directory that acts as the base directory.
        let temp_dir = tempdir().expect("failed to create temp dir");
        let base_dir = temp_dir.path();

        // Create a Package.swift in the base directory.
        let package_path = base_dir.join("Package.swift");
        fs::write(&package_path, "package contents").expect("failed to write Package.swift");

        // Simulate a TODO file path inside the base directory.
        let file_path = base_dir.join("some_file.txt");

        // Since the base directory is itself a Swift package, get_search_roots should return only base_dir.
        let candidate = determine_search_root(base_dir, file_path.to_str().unwrap());
        assert_eq!(candidate, base_dir);
    }

    #[test]
    fn test_candidate_subdirectory_selected() {
        // Create a temporary directory that acts as the base directory.
        let temp_dir = tempdir().expect("failed to create temp dir");
        let base_dir = temp_dir.path();

        // Do NOT create Package.swift in the base directory.
        // Instead, create two subdirectories with Package.swift.
        let sub1 = base_dir.join("sub1");
        let sub2 = base_dir.join("sub2");
        fs::create_dir_all(&sub1).expect("failed to create sub1");
        fs::create_dir_all(&sub2).expect("failed to create sub2");

        // Create Package.swift in each subdirectory.
        fs::write(sub1.join("Package.swift"), "package contents")
            .expect("failed to write Package.swift in sub1");
        fs::write(sub2.join("Package.swift"), "package contents")
            .expect("failed to write Package.swift in sub2");

        // Simulate a TODO file path inside sub1.
        let todo_file = sub1.join("some_file.txt");
        let candidate = determine_search_root(base_dir, todo_file.to_str().unwrap());
        // We expect the candidate to be sub1 (the deeper candidate) rather than the base directory.
        assert_eq!(candidate, sub1);
    }
}

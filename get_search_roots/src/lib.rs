// rust/get_search_roots/src/lib.rs

use std::collections::BTreeSet;
use std::error::Error;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Returns a list of directories that are potential Swift package roots.
/// - If the provided root itself contains a "Package.swift", returns just that directory.
/// - Otherwise, returns the provided root (if its basename isn't ".build") along with
///   any subdirectories (excluding those under any ".build" directories) that contain "Package.swift".
pub fn get_search_roots(root: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    // Check that the provided root exists and is a directory.
    if !root.exists() || !root.is_dir() {
        return Err(format!("Error: '{}' is not a valid directory.", root.display()).into());
    }

    // If the root itself is a Swift package (contains Package.swift), return it.
    if root.join("Package.swift").is_file() {
        return Ok(vec![root.to_path_buf()]);
    }

    let mut found_dirs: BTreeSet<PathBuf> = BTreeSet::new();

    // Include the provided root unless its basename is ".build".
    if root.file_name().map_or(true, |name| name != ".build") {
        found_dirs.insert(root.to_path_buf());
    }

    // Recursively search for "Package.swift" files under the root, excluding those under ".build".
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() && entry.file_name() == "Package.swift" {
            if entry.path().components().any(|comp| comp.as_os_str() == ".build") {
                continue;
            }
            if let Some(parent) = entry.path().parent() {
                found_dirs.insert(parent.to_path_buf());
            }
        }
    }

    Ok(found_dirs.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;
    use tempfile::TempDir;

    // Test that if the provided root does NOT have a Package.swift but contains a subdirectory that does,
    // then get_search_roots returns both the repo root and the subpackage directory.
    #[test]
    fn test_returns_main_repo_and_subpackage() {
        let tmp_dir = TempDir::new().unwrap();
        let repo_path = tmp_dir.path();

        // Create a subdirectory that is a Swift package.
        let sub_pkg = repo_path.join("SubPackage");
        fs::create_dir_all(&sub_pkg).unwrap();
        File::create(sub_pkg.join("Package.swift")).unwrap();

        // Create a non-package subdirectory.
        let non_pkg = repo_path.join("NonPackage");
        fs::create_dir_all(&non_pkg).unwrap();
        fs::write(non_pkg.join("somefile.txt"), "just some text").unwrap();

        let roots = get_search_roots(repo_path).unwrap();
        let mut paths: Vec<String> = roots.into_iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        paths.sort();

        // Expect the repo root to be included.
        assert!(paths.iter().any(|s| s == repo_path.to_string_lossy().as_ref()));
        // Expect the SubPackage directory (which contains Package.swift) to be included.
        assert!(paths.iter().any(|s| s == sub_pkg.to_string_lossy().as_ref()));
        // The NonPackage directory should not be present.
        assert!(!paths.iter().any(|s| s == non_pkg.to_string_lossy().as_ref()));
    }

    // Test that when the provided root itself is a Swift package (contains Package.swift),
    // then only that directory is returned.
    #[test]
    fn test_returns_only_package_root_when_given_package_root() {
        let pkg_dir = TempDir::new().unwrap();
        let pkg_path = pkg_dir.path();

        File::create(pkg_path.join("Package.swift")).unwrap();
        let sub_pkg = pkg_path.join("SubPackage");
        fs::create_dir_all(&sub_pkg).unwrap();
        File::create(sub_pkg.join("Package.swift")).unwrap();

        let roots = get_search_roots(pkg_path).unwrap();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0], pkg_path);
    }

    // Test that directories under a .build directory are excluded.
    #[test]
    fn test_excludes_directories_under_build() {
        let tmp_dir = TempDir::new().unwrap();
        let repo_path = tmp_dir.path();

        let build_sub = repo_path.join(".build").join("ThirdParty");
        fs::create_dir_all(&build_sub).unwrap();
        File::create(build_sub.join("Package.swift")).unwrap();

        let valid_sub = repo_path.join("ValidPackage");
        fs::create_dir_all(&valid_sub).unwrap();
        File::create(valid_sub.join("Package.swift")).unwrap();

        let roots = get_search_roots(repo_path).unwrap();
        let paths: Vec<String> = roots
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        // ValidPackage should be included.
        assert!(paths.iter().any(|s| s == valid_sub.to_string_lossy().as_ref()));
        // None of the returned paths should include ".build".
        for p in paths {
            assert!(
                !p.contains("/.build/"),
                "Path {} should not contain .build",
                p
            );
        }
    }

    // Test that if the provided directory is a .build directory (and it does not contain Package.swift),
    // then get_search_roots returns an empty list.
    #[test]
    fn test_does_not_return_build_directory_itself() {
        let tmp_dir = TempDir::new().unwrap();
        let repo_path = tmp_dir.path();

        let build_dir = repo_path.join(".build");
        fs::create_dir_all(&build_dir).unwrap();

        let result = get_search_roots(&build_dir).unwrap();
        assert!(result.is_empty(), "Expected empty result for .build directory, got {:?}", result);
    }

    // Test that passing an invalid (non-existent) directory returns an error.
    #[test]
    fn test_invalid_directory() {
        let non_existent = Path::new("non_existent_directory");
        let result = get_search_roots(non_existent);
        assert!(result.is_err(), "Expected error for non-existent directory");
    }

    // --- Additional tests for increased coverage ---

    // Test that if no Package.swift is found anywhere, the function returns just the provided root.
    #[test]
    fn test_no_package_found() {
        let tmp_dir = TempDir::new().unwrap();
        let repo_path = tmp_dir.path();

        // Do not create any Package.swift files.
        let roots = get_search_roots(repo_path).unwrap();
        // Since repo_path is not named ".build", it should be included.
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0], repo_path);
    }

    // Test that providing a file (not a directory) returns an error.
    #[test]
    fn test_path_is_file() {
        let tmp_dir = TempDir::new().unwrap();
        let file_path = tmp_dir.path().join("not_a_directory.txt");
        File::create(&file_path).unwrap();

        let result = get_search_roots(&file_path);
        assert!(result.is_err(), "Expected error when passing a file instead of a directory");
    }

    // Test multiple nested package directories and check deduplication.
    #[test]
    fn test_multiple_nested_packages() {
        let tmp_dir = TempDir::new().unwrap();
        let repo_path = tmp_dir.path();

        // Create several nested directories that contain Package.swift files.
        let pkg_a = repo_path.join("A");
        fs::create_dir_all(&pkg_a).unwrap();
        File::create(pkg_a.join("Package.swift")).unwrap();

        let pkg_b = repo_path.join("B");
        fs::create_dir_all(&pkg_b).unwrap();
        File::create(pkg_b.join("Package.swift")).unwrap();

        let pkg_b_c = pkg_b.join("C");
        fs::create_dir_all(&pkg_b_c).unwrap();
        File::create(pkg_b_c.join("Package.swift")).unwrap();

        // The provided root does not contain a Package.swift, so it should be included,
        // along with all subdirectories containing Package.swift.
        let mut roots = get_search_roots(repo_path).unwrap();
        roots.sort();

        // We expect the following paths:
        // - repo_path (the root, because its basename isn't ".build")
        // - pkg_a
        // - pkg_b
        // - pkg_b_c
        assert_eq!(roots.len(), 4);
        let expected = vec![
            repo_path.to_path_buf(),
            pkg_a,
            pkg_b,
            pkg_b_c,
        ];
        // Sort both lists for comparison.
        let mut expected_sorted = expected.clone();
        expected_sorted.sort();
        assert_eq!(roots, expected_sorted);
    }
}

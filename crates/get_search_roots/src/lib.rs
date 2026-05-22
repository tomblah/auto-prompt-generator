// crates/get_search_roots/src/lib.rs

use anyhow::{anyhow, Result};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Returns a list of directories that are potential Swift package roots.
/// - If the provided root itself contains a "Package.swift", returns just that directory.
/// - Otherwise, returns the provided root (if its basename isn't ".build") along with
///   any subdirectories (excluding those under any ".build" directories) that contain "Package.swift".
pub fn get_search_roots(root: &Path) -> Result<Vec<PathBuf>> {
    if !root.exists() || !root.is_dir() {
        return Err(anyhow!(
            "Error: '{}' is not a valid directory.",
            root.display()
        ));
    }

    if root.join("Package.swift").is_file() {
        return Ok(vec![root.to_path_buf()]);
    }

    let mut found_dirs: BTreeSet<PathBuf> = BTreeSet::new();

    if root.file_name().is_none_or(|name| name != ".build") {
        found_dirs.insert(root.to_path_buf());
    }

    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() && entry.file_name() == "Package.swift" {
            if entry
                .path()
                .components()
                .any(|comp| comp.as_os_str() == ".build")
            {
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
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn test_returns_main_repo_and_subpackage() {
        let tmp_dir = TempDir::new().unwrap();
        let repo_path = tmp_dir.path();

        let sub_pkg = repo_path.join("SubPackage");
        fs::create_dir_all(&sub_pkg).unwrap();
        File::create(sub_pkg.join("Package.swift")).unwrap();

        let non_pkg = repo_path.join("NonPackage");
        fs::create_dir_all(&non_pkg).unwrap();
        fs::write(non_pkg.join("somefile.txt"), "just some text").unwrap();

        let roots = get_search_roots(repo_path).unwrap();
        let mut paths: Vec<String> = roots
            .into_iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        paths.sort();

        assert!(paths
            .iter()
            .any(|s| s == repo_path.to_string_lossy().as_ref()));
        assert!(paths
            .iter()
            .any(|s| s == sub_pkg.to_string_lossy().as_ref()));
        assert!(!paths
            .iter()
            .any(|s| s == non_pkg.to_string_lossy().as_ref()));
    }

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

        assert!(paths
            .iter()
            .any(|s| s == valid_sub.to_string_lossy().as_ref()));
        for p in paths {
            assert!(
                !p.contains("/.build/"),
                "Path {} should not contain .build",
                p
            );
        }
    }

    #[test]
    fn test_does_not_return_build_directory_itself() {
        let tmp_dir = TempDir::new().unwrap();
        let repo_path = tmp_dir.path();

        let build_dir = repo_path.join(".build");
        fs::create_dir_all(&build_dir).unwrap();

        let result = get_search_roots(&build_dir).unwrap();
        assert!(
            result.is_empty(),
            "Expected empty result for .build directory, got {:?}",
            result
        );
    }

    #[test]
    fn test_invalid_directory() {
        let non_existent = Path::new("non_existent_directory");
        let result = get_search_roots(non_existent);
        assert!(result.is_err(), "Expected error for non-existent directory");
    }

    #[test]
    fn test_no_package_found() {
        let tmp_dir = TempDir::new().unwrap();
        let repo_path = tmp_dir.path();

        let roots = get_search_roots(repo_path).unwrap();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0], repo_path);
    }

    #[test]
    fn test_path_is_file() {
        let tmp_dir = TempDir::new().unwrap();
        let file_path = tmp_dir.path().join("not_a_directory.txt");
        File::create(&file_path).unwrap();

        let result = get_search_roots(&file_path);
        assert!(
            result.is_err(),
            "Expected error when passing a file instead of a directory"
        );
    }

    #[test]
    fn test_multiple_nested_packages() {
        let tmp_dir = TempDir::new().unwrap();
        let repo_path = tmp_dir.path();

        let pkg_a = repo_path.join("A");
        fs::create_dir_all(&pkg_a).unwrap();
        File::create(pkg_a.join("Package.swift")).unwrap();

        let pkg_b = repo_path.join("B");
        fs::create_dir_all(&pkg_b).unwrap();
        File::create(pkg_b.join("Package.swift")).unwrap();

        let pkg_b_c = pkg_b.join("C");
        fs::create_dir_all(&pkg_b_c).unwrap();
        File::create(pkg_b_c.join("Package.swift")).unwrap();

        let mut roots = get_search_roots(repo_path).unwrap();
        roots.sort();

        assert_eq!(roots.len(), 4);
        let expected = vec![repo_path.to_path_buf(), pkg_a, pkg_b, pkg_b_c];
        let mut expected_sorted = expected.clone();
        expected_sorted.sort();
        assert_eq!(roots, expected_sorted);
    }
}

// rust/find_definition_files/src/lib.rs

use regex::Regex;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// Helper function: checks for allowed file extensions.
fn allowed_extension(path: &Path) -> bool {
    match path.extension().and_then(|s| s.to_str()) {
        Some(ext) => {
            let ext_lower = ext.to_lowercase();
            ext_lower == "swift" || ext_lower == "h" || ext_lower == "m" || ext_lower == "js"
        }
        None => false,
    }
}

// Helper function: returns true if any path component is ".build" or "Pods".
fn file_in_excluded_dir(path: &Path) -> bool {
    path.components().any(|c| {
        let s = c.as_os_str().to_string_lossy();
        s == ".build" || s == "Pods"
    })
}

// Helper: Given a root directory, returns a vector of search roots.
fn get_search_roots(root: &Path) -> Vec<PathBuf> {
    let mut roots = BTreeSet::new();
    if root.join("Package.swift").is_file() {
        roots.insert(root.to_path_buf());
    } else {
        if root.file_name().map(|s| s != ".build").unwrap_or(true) {
            roots.insert(root.to_path_buf());
        }
        for entry in WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file() && e.file_name() == "Package.swift")
        {
            if entry.path().components().any(|c| c.as_os_str() == ".build") {
                continue;
            }
            if let Some(parent) = entry.path().parent() {
                roots.insert(parent.to_path_buf());
            }
        }
    }
    roots.into_iter().collect()
}

/// Public function: Given a types file and a root directory, scans for files containing definitions
/// for any of the types listed. Returns a sorted set of matching file paths.
pub fn find_definition_files(
    types_file: &Path,
    root: &Path,
) -> Result<BTreeSet<PathBuf>, Box<dyn std::error::Error>> {
    let types_content = fs::read_to_string(types_file)?;
    let types: Vec<String> = types_content
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if types.is_empty() {
        return Err("No types found in the types file.".into());
    }
    let types_regex = types.join("|");
    let pattern = format!(
        r"\b(?:class|struct|enum|protocol|typealias)\s+(?:{})\b",
        types_regex
    );
    let re = Regex::new(&pattern)?;

    let search_roots = get_search_roots(root);
    let mut found_files = BTreeSet::new();

    for sr in search_roots {
        for entry in WalkDir::new(&sr)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if !allowed_extension(path) || file_in_excluded_dir(path) {
                continue;
            }
            if let Ok(content) = fs::read_to_string(path) {
                if re.is_match(&content) {
                    found_files.insert(path.to_path_buf());
                }
            }
        }
    }
    Ok(found_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn test_allowed_extension() {
        assert!(allowed_extension(Path::new("test.swift")));
        assert!(allowed_extension(Path::new("test.h")));
        assert!(allowed_extension(Path::new("test.m")));
        assert!(allowed_extension(Path::new("test.js")));
        assert!(!allowed_extension(Path::new("test.txt")));
    }

    #[test]
    fn test_file_in_excluded_dir() {
        let path1 = Path::new("/home/user/Pods/file.swift");
        let path2 = Path::new("/home/user/.build/file.swift");
        let path3 = Path::new("/home/user/src/file.swift");
        assert!(file_in_excluded_dir(path1));
        assert!(file_in_excluded_dir(path2));
        assert!(!file_in_excluded_dir(path3));
    }

    #[test]
    fn test_get_search_roots_when_root_is_package() {
        let dir = tempdir().unwrap();
        // Create a Package.swift file in the temporary directory.
        let package_path = dir.path().join("Package.swift");
        fs::write(&package_path, "swift package content").unwrap();

        let roots = get_search_roots(dir.path());
        // When the root is a Swift package, get_search_roots should return only the root.
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0], dir.path());
    }

    #[test]
    fn test_find_definition_files() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create a types file containing a type name.
        let types_file_path = root.join("types.txt");
        fs::write(&types_file_path, "MyType\n").unwrap();

        // Create a file that contains a valid definition: "class MyType"
        let good_file_path = root.join("good.swift");
        fs::write(&good_file_path, "import Foundation\nclass MyType {}\n").unwrap();

        // Create a file that does not contain any matching definition.
        let bad_file_path = root.join("bad.swift");
        fs::write(&bad_file_path, "import Foundation\n// no definitions here\n").unwrap();

        // Create a file inside an excluded directory ("Pods").
        let excluded_dir = root.join("Pods");
        fs::create_dir_all(&excluded_dir).unwrap();
        let excluded_file_path = excluded_dir.join("excluded.swift");
        fs::write(&excluded_file_path, "class MyType {}\n").unwrap();

        let found = find_definition_files(&types_file_path, root).expect("Should succeed");

        // Only the good_file should be detected.
        assert!(found.contains(&good_file_path));
        assert!(!found.contains(&bad_file_path));
        assert!(!found.contains(&excluded_file_path));
    }
}

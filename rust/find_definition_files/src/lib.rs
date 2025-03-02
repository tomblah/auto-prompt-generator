// rust/find_definition_files/src/lib.rs

use regex::Regex;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// A helper struct that encapsulates the information needed to find definition files.
struct DefinitionFinder {
    regex: Regex,
    search_roots: Vec<PathBuf>,
}

impl DefinitionFinder {
    /// Constructs a new `DefinitionFinder` from the given types file and root directory.
    fn new(types_file: &Path, root: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        // Read and parse the types file.
        let types_content = fs::read_to_string(types_file)?;
        let types: Vec<String> = types_content
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        if types.is_empty() {
            return Err("No types found in the types file.".into());
        }
        // Build the regex to match definitions like "class MyType", "struct MyType", etc.
        let types_regex = types.join("|");
        let pattern = format!(
            r"\b(?:class|struct|enum|protocol|typealias)\s+(?:{})\b",
            types_regex
        );
        let regex = Regex::new(&pattern)?;

        // Determine the search roots.
        let search_roots = Self::get_search_roots(root);

        Ok(Self { regex, search_roots })
    }

    /// Returns a vector of search roots given a starting directory.
    /// If the root itself contains a "Package.swift", returns only that directory.
    /// Otherwise, returns the root (if not named ".build") and all subdirectories
    /// containing "Package.swift", skipping any under a ".build" directory.
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

    /// Returns true if the file has one of the allowed extensions.
    fn allowed_extension(path: &Path) -> bool {
        path.extension()
            .and_then(|s| s.to_str())
            .map(|ext| {
                let ext_lower = ext.to_lowercase();
                ext_lower == "swift" || ext_lower == "h" || ext_lower == "m" || ext_lower == "js"
            })
            .unwrap_or(false)
    }

    /// Returns true if any component of the path is named ".build" or "Pods".
    fn file_in_excluded_dir(path: &Path) -> bool {
        path.components().any(|c| {
            let s = c.as_os_str().to_string_lossy();
            s == ".build" || s == "Pods"
        })
    }

    /// Scans all search roots for files whose allowed extension is valid and which are not in
    /// an excluded directory, then checks if their content matches the definition regex.
    fn find_files(&self) -> BTreeSet<PathBuf> {
        let mut found_files = BTreeSet::new();
        for sr in &self.search_roots {
            for entry in WalkDir::new(sr)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();
                if !Self::allowed_extension(path) || Self::file_in_excluded_dir(path) {
                    continue;
                }
                if let Ok(content) = fs::read_to_string(path) {
                    if self.regex.is_match(&content) {
                        found_files.insert(path.to_path_buf());
                    }
                }
            }
        }
        found_files
    }
}

/// Public function: given a types file and a root directory,
/// scans for files that contain definitions for any of the types listed in the types file.
/// Returns a sorted set of matching file paths.
pub fn find_definition_files(
    types_file: &Path,
    root: &Path,
) -> Result<BTreeSet<PathBuf>, Box<dyn std::error::Error>> {
    let finder = DefinitionFinder::new(types_file, root)?;
    Ok(finder.find_files())
}

/// The following free functions are re-exported for testing purposes so that the tests
/// can continue to call them as before without any changes.
#[cfg(test)]
pub fn allowed_extension(path: &Path) -> bool {
    DefinitionFinder::allowed_extension(path)
}

#[cfg(test)]
pub fn file_in_excluded_dir(path: &Path) -> bool {
    DefinitionFinder::file_in_excluded_dir(path)
}

#[cfg(test)]
pub fn get_search_roots(root: &Path) -> Vec<PathBuf> {
    DefinitionFinder::get_search_roots(root)
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
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
        let path1 = PathBuf::from("/home/user/Pods/file.swift");
        let path2 = PathBuf::from("/home/user/.build/file.swift");
        let path3 = PathBuf::from("/home/user/src/file.swift");
        assert!(file_in_excluded_dir(&path1));
        assert!(file_in_excluded_dir(&path2));
        assert!(!file_in_excluded_dir(&path3));
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
    fn test_find_definition_files_basic() {
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

    // --- Converted tests from bats ---

    #[test]
    fn test_excludes_build_directory() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create a Swift file in a normal directory.
        let sources_dir = root.join("Sources");
        fs::create_dir_all(&sources_dir).unwrap();
        let normal_file = sources_dir.join("MyType.swift");
        fs::write(&normal_file, "class MyType {}\n").unwrap();

        // Create a Swift file in a .build directory.
        let build_dir = root.join(".build/somepath");
        fs::create_dir_all(&build_dir).unwrap();
        let build_file = build_dir.join("MyType.swift");
        fs::write(&build_file, "class MyType {}\n").unwrap();

        // Create a types file listing the type "MyType".
        let types_file = root.join("types.txt");
        fs::write(&types_file, "MyType\n").unwrap();

        let found = find_definition_files(&types_file, root).expect("find_definition_files failed");

        // The result should include the file in Sources but not the one in .build.
        assert!(found.contains(&normal_file));
        assert!(!found.contains(&build_file));
    }

    #[test]
    fn test_deduplicated_files_combined_regex() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create a directory "Combined" with multiple files.
        let combined_dir = root.join("Combined");
        fs::create_dir_all(&combined_dir).unwrap();

        // File with both TypeOne and TypeTwo definitions.
        let both_file = combined_dir.join("BothTypes.swift");
        fs::write(&both_file, "class TypeOne {}\nstruct TypeTwo {}\n").unwrap();

        // File with only TypeOne definition.
        let only_file = combined_dir.join("OnlyTypeOne.swift");
        fs::write(&only_file, "enum TypeOne {}\n").unwrap();

        // File with an unrelated definition.
        let other_file = combined_dir.join("Other.swift");
        fs::write(&other_file, "protocol OtherType {}\n").unwrap();

        // Create a types file with both TypeOne and TypeTwo.
        let types_file = combined_dir.join("new_types.txt");
        fs::write(&types_file, "TypeOne\nTypeTwo\n").unwrap();

        let found = find_definition_files(&types_file, &combined_dir).expect("find_definition_files failed");

        // Expect BothTypes.swift and OnlyTypeOne.swift to be found, but not Other.swift.
        assert!(found.contains(&both_file));
        assert!(found.contains(&only_file));
        assert!(!found.contains(&other_file));

        // Since the return type is a BTreeSet, files are deduplicated.
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn test_excludes_pods_directory() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create a Swift file in the Sources directory.
        let sources_dir = root.join("Sources");
        fs::create_dir_all(&sources_dir).unwrap();
        let source_file = sources_dir.join("MyType.swift");
        fs::write(&source_file, "class MyType {}\n").unwrap();

        // Create a Swift file in the Pods directory.
        let pods_dir = root.join("Pods");
        fs::create_dir_all(&pods_dir).unwrap();
        let pods_file = pods_dir.join("MyType.swift");
        fs::write(&pods_file, "class MyType {}\n").unwrap();

        // Create a types file.
        let types_file = root.join("types.txt");
        fs::write(&types_file, "MyType\n").unwrap();

        let found = find_definition_files(&types_file, root).expect("find_definition_files failed");

        // The result should include the file in Sources but not the one in Pods.
        assert!(found.contains(&source_file));
        assert!(!found.contains(&pods_file));
    }

    #[test]
    fn test_empty_when_only_pods_exist() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Only create files in the Pods directory.
        let pods_dir = root.join("Pods/SubModule");
        fs::create_dir_all(&pods_dir).unwrap();
        let pods_file = pods_dir.join("MyType.swift");
        fs::write(&pods_file, "class MyType {}\n").unwrap();

        // Create a types file.
        let types_file = root.join("types.txt");
        fs::write(&types_file, "MyType\n").unwrap();

        let found = find_definition_files(&types_file, root).expect("find_definition_files failed");

        // Expect no files to be found.
        assert!(found.is_empty());
    }

    #[test]
    fn test_includes_objc_files() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create a directory for Objective-C files.
        let objc_dir = root.join("ObjC");
        fs::create_dir_all(&objc_dir).unwrap();
        let header_file = objc_dir.join("MyType.h");
        let impl_file = objc_dir.join("MyType.m");
        fs::write(&header_file, "class MyType { }").unwrap();
        fs::write(&impl_file, "class MyType { }").unwrap();

        // Create a types file.
        let types_file = root.join("types.txt");
        fs::write(&types_file, "MyType\n").unwrap();

        let found = find_definition_files(&types_file, root).expect("find_definition_files failed");

        // Both the header and implementation files should be included.
        assert!(found.contains(&header_file));
        assert!(found.contains(&impl_file));
    }
    
    #[test]
    fn test_missing_types_file() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let missing_types_file = root.join("nonexistent.txt");

        // Since the types file does not exist, we expect an error.
        let result = find_definition_files(&missing_types_file, root);
        assert!(result.is_err(), "Expected error when types file is missing");
    }

    #[test]
    fn test_empty_types_file() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let empty_types_file = root.join("empty.txt");
        fs::write(&empty_types_file, "").unwrap();

        // The function should error out if no types are found.
        let result = find_definition_files(&empty_types_file, root);
        assert!(result.is_err(), "Expected error when types file is empty");
    }

    #[test]
    fn test_non_swift_file_ignored() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create a types file containing the type name.
        let types_file = root.join("types.txt");
        fs::write(&types_file, "MyType\n").unwrap();

        // Create a file with a valid definition but with a .txt extension.
        let non_swift_file = root.join("definition.txt");
        fs::write(&non_swift_file, "class MyType {}\n").unwrap();

        let found = find_definition_files(&types_file, root).expect("find_definition_files failed");

        // The non-Swift file should not be included.
        assert!(!found.contains(&non_swift_file));
    }

    #[test]
    fn test_case_sensitivity() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create a types file with "MyType".
        let types_file = root.join("types.txt");
        fs::write(&types_file, "MyType\n").unwrap();

        // Create one file with the correct case.
        let correct_case = root.join("correct.swift");
        fs::write(&correct_case, "class MyType {}\n").unwrap();

        // Create another file with a lower-case variant.
        let wrong_case = root.join("wrong.swift");
        fs::write(&wrong_case, "class mytype {}\n").unwrap();

        let found = find_definition_files(&types_file, root).expect("find_definition_files failed");

        // Only the file with the correct case should be included.
        assert!(found.contains(&correct_case));
        assert!(!found.contains(&wrong_case));
    }

    #[test]
    fn test_duplicate_definitions_deduplicated() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create a types file.
        let types_file = root.join("types.txt");
        fs::write(&types_file, "MyType\n").unwrap();

        // Create a file that contains two definitions of "MyType".
        let dup_file = root.join("dup.swift");
        fs::write(&dup_file, "class MyType {}\nclass MyType {}\n").unwrap();

        let found = find_definition_files(&types_file, root).expect("find_definition_files failed");

        // The file should appear only once.
        assert_eq!(found.iter().filter(|&p| *p == dup_file).count(), 1);
    }

    #[test]
    fn test_no_matching_definitions() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create a types file with a type name.
        let types_file = root.join("types.txt");
        fs::write(&types_file, "NonExistentType\n").unwrap();

        // Create a Swift file that does not contain the definition.
        let non_match = root.join("non_match.swift");
        fs::write(&non_match, "class SomeOtherType {}\n").unwrap();

        let found = find_definition_files(&types_file, root).expect("find_definition_files failed");

        // Expect an empty set if nothing matches.
        assert!(found.is_empty());
    }
    
    // Test that when the provided root is not a package but one or more subdirectories contain a Package.swift,
    // get_search_roots returns both the root (if its basename isn’t ".build") and each subdirectory.
    #[test]
    fn test_get_search_roots_with_subpackages() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create a subdirectory "SubPackage" that contains a Package.swift file.
        let subpackage = root.join("SubPackage");
        fs::create_dir_all(&subpackage).unwrap();
        fs::write(&subpackage.join("Package.swift"), "swift package content").unwrap();

        let roots = get_search_roots(root);
        // Should include both the root and the subpackage directory.
        assert!(roots.contains(&root.to_path_buf()));
        assert!(roots.contains(&subpackage));
        assert_eq!(roots.len(), 2);
    }

    // Test that JavaScript (.js) files are included if they contain a valid definition.
    #[test]
    fn test_includes_javascript_files() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create a JavaScript file with a valid class definition.
        let js_file = root.join("script.js");
        fs::write(&js_file, "class MyType {}").unwrap();

        // Create a types file that contains "MyType".
        let types_file = root.join("types.txt");
        fs::write(&types_file, "MyType\n").unwrap();

        let found = find_definition_files(&types_file, root).expect("find_definition_files failed");
        assert!(found.contains(&js_file));
    }

    // Test that definitions using keywords like "protocol" and "typealias" are matched.
    #[test]
    fn test_additional_definition_keywords() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create a file with a protocol definition.
        let protocol_file = root.join("protocol.swift");
        fs::write(&protocol_file, "protocol MyProtocol {}").unwrap();

        // Create a file with a typealias definition.
        let typealias_file = root.join("typealias.swift");
        fs::write(&typealias_file, "typealias MyAlias = Int").unwrap();

        // Create a types file with both keywords.
        let types_file = root.join("types.txt");
        fs::write(&types_file, "MyProtocol\nMyAlias\n").unwrap();

        let found = find_definition_files(&types_file, root).expect("find_definition_files failed");
        assert!(found.contains(&protocol_file));
        assert!(found.contains(&typealias_file));
    }

    // Test that unreadable files are skipped.
    #[test]
    #[cfg(unix)]
    fn test_unreadable_files() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create a file that contains a valid definition.
        let unreadable_file = root.join("unreadable.swift");
        fs::write(&unreadable_file, "class MyType {}").unwrap();

        // Remove read permission.
        let mut perms = fs::metadata(&unreadable_file).unwrap().permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&unreadable_file, perms).unwrap();

        // Create a types file.
        let types_file = root.join("types.txt");
        fs::write(&types_file, "MyType\n").unwrap();

        let found = find_definition_files(&types_file, root).expect("find_definition_files failed");
        // The unreadable file should be skipped.
        assert!(!found.contains(&unreadable_file));

        // Restore permissions so that the temporary file can be cleaned up.
        let mut perms = fs::metadata(&unreadable_file).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&unreadable_file, perms).unwrap();
    }
}

#[cfg(test)]
mod internal_tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;
    use std::path::Path;

    #[test]
    fn test_definition_finder_new_error_on_empty_types() {
        let dir = tempdir().unwrap();
        let types_path = dir.path().join("empty_types.txt");
        fs::write(&types_path, "").unwrap();

        // Attempting to create a DefinitionFinder with an empty types file should error.
        let result = DefinitionFinder::new(&types_path, dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_definition_finder_new_valid() {
        let dir = tempdir().unwrap();
        let types_path = dir.path().join("types.txt");
        fs::write(&types_path, "MyType\n").unwrap();

        // Create a dummy file to ensure get_search_roots returns something.
        let dummy_file = dir.path().join("dummy.swift");
        fs::write(&dummy_file, "class MyType {}\n").unwrap();

        let finder = DefinitionFinder::new(&types_path, dir.path());
        assert!(finder.is_ok());
        let finder = finder.unwrap();
        // The regex should match the definition in our dummy file.
        assert!(finder.regex.is_match("class MyType {}"));
    }

    #[test]
    fn test_definition_finder_find_files() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create a types file.
        let types_path = root.join("types.txt");
        fs::write(&types_path, "MyType\n").unwrap();

        // Create a valid file.
        let valid_file = root.join("valid.swift");
        fs::write(&valid_file, "class MyType {}\n").unwrap();

        // Create an invalid file (should not match).
        let invalid_file = root.join("invalid.swift");
        fs::write(&invalid_file, "class NotMyType {}\n").unwrap();

        // Create a file in an excluded directory.
        let pods_dir = root.join("Pods");
        fs::create_dir_all(&pods_dir).unwrap();
        let excluded_file = pods_dir.join("excluded.swift");
        fs::write(&excluded_file, "class MyType {}\n").unwrap();

        // Build the finder and get found files.
        let finder = DefinitionFinder::new(&types_path, root).unwrap();
        let found = finder.find_files();

        // Only valid_file should be found.
        assert!(found.contains(&valid_file));
        assert!(!found.contains(&invalid_file));
        assert!(!found.contains(&excluded_file));
    }

    #[test]
    fn test_get_search_roots_multiple_subpackages() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create two subdirectories with a Package.swift file.
        let sub1 = root.join("Sub1");
        fs::create_dir_all(&sub1).unwrap();
        fs::write(&sub1.join("Package.swift"), "content").unwrap();

        let sub2 = root.join("Sub2");
        fs::create_dir_all(&sub2).unwrap();
        fs::write(&sub2.join("Package.swift"), "content").unwrap();

        let roots = DefinitionFinder::get_search_roots(root);
        // Expect the root itself (if its not named ".build") and the two subpackages.
        assert!(roots.contains(&root.to_path_buf()));
        assert!(roots.contains(&sub1));
        assert!(roots.contains(&sub2));
        assert_eq!(roots.len(), 3);
    }
}

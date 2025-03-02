use regex::Regex;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Returns true if the file has an allowed extension.
fn allowed_extension(path: &Path) -> bool {
    match path.extension().and_then(|s| s.to_str()) {
        Some(ext) => {
            let ext_lower = ext.to_lowercase();
            ext_lower == "swift" || ext_lower == "h" || ext_lower == "m" || ext_lower == "js"
        }
        None => false,
    }
}

/// Returns true if any component of the path is named ".build" or "Pods".
fn file_in_excluded_dir(path: &Path) -> bool {
    path.components().any(|c| {
        let s = c.as_os_str().to_string_lossy();
        s == ".build" || s == "Pods"
    })
}

/// Given a root directory, returns a vector of search roots:
/// - If the provided root itself contains a "Package.swift", returns only that directory.
/// - Otherwise, returns the root (if not named ".build") and all subdirectories containing "Package.swift",
///   skipping any that lie under a ".build" directory.
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

/// Core function: given a types file and a root directory,
/// scans for files that contain definitions for any of the types listed.
/// Returns a sorted set of matching file paths.
///
/// The return type now uses `Box<dyn std::error::Error + Send + Sync + 'static>` to ensure that
/// the error type satisfies thread-safety bounds.
pub fn find_definition_files(
    types_file: &Path,
    root: &Path,
) -> Result<BTreeSet<PathBuf>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    // Read the types file.
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

    // Build a regex that matches definitions like:
    // "class MyType", "struct MyType", "enum MyType", etc.
    let pattern = format!(
        r"\b(?:class|struct|enum|protocol|typealias)\s+(?:{})\b",
        types_regex
    );
    let re = Regex::new(&pattern)?;

    let search_roots = get_search_roots(root);

    let mut found_files = BTreeSet::new();
    // For each search root, traverse files.
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
    use std::path::Path;

    #[test]
    fn test_find_definition_files() {
        // Create a temporary file containing type names.
        let types_file = "test_types.txt";
        fs::write(types_file, "MyType\nAnotherType").unwrap();

        // Use the current directory as the search root.
        let search_root = Path::new(".");

        // Call the function.
        let result = find_definition_files(Path::new(types_file), search_root)
            .expect("Failed to find definition files");

        // In this simple test we expect at least one file (this file) to be returned.
        // Adjust this based on your actual project files.
        assert!(!result.is_empty());

        // Clean up the temporary file.
        fs::remove_file(types_file).unwrap();
    }
}

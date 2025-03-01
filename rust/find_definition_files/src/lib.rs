// rust/find_definition_files/src/lib.rs

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

/// Returns search roots from the given `root` directory.
/// If the provided root itself contains a "Package.swift", returns only that directory.
/// Otherwise, returns the root (if its basename isn’t ".build") plus any subdirectories
/// containing "Package.swift", skipping those under a ".build" directory.
pub fn get_search_roots(root: &Path) -> Vec<PathBuf> {
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
            if entry
                .path()
                .components()
                .any(|c| c.as_os_str() == ".build")
            {
                continue;
            }
            if let Some(parent) = entry.path().parent() {
                roots.insert(parent.to_path_buf());
            }
        }
    }
    roots.into_iter().collect()
}

/// Core function that scans for files with definitions of any types
/// listed in the given types file, starting from `root`.
pub fn find_definition_files(
    types_file: &Path,
    root: &Path,
) -> Result<BTreeSet<PathBuf>, Box<dyn std::error::Error>> {
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

    // Build a regex that matches definitions like "class MyType", "struct MyType", etc.
    let pattern = format!(
        r"\b(?:class|struct|enum|protocol|typealias)\s+(?:{})\b",
        types_regex
    );
    let re = Regex::new(&pattern)?;

    let search_roots = get_search_roots(root);

    let mut found_files = BTreeSet::new();

    // Traverse each search root.
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

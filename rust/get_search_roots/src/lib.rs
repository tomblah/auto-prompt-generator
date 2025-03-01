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

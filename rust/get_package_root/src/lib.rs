// src/lib.rs

use std::path::{Path, PathBuf};

/// Starting from `start_path`, traverse upward until a directory containing
/// "Package.swift" is found. If found, returns that directory as a `PathBuf`;
/// otherwise returns `None`.
pub fn get_package_root(start_path: &Path) -> Option<PathBuf> {
    // If start_path is a file, use its parent; if it's already a directory, use it.
    let mut current_dir: PathBuf = if start_path.is_dir() {
        start_path.to_path_buf()
    } else {
        start_path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("/"))
    };

    // Walk upward until we reach the root.
    while current_dir.as_os_str() != "/" {
        if current_dir.join("Package.swift").is_file() {
            return Some(current_dir);
        }
        if let Some(parent) = current_dir.parent() {
            current_dir = parent.to_path_buf();
        } else {
            break;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::tempdir;

    #[test]
    fn returns_package_root_if_found() {
        let dir = tempdir().unwrap();
        let pkg_dir = dir.path().join("MyPackage");
        fs::create_dir(&pkg_dir).unwrap();
        // Create a dummy Package.swift file.
        File::create(pkg_dir.join("Package.swift")).unwrap();
        let found = get_package_root(&pkg_dir.join("SomeFile.swift"));
        assert_eq!(found.unwrap(), pkg_dir);
    }

    #[test]
    fn returns_none_if_not_found() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("SomeFile.swift");
        File::create(&file).unwrap();
        assert!(get_package_root(&file).is_none());
    }
}

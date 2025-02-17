// src/main.rs
//
// This program returns a list of directories that are potential Swift package roots.
// If the provided root itself contains a "Package.swift" file, it prints just that directory.
// Otherwise, it prints the provided root (if it isn’t named ".build") and then searches recursively
// for any subdirectories containing "Package.swift", excluding those under any ".build" directories.
//
// Usage: get_search_roots <git_root_or_package_root>
//
// To use this in your workspace, add the "walkdir" crate to your Cargo.toml dependencies.

use std::collections::BTreeSet;
use std::env;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Ensure exactly one argument is provided.
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <git_root_or_package_root>", args[0]);
        std::process::exit(1);
    }
    let root_str = &args[1];
    let root = Path::new(root_str);

    // Check that the provided root exists and is a directory.
    if !root.exists() || !root.is_dir() {
        eprintln!("Error: '{}' is not a valid directory.", root_str);
        std::process::exit(1);
    }

    // If the root itself is a Swift package (contains Package.swift), print it and exit.
    if root.join("Package.swift").is_file() {
        println!("{}", root.display());
        return Ok(());
    }

    // Use a BTreeSet to collect unique directories in sorted order.
    let mut found_dirs: BTreeSet<PathBuf> = BTreeSet::new();

    // Include the provided root unless its basename is ".build".
    if root.file_name().map_or(true, |name| name != ".build") {
        found_dirs.insert(root.to_path_buf());
    }

    // Recursively search for "Package.swift" files under the root.
    // Exclude any file whose path contains a directory named ".build".
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

    // Print each found directory (one per line).
    for dir in found_dirs {
        println!("{}", dir.display());
    }

    Ok(())
}

// crates/lang_support/tests/source_files.rs

use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::PermissionsExt;

use lang_support::walk_source_files;
use tempfile::tempdir;

#[test]
#[cfg(unix)]
fn walk_source_files_filters_supported_readable_sources() {
    let dir = tempdir().expect("Failed to create temp dir");
    let root = dir.path();

    fs::write(root.join("model.swift"), "class Model {}\n").expect("Failed to write Swift file");
    fs::write(root.join("view.jsx"), "class View {}\n").expect("Failed to write JSX file");
    fs::write(root.join("notes.txt"), "class Ignored {}\n").expect("Failed to write text file");

    let build_dir = root.join(".build");
    fs::create_dir(&build_dir).expect("Failed to create .build dir");
    fs::write(build_dir.join("Generated.swift"), "class Generated {}\n")
        .expect("Failed to write generated file");

    let pods_dir = root.join("Pods");
    fs::create_dir(&pods_dir).expect("Failed to create Pods dir");
    fs::write(pods_dir.join("Vendor.m"), "@interface Vendor\n@end\n")
        .expect("Failed to write vendor file");

    let unreadable_path = root.join("Unreadable.swift");
    fs::write(&unreadable_path, "class Unreadable {}\n").expect("Failed to write unreadable file");
    let original_permissions = fs::metadata(&unreadable_path)
        .expect("Failed to read unreadable metadata")
        .permissions();
    let mut unreadable_permissions = original_permissions.clone();
    unreadable_permissions.set_mode(0o000);
    fs::set_permissions(&unreadable_path, unreadable_permissions)
        .expect("Failed to remove read permissions");

    let found: BTreeSet<_> = walk_source_files(root)
        .into_iter()
        .map(|source_file| {
            source_file
                .path
                .file_name()
                .expect("Expected filename")
                .to_string_lossy()
                .into_owned()
        })
        .collect();

    fs::set_permissions(&unreadable_path, original_permissions)
        .expect("Failed to restore read permissions");

    assert_eq!(
        found,
        BTreeSet::from(["model.swift".to_string(), "view.jsx".to_string()])
    );
}

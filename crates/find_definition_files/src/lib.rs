// crates/find_definition_files/src/lib.rs

use anyhow::Result;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use get_search_roots::get_search_roots;
use lang_support::{walk_source_files, SourceFile};

/// ---------------------------------------------------------------------------
///  DefinitionFinder
/// ---------------------------------------------------------------------------
pub struct DefinitionFinder {
    types: Vec<String>,
    search_roots: Vec<PathBuf>,
}

impl DefinitionFinder {
    pub fn new(types: &BTreeSet<String>, root: &Path) -> Result<Self> {
        let search_roots = get_search_roots(root)?;
        Ok(Self {
            types: types.iter().cloned().collect(),
            search_roots,
        })
    }

    /// Walk every search‑root and collect files that define any wanted type.
    pub fn find_files(&self) -> BTreeSet<PathBuf> {
        let mut found_files = BTreeSet::new();

        for sr in &self.search_roots {
            for source_file in walk_source_files(sr) {
                if source_file
                    .language
                    .file_defines_any(&source_file.content, &self.types)
                {
                    found_files.insert(source_file.path);
                }
            }
        }

        found_files
    }
}

/// Filters a pre-walked set of source files for those that define any of the
/// requested types. Use when the caller has already materialised the source
/// collection and wants to avoid a redundant filesystem walk.
pub fn find_definition_files_from_sources(
    types: &BTreeSet<String>,
    sources: &[SourceFile],
) -> BTreeSet<PathBuf> {
    if types.is_empty() {
        return BTreeSet::new();
    }
    let type_vec: Vec<String> = types.iter().cloned().collect();
    sources
        .iter()
        .filter(|sf| sf.language.file_defines_any(&sf.content, &type_vec))
        .map(|sf| sf.path.clone())
        .collect()
}

/// Public API — walks the source tree then filters for definitions.
pub fn find_definition_files(types: &BTreeSet<String>, root: &Path) -> Result<BTreeSet<PathBuf>> {
    if types.is_empty() {
        return Ok(BTreeSet::new());
    }
    let finder = DefinitionFinder::new(types, root)?;
    Ok(finder.find_files())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn types(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_get_search_roots_when_root_is_package() {
        let dir = tempdir().unwrap();
        let package_path = dir.path().join("Package.swift");
        fs::write(&package_path, "swift package content").unwrap();

        let roots = get_search_roots(dir.path()).unwrap();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0], dir.path());
    }

    #[test]
    fn test_find_definition_files_basic() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let good_file_path = root.join("good.swift");
        fs::write(&good_file_path, "import Foundation\nclass MyType {}\n").unwrap();

        let bad_file_path = root.join("bad.swift");
        fs::write(
            &bad_file_path,
            "import Foundation\n// no definitions here\n",
        )
        .unwrap();

        let excluded_dir = root.join("Pods");
        fs::create_dir_all(&excluded_dir).unwrap();
        let excluded_file_path = excluded_dir.join("excluded.swift");
        fs::write(&excluded_file_path, "class MyType {}\n").unwrap();

        let found = find_definition_files(&types(&["MyType"]), root).expect("Should succeed");

        assert!(found.contains(&good_file_path));
        assert!(!found.contains(&bad_file_path));
        assert!(!found.contains(&excluded_file_path));
    }

    #[test]
    fn test_excludes_build_directory() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let sources_dir = root.join("Sources");
        fs::create_dir_all(&sources_dir).unwrap();
        let normal_file = sources_dir.join("MyType.swift");
        fs::write(&normal_file, "class MyType {}\n").unwrap();

        let build_dir = root.join(".build/somepath");
        fs::create_dir_all(&build_dir).unwrap();
        let build_file = build_dir.join("MyType.swift");
        fs::write(&build_file, "class MyType {}\n").unwrap();

        let found =
            find_definition_files(&types(&["MyType"]), root).expect("find_definition_files failed");

        assert!(found.contains(&normal_file));
        assert!(!found.contains(&build_file));
    }

    #[test]
    fn test_deduplicated_files_combined() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let combined_dir = root.join("Combined");
        fs::create_dir_all(&combined_dir).unwrap();

        let both_file = combined_dir.join("BothTypes.swift");
        fs::write(&both_file, "class TypeOne {}\nstruct TypeTwo {}\n").unwrap();

        let only_file = combined_dir.join("OnlyTypeOne.swift");
        fs::write(&only_file, "enum TypeOne {}\n").unwrap();

        let other_file = combined_dir.join("Other.swift");
        fs::write(&other_file, "protocol OtherType {}\n").unwrap();

        let found = find_definition_files(&types(&["TypeOne", "TypeTwo"]), &combined_dir)
            .expect("find_definition_files failed");

        assert!(found.contains(&both_file));
        assert!(found.contains(&only_file));
        assert!(!found.contains(&other_file));
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn test_excludes_pods_directory() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let sources_dir = root.join("Sources");
        fs::create_dir_all(&sources_dir).unwrap();
        let source_file = sources_dir.join("MyType.swift");
        fs::write(&source_file, "class MyType {}\n").unwrap();

        let pods_dir = root.join("Pods");
        fs::create_dir_all(&pods_dir).unwrap();
        let pods_file = pods_dir.join("MyType.swift");
        fs::write(&pods_file, "class MyType {}\n").unwrap();

        let found =
            find_definition_files(&types(&["MyType"]), root).expect("find_definition_files failed");

        assert!(found.contains(&source_file));
        assert!(!found.contains(&pods_file));
    }

    #[test]
    fn test_empty_types() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let result = find_definition_files(&BTreeSet::new(), root)
            .expect("find_definition_files should succeed");
        assert!(
            result.is_empty(),
            "Expected an empty set when types set is empty"
        );
    }

    #[test]
    fn test_non_swift_file_ignored() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let non_swift_file = root.join("definition.txt");
        fs::write(&non_swift_file, "class MyType {}\n").unwrap();

        let found =
            find_definition_files(&types(&["MyType"]), root).expect("find_definition_files failed");

        assert!(!found.contains(&non_swift_file));
    }

    #[test]
    fn test_case_sensitivity() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let correct_case = root.join("correct.swift");
        fs::write(&correct_case, "class MyType {}\n").unwrap();

        let wrong_case = root.join("wrong.swift");
        fs::write(&wrong_case, "class mytype {}\n").unwrap();

        let found =
            find_definition_files(&types(&["MyType"]), root).expect("find_definition_files failed");

        assert!(found.contains(&correct_case));
        assert!(!found.contains(&wrong_case));
    }

    #[test]
    fn test_duplicate_definitions_deduplicated() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let dup_file = root.join("dup.swift");
        fs::write(&dup_file, "class MyType {}\nclass MyType {}\n").unwrap();

        let found =
            find_definition_files(&types(&["MyType"]), root).expect("find_definition_files failed");

        let count = found.iter().filter(|&p| *p == dup_file).count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_no_matching_definitions() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let non_match = root.join("non_match.swift");
        fs::write(&non_match, "class SomeOtherType {}\n").unwrap();

        let found = find_definition_files(&types(&["NonExistentType"]), root)
            .expect("find_definition_files failed");

        assert!(found.is_empty());
    }

    #[test]
    fn test_get_search_roots_with_subpackages() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let subpackage = root.join("SubPackage");
        fs::create_dir_all(&subpackage).unwrap();
        fs::write(subpackage.join("Package.swift"), "swift package content").unwrap();

        let roots = get_search_roots(root).unwrap();
        assert!(roots.contains(&root.to_path_buf()));
        assert!(roots.contains(&subpackage));
        assert_eq!(roots.len(), 2);
    }

    #[test]
    fn test_includes_javascript_files() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let js_file = root.join("script.js");
        fs::write(&js_file, "class MyType {}").unwrap();

        let found =
            find_definition_files(&types(&["MyType"]), root).expect("find_definition_files failed");
        assert!(found.contains(&js_file));
    }

    #[test]
    fn test_additional_definition_keywords() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let protocol_file = root.join("protocol.swift");
        fs::write(&protocol_file, "protocol MyProtocol {}").unwrap();

        let typealias_file = root.join("typealias.swift");
        fs::write(&typealias_file, "typealias MyAlias = Int").unwrap();

        let found = find_definition_files(&types(&["MyProtocol", "MyAlias"]), root)
            .expect("find_definition_files failed");
        assert!(found.contains(&protocol_file));
        assert!(found.contains(&typealias_file));
    }

    #[test]
    #[cfg(unix)]
    fn test_unreadable_files() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let root = dir.path();

        let unreadable_file = root.join("unreadable.swift");
        fs::write(&unreadable_file, "class MyType {}").unwrap();

        let mut perms = fs::metadata(&unreadable_file).unwrap().permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&unreadable_file, perms).unwrap();

        let found =
            find_definition_files(&types(&["MyType"]), root).expect("find_definition_files failed");
        assert!(!found.contains(&unreadable_file));

        let mut perms = fs::metadata(&unreadable_file).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&unreadable_file, perms).unwrap();
    }

    #[test]
    fn test_from_sources_filters_definitions() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::write(root.join("Match.swift"), "class MyType {}\n").unwrap();
        fs::write(root.join("NoMatch.swift"), "let x = 1\n").unwrap();

        let sources = walk_source_files(root);
        let found = find_definition_files_from_sources(&types(&["MyType"]), &sources);

        assert!(found.contains(&root.join("Match.swift")));
        assert!(!found.contains(&root.join("NoMatch.swift")));
    }

    #[test]
    fn test_from_sources_returns_empty_for_empty_types() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("A.swift"), "class A {}\n").unwrap();
        let sources = walk_source_files(dir.path());

        let found = find_definition_files_from_sources(&BTreeSet::new(), &sources);
        assert!(found.is_empty());
    }
}

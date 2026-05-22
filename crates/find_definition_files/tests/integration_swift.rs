// crates/find_definition_files/tests/integration_swift.rs

use find_definition_files::find_definition_files;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn types(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

mod integration_swift {
    use super::*;

    #[test]
    fn test_find_definition_files_basic() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;

        let file1_path = dir.path().join("file1.swift");
        fs::write(&file1_path, "class MyType {}")?;
        let file2_path = dir.path().join("file2.swift");
        fs::write(&file2_path, "struct OtherType {}")?;
        let file3_path = dir.path().join("file3.swift");
        fs::write(&file3_path, "enum Unmatched {}")?;

        let types = types(&["MyType", "OtherType"]);
        let result = find_definition_files(&types, dir.path())?;
        let mut expected = BTreeSet::new();
        expected.insert(file1_path);
        expected.insert(file2_path);

        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_find_definition_files_with_subdirectories() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;

        let root_file = dir.path().join("root.swift");
        fs::write(&root_file, "class MyType {}")?;

        let sub1 = dir.path().join("Sub1");
        fs::create_dir_all(&sub1)?;
        let sub1_file = sub1.join("sub1.swift");
        fs::write(&sub1_file, "struct OtherType {}")?;

        let pods_dir = dir.path().join("Pods");
        fs::create_dir_all(&pods_dir)?;
        let pods_file = pods_dir.join("ignored.swift");
        fs::write(&pods_file, "class MyType {}")?;

        let build_dir = dir.path().join(".build");
        fs::create_dir_all(&build_dir)?;
        let build_file = build_dir.join("ignored.swift");
        fs::write(&build_file, "struct OtherType {}")?;

        let sub2 = dir.path().join("Sub2");
        fs::create_dir_all(&sub2)?;
        let sub2_file = sub2.join("sub2.swift");
        fs::write(&sub2_file, "enum NotRelevant {}")?;

        let types = types(&["MyType", "OtherType"]);
        let result = find_definition_files(&types, dir.path())?;
        let mut expected = BTreeSet::new();
        expected.insert(root_file);
        expected.insert(sub1_file);

        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_find_definition_files_non_swift_files_ignored() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = tempdir()?;

        let swift_file = dir.path().join("a.swift");
        fs::write(&swift_file, "class MyType {}")?;

        let txt_file = dir.path().join("b.txt");
        fs::write(&txt_file, "class MyType {}")?;

        let types = types(&["MyType"]);
        let result = find_definition_files(&types, dir.path())?;
        let mut expected = BTreeSet::new();
        expected.insert(swift_file);

        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_find_definition_files_empty_types_file() {
        let dir = tempdir().unwrap();

        let types: BTreeSet<String> = BTreeSet::new();
        let result = find_definition_files(&types, dir.path())
            .expect("Should succeed with empty set for empty types file");
        assert!(
            result.is_empty(),
            "Expected an empty set when types file is empty"
        );
    }

    #[test]
    fn test_find_definition_files_no_matches() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;

        let file = dir.path().join("file.swift");
        fs::write(&file, "class SomeOtherType {}")?;

        let types = types(&["NonExistentType"]);
        let result = find_definition_files(&types, dir.path())?;
        let expected: BTreeSet<PathBuf> = BTreeSet::new();

        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_find_definition_files_swift_function_definitions(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;

        let function_path = dir.path().join("Helpers.swift");
        fs::write(&function_path, "func fetchData() -> Data { Data() }")?;

        let types = types(&["fetchData"]);
        let result = find_definition_files(&types, dir.path())?;

        let mut expected = BTreeSet::new();
        expected.insert(function_path);
        assert_eq!(result, expected);
        Ok(())
    }
}

// crates/find_definition_files/tests/integration_js.rs

use find_definition_files::find_definition_files;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn types(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

mod integration_javascript {
    use super::*;

    #[test]
    fn test_find_definition_files_basic_js() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;

        let file1_path = dir.path().join("file1.js");
        fs::write(&file1_path, "class MyClass {}")?;
        let file2_path = dir.path().join("file2.js");
        fs::write(&file2_path, "class OtherClass {}")?;
        let file3_path = dir.path().join("file3.js");
        fs::write(&file3_path, "function notAMatch() {}")?;

        let types = types(&["MyClass", "OtherClass"]);
        let result = find_definition_files(&types, dir.path())?;
        let mut expected = BTreeSet::new();
        expected.insert(file1_path);
        expected.insert(file2_path);

        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_find_definition_files_with_subdirectories_js() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = tempdir()?;

        let root_file = dir.path().join("root.js");
        fs::write(&root_file, "class MyClass {}")?;

        let sub1 = dir.path().join("Sub1");
        fs::create_dir_all(&sub1)?;
        let sub1_file = sub1.join("sub1.js");
        fs::write(&sub1_file, "class MyClass {}")?;

        let pods_dir = dir.path().join("Pods");
        fs::create_dir_all(&pods_dir)?;
        let pods_file = pods_dir.join("ignored.js");
        fs::write(&pods_file, "class MyClass {}")?;

        let types = types(&["MyClass"]);
        let result = find_definition_files(&types, dir.path())?;
        let mut expected = BTreeSet::new();
        expected.insert(root_file);
        expected.insert(sub1_file);

        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_find_definition_files_non_js_files_ignored() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;

        let js_file = dir.path().join("a.js");
        fs::write(&js_file, "class MyClass {}")?;
        let txt_file = dir.path().join("b.txt");
        fs::write(&txt_file, "class MyClass {}")?;

        let types = types(&["MyClass"]);
        let result = find_definition_files(&types, dir.path())?;
        let mut expected = BTreeSet::new();
        expected.insert(js_file);

        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_find_definition_files_empty_types_file_js() {
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
    fn test_find_definition_files_no_matches_js() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;

        let file = dir.path().join("file.js");
        fs::write(&file, "class SomeOtherClass {}")?;

        let types = types(&["NonExistentClass"]);
        let result = find_definition_files(&types, dir.path())?;
        let expected: BTreeSet<PathBuf> = BTreeSet::new();

        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_find_definition_files_js_function_definitions() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = tempdir()?;

        let function_path = dir.path().join("helper.js");
        fs::write(&function_path, "export function helper() {}")?;

        let types = types(&["helper"]);
        let result = find_definition_files(&types, dir.path())?;

        let mut expected = BTreeSet::new();
        expected.insert(function_path);
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_find_definition_files_additional_js_extensions(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;

        let jsx_path = dir.path().join("component.jsx");
        fs::write(&jsx_path, "class JsxType {}")?;
        let mjs_path = dir.path().join("module.mjs");
        fs::write(&mjs_path, "class MjsType {}")?;
        let cjs_path = dir.path().join("common.cjs");
        fs::write(&cjs_path, "class CjsType {}")?;

        let types = types(&["JsxType", "MjsType", "CjsType"]);
        let result = find_definition_files(&types, dir.path())?;

        let mut expected = BTreeSet::new();
        expected.insert(jsx_path);
        expected.insert(mjs_path);
        expected.insert(cjs_path);
        assert_eq!(result, expected);
        Ok(())
    }
}

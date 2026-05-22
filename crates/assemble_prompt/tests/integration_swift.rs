// crates/assemble_prompt/tests/integration_swift.rs

#[cfg(test)]
mod integration_swift {
    use assemble_prompt::{assemble_prompt, AssemblyOptions};
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use std::process::Command;
    use tempfile::{NamedTempFile, TempDir};

    /// Test assembling a prompt from a single Swift file.
    #[test]
    fn test_assemble_prompt_single_file() {
        // Create a temporary Swift file with some content.
        let mut swift_file = NamedTempFile::new().expect("Failed to create temp Swift file");
        let swift_content = "public class MyClass {\n    func test() {}\n}\n";
        write!(swift_file, "{}", swift_content).expect("Failed to write to Swift file");

        let found_files_vec = vec![swift_file.path().to_path_buf()];

        let file_name = swift_file
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        // Call assemble_prompt with the in‑memory list.
        let output = assemble_prompt(
            &found_files_vec,
            &AssemblyOptions {
                todo_file_basename: Some(file_name.clone()),
                diff_branch: None,
            },
        )
        .expect("assemble_prompt failed");

        // Check that the output contains a header for the Swift file and the file content.
        assert!(
            output.contains(&format!("The contents of {} is as follows:", file_name)),
            "Output should include the file header for {}",
            file_name
        );
        assert!(
            output.contains("public class MyClass"),
            "Output should contain the Swift file content"
        );
        // Verify that the fixed instruction is appended.
        assert!(
            output.contains("Can you do the TODO:- in the above code?"),
            "Output should contain the fixed instruction"
        );
    }

    /// Test that multiple Swift files (including duplicates) are processed correctly.
    #[test]
    fn test_assemble_prompt_multiple_files_deduplicated() {
        // Create two temporary Swift files.
        let mut swift_file1 = NamedTempFile::new().expect("Failed to create Swift file 1");
        let swift_content1 = "public struct StructOne {}\n";
        write!(swift_file1, "{}", swift_content1).expect("Failed to write to Swift file 1");

        let mut swift_file2 = NamedTempFile::new().expect("Failed to create Swift file 2");
        let swift_content2 = "public enum EnumTwo {}\n";
        write!(swift_file2, "{}", swift_content2).expect("Failed to write to Swift file 2");

        // Build found_files list including a duplicate of swift_file1, then sort and dedup.
        let mut found_files_vec: Vec<PathBuf> = vec![
            swift_file1.path().to_path_buf(),
            swift_file2.path().to_path_buf(),
            swift_file1.path().to_path_buf(),
        ];
        found_files_vec.sort();
        found_files_vec.dedup();

        let file_name1 = swift_file1
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let output = assemble_prompt(
            &found_files_vec,
            &AssemblyOptions {
                todo_file_basename: Some(file_name1.clone()),
                diff_branch: None,
            },
        )
        .expect("assemble_prompt failed");

        // Bind file names.
        let file_name1_dup = swift_file1
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let file_name2 = swift_file2
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();

        // Check that the prompt includes headers for both files.
        let header_count = output
            .matches(&format!(
                "The contents of {} is as follows:",
                file_name1_dup
            ))
            .count();
        assert_eq!(
            header_count, 1,
            "The header for {} should appear only once, but found {} times",
            file_name1_dup, header_count
        );
        assert!(
            output.contains(&format!("The contents of {} is as follows:", file_name2)),
            "Output should include the header for {}",
            file_name2
        );
        // Verify both file contents are present.
        assert!(
            output.contains("public struct StructOne"),
            "Output should contain the content from Swift file 1"
        );
        assert!(
            output.contains("public enum EnumTwo"),
            "Output should contain the content from Swift file 2"
        );
        assert!(
            output.contains("Can you do the TODO:- in the above code?"),
            "Output should contain the fixed instruction"
        );
    }

    /// Test that if a found file does not exist, it is skipped.
    #[test]
    fn test_assemble_prompt_with_missing_file() {
        // Create a valid Swift file.
        let mut swift_file = NamedTempFile::new().expect("Failed to create Swift file");
        let swift_content = "public class MissingTest {}\n";
        write!(swift_file, "{}", swift_content).expect("Failed to write to Swift file");

        let found_files_vec: Vec<PathBuf> = vec![
            swift_file.path().to_path_buf(),
            PathBuf::from("/path/to/nonexistent/file.swift"),
        ];

        let output = assemble_prompt(&found_files_vec, &AssemblyOptions::default())
            .expect("assemble_prompt failed");

        let file_name = swift_file
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();

        // Verify that output contains header and content for the valid file.
        assert!(
            output.contains(&format!("The contents of {} is as follows:", file_name)),
            "Output should include header for the valid Swift file"
        );
        assert!(
            output.contains("public class MissingTest"),
            "Output should contain the valid Swift file content"
        );
        // Ensure that no reference to the missing file appears.
        assert!(
            !output.contains("nonexistent"),
            "Output should not reference the missing file"
        );
        assert!(
            output.contains("Can you do the TODO:- in the above code?"),
            "Output should contain the fixed instruction"
        );
    }

    /// Test that an empty found_files list results in a prompt containing only the fixed instruction.
    #[test]
    fn test_assemble_prompt_empty_found_files() {
        let found_files: Vec<PathBuf> = Vec::new();

        let output = assemble_prompt(&found_files, &AssemblyOptions::default())
            .expect("assemble_prompt failed");

        let trimmed_output = output.trim();
        assert!(
            trimmed_output.starts_with("Can you do the TODO:- in the above code?"),
            "Output should start with the fixed instruction when no files are provided, got: {}",
            trimmed_output
        );
        assert!(
            trimmed_output.ends_with("doesn't have the hyphen"),
            "Output should end with the fixed instruction when no files are provided, got: {}",
            trimmed_output
        );
    }

    /// New test: Verify that a Swift file with a generic function is processed correctly.
    #[test]
    fn test_assemble_prompt_swift_generics() {
        // Create a temporary Swift file containing a generic function.
        let mut swift_file = NamedTempFile::new().expect("Failed to create temp Swift file");
        let swift_content = r#"
public func genericFunction<T: Equatable>(param: T) -> T? {
    print("Inside generic function")
}

// v
// Extra context that is not part of the function block.
// ^

 // TODO: - perform generic task
"#;
        write!(swift_file, "{}", swift_content).expect("Failed to write to Swift file");

        let found_files = vec![swift_file.path().to_path_buf()];

        let file_name = swift_file
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        // Call assemble_prompt.
        let output = assemble_prompt(
            &found_files,
            &AssemblyOptions {
                todo_file_basename: Some(file_name.clone()),
                diff_branch: None,
            },
        )
        .expect("assemble_prompt failed");

        // Verify that the output contains the header for the Swift file,
        // and that it includes key substrings from the generic function declaration.
        assert!(
            output.contains(&format!("The contents of {} is as follows:", file_name)),
            "Output should include the file header for {}",
            file_name
        );
        assert!(
            output.contains("genericFunction"),
            "Output should contain the function name 'genericFunction'"
        );
        assert!(
            output.contains("-> T?"),
            "Output should indicate the optional return type"
        );
        assert!(
            output.contains("print(\"Inside generic function\")"),
            "Output should contain the function body"
        );
        assert!(
            output.contains("Can you do the TODO:- in the above code?"),
            "Output should contain the fixed instruction"
        );
    }

    #[test]
    fn test_assemble_prompt_with_diff_option() {
        // Create a temporary directory to initialize a new git repository.
        let temp_dir = TempDir::new().expect("Failed to create temporary directory");
        let repo_path = temp_dir.path();

        // Initialize a new git repository.
        let init_status = Command::new("git")
            .arg("init")
            .current_dir(repo_path)
            .status()
            .expect("Failed to initialize git repository");
        assert!(init_status.success(), "Git init failed");

        // Create a Swift file in the repository.
        let swift_file_path = repo_path.join("DiffTest.swift");
        let initial_content = "public class DiffTest {\n}\n";
        fs::write(&swift_file_path, initial_content).expect("Failed to write initial Swift file");

        // Add and commit the file.
        let add_status = Command::new("git")
            .args(["add", "DiffTest.swift"])
            .current_dir(repo_path)
            .status()
            .expect("Failed to git add file");
        assert!(add_status.success(), "Git add failed");

        let commit_status = Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(repo_path)
            .status()
            .expect("Failed to git commit");
        assert!(commit_status.success(), "Git commit failed");

        // Modify the Swift file to create a diff.
        let modified_content = "public class DiffTest {\n    func newDiff() {}\n}\n";
        fs::write(&swift_file_path, modified_content).expect("Failed to modify Swift file");

        let found_files = vec![swift_file_path.clone()];

        let file_basename = swift_file_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        // Call assemble_prompt.
        let output = assemble_prompt(
            &found_files,
            &AssemblyOptions {
                todo_file_basename: Some(file_basename),
                diff_branch: Some("HEAD".to_string()),
            },
        )
        .expect("assemble_prompt failed");

        // Verify that the output contains the diff section.
        assert!(
            output.contains("The diff for"),
            "Output should contain a diff section"
        );
        assert!(
            output.contains("func newDiff()"),
            "Diff should mention the modified content from the Swift file"
        );
    }
}

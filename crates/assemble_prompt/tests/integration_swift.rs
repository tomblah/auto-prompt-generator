// rust/assemble_prompt/tests/integration_swift.rs

#[cfg(test)]
mod integration_swift {
    use assemble_prompt::assemble_prompt;
    use std::env;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use std::process::Command;
    use tempfile::{NamedTempFile, TempDir};

    /// Helper function to read a temporary found_files file into a Vec<String>.
    fn read_found_files(file_path: &str) -> Vec<String> {
        fs::read_to_string(file_path)
            .expect("Failed to read found files list")
            .lines()
            .map(|l| l.to_string())
            .collect()
    }

    /// Test assembling a prompt from a single Swift file.
    #[test]
    fn test_assemble_prompt_single_file() {
        // Create a temporary Swift file with some content.
        let mut swift_file = NamedTempFile::new().expect("Failed to create temp Swift file");
        let swift_content = "public class MyClass {\n    func test() {}\n}\n";
        write!(swift_file, "{}", swift_content).expect("Failed to write to Swift file");
        let swift_path = swift_file.path().to_str().unwrap().to_string();

        // Create a temporary found_files file that contains the Swift file path.
        let mut found_files_temp =
            NamedTempFile::new().expect("Failed to create found files temp file");
        writeln!(found_files_temp, "{}", swift_path)
            .expect("Failed to write to found files file");
        let found_files_path = found_files_temp
            .into_temp_path()
            .keep()
            .expect("Failed to persist found files list");

        // Read the found files into a vector.
        let found_files_vec = read_found_files(found_files_path.to_str().unwrap());

        // Set the TODO_FILE_BASENAME so that context is appended if applicable.
        let file_name = swift_file
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        env::set_var("TODO_FILE_BASENAME", &file_name);

        // Call assemble_prompt with the in‑memory list.
        let output = assemble_prompt(&found_files_vec, "ignored instruction")
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
        write!(swift_file1, "{}", swift_content1)
            .expect("Failed to write to Swift file 1");
        let swift_path1 = swift_file1.path().to_str().unwrap().to_string();

        let mut swift_file2 = NamedTempFile::new().expect("Failed to create Swift file 2");
        let swift_content2 = "public enum EnumTwo {}\n";
        write!(swift_file2, "{}", swift_content2)
            .expect("Failed to write to Swift file 2");
        let swift_path2 = swift_file2.path().to_str().unwrap().to_string();

        // Create a temporary found_files file that includes both files and a duplicate of swift_file1.
        let mut found_files_temp =
            NamedTempFile::new().expect("Failed to create found files file");
        writeln!(found_files_temp, "{}", swift_path1)
            .expect("Failed to write to found files file");
        writeln!(found_files_temp, "{}", swift_path2)
            .expect("Failed to write to found files file");
        // Duplicate swift_file1.
        writeln!(found_files_temp, "{}", swift_path1)
            .expect("Failed to write duplicate entry");
        let found_files_path = found_files_temp
            .into_temp_path()
            .keep()
            .expect("Failed to persist found files list");

        // Read the found files into a vector.
        let found_files_vec = read_found_files(found_files_path.to_str().unwrap());

        // Set the TODO_FILE_BASENAME for swift_file1.
        let file_name1 = PathBuf::from(&swift_path1)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        env::set_var("TODO_FILE_BASENAME", &file_name1);

        let output = assemble_prompt(&found_files_vec, "ignored instruction")
            .expect("assemble_prompt failed");

        // Bind file names.
        let file_name1_dup = PathBuf::from(&swift_path1)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let file_name2 = PathBuf::from(&swift_path2)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();

        // Check that the prompt includes headers for both files.
        let header_count = output.matches(&format!("The contents of {} is as follows:", file_name1_dup)).count();
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
        write!(swift_file, "{}", swift_content)
            .expect("Failed to write to Swift file");
        let swift_path = swift_file.path().to_str().unwrap().to_string();

        // Create a found_files file that includes one valid and one non-existent file.
        let mut found_files_temp =
            NamedTempFile::new().expect("Failed to create found files file");
        writeln!(found_files_temp, "{}", swift_path)
            .expect("Failed to write valid file path");
        writeln!(found_files_temp, "/path/to/nonexistent/file.swift")
            .expect("Failed to write non-existent file path");
        let found_files_path = found_files_temp
            .into_temp_path()
            .keep()
            .expect("Failed to persist found files list");

        let found_files_vec = read_found_files(found_files_path.to_str().unwrap());

        let output = assemble_prompt(&found_files_vec, "ignored instruction")
            .expect("assemble_prompt failed");

        let file_name = PathBuf::from(&swift_path)
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
        // Use an empty in-memory found_files list.
        let found_files: Vec<String> = Vec::new();

        let output = assemble_prompt(&found_files, "ignored instruction")
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
        write!(swift_file, "{}", swift_content)
            .expect("Failed to write to Swift file");
        let swift_path = swift_file.path().to_str().unwrap().to_string();

        // Build the in-memory found_files list.
        let found_files = vec![swift_path.clone()];

        // Set the TODO_FILE_BASENAME to the Swift file's basename.
        let file_name = swift_file
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        env::set_var("TODO_FILE_BASENAME", &file_name);

        // Call assemble_prompt.
        let output = assemble_prompt(&found_files, "ignored instruction")
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
            .current_dir(&repo_path)
            .status()
            .expect("Failed to initialize git repository");
        assert!(init_status.success(), "Git init failed");

        // Create a Swift file in the repository.
        let swift_file_path = repo_path.join("DiffTest.swift");
        let initial_content = "public class DiffTest {\n}\n";
        fs::write(&swift_file_path, initial_content)
            .expect("Failed to write initial Swift file");

        // Add and commit the file.
        let add_status = Command::new("git")
            .args(&["add", "DiffTest.swift"])
            .current_dir(&repo_path)
            .status()
            .expect("Failed to git add file");
        assert!(add_status.success(), "Git add failed");

        let commit_status = Command::new("git")
            .args(&["commit", "-m", "Initial commit"])
            .current_dir(&repo_path)
            .status()
            .expect("Failed to git commit");
        assert!(commit_status.success(), "Git commit failed");

        // Modify the Swift file to create a diff.
        let modified_content = "public class DiffTest {\n    func newDiff() {}\n}\n";
        fs::write(&swift_file_path, modified_content)
            .expect("Failed to modify Swift file");

        // Build the in-memory found_files list.
        let found_files = vec![swift_file_path.to_str().unwrap().to_string()];

        // Set DIFF_WITH_BRANCH to "HEAD".
        env::set_var("DIFF_WITH_BRANCH", "HEAD");

        // Set TODO_FILE_BASENAME.
        let file_basename = swift_file_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        env::set_var("TODO_FILE_BASENAME", &file_basename);

        // Call assemble_prompt.
        let output = assemble_prompt(&found_files, "ignored instruction")
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

        // Cleanup.
        env::remove_var("DIFF_WITH_BRANCH");
    }
}

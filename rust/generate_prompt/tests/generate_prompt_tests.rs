use assert_cmd::Command;
use predicates::prelude::*;
use std::env;
use std::fs;
use std::io::Write;
use tempfile::TempDir;

/// On Unix systems, creates a dummy executable (a shell script) in the given temporary directory.
/// The script simply echoes the provided output (or executes a simple shell command).
#[cfg(unix)]
fn create_dummy_executable(dir: &TempDir, name: &str, output: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, format!("#!/bin/sh\n{}", output)).unwrap();
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_cmd::prelude::*;
    use std::process::Command;

    /// Helper: clear GET_GIT_ROOT so tests that don't need it won't use a stale value.
    fn clear_git_root() {
        env::remove_var("GET_GIT_ROOT");
    }

    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_singular_mode() {
        clear_git_root();
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        // Set up dummy commands.
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        // Create a TODO file with the expected content.
        fs::write(&todo_file, "   // TODO: - Fix critical bug").unwrap();
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Fix critical bug");
        // In singular mode, we expect the file list to contain only the TODO file.
        create_dummy_executable(&temp_dir, "filter_files_singular", &todo_file);
        // Dummy assemble_prompt (not used in singular mode output) is set up for consistency.
        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

        // Force the instruction file override.
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--singular");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Found exactly one instruction in"))
            .stdout(predicate::str::contains("Singular mode enabled: only including the TODO file"))
            .stdout(predicate::str::contains("// TODO: - Fix critical bug"))
            .stdout(predicate::str::contains("Prompt has been copied to clipboard."));
        
        env::remove_var("GET_GIT_ROOT");
    }


    /// If --include-references is used but the TODO file isn’t a Swift file, we should error out.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_include_references_error_for_non_swift() {
        clear_git_root();
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        // Use a TODO file with .js extension.
        let todo_file = format!("{}/TODO.js", fake_git_root_path);
        fs::write(&todo_file, "   // TODO: - Fix issue").unwrap();
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Fix issue");
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "DummyType").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
        let def_file = format!("{}/Definition.swift", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_definition_files", &def_file);
        create_dummy_executable(&temp_dir, "filter_files_singular", &todo_file);
        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

        // Force the instruction file override.
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);

        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--include-references");

        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("--include-references is only supported for Swift files"));

        clear_git_root();
    }

    /// Test a normal (non‑singular) run with multiple definition files.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_normal_mode() {
        // Use a fake Git root by setting GET_GIT_ROOT.
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        env::set_var("GET_GIT_ROOT", fake_git_root_path);

        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        // Write a TODO file that includes a type declaration so that extract_types_from_file extracts "TypeFixBug".
        fs::write(&todo_file, "class TypeFixBug {}\n   // TODO: - Fix bug").unwrap();
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Fix bug");

        // Force the instruction file override.
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);

        // Create a dummy types file.
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TypeFixBug").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());

        // Create definition files in the fake Git root.
        let def_file1 = fake_git_root.path().join("Definition1.swift");
        fs::write(&def_file1, "class TypeFixBug {}").unwrap();
        let def_file2 = fake_git_root.path().join("Definition2.swift");
        fs::write(&def_file2, "class TypeFixBug {}").unwrap();

        // Dummy assemble_prompt is not used in normal mode because the final prompt is not printed (clipboard copy occurs).
        // Instead, we check that the output contains key status messages.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Found exactly one instruction in"))
            .stdout(predicate::str::contains("Instruction content: // TODO: - Fix bug"))
            .stdout(predicate::str::contains("Definition1.swift"))
            .stdout(predicate::str::contains("Definition2.swift"))
            .stdout(predicate::str::contains("Prompt has been copied to clipboard."));
        
        env::remove_var("GET_GIT_ROOT");
    }

    /// Test inclusion of referencing files for Swift.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_include_references_for_swift() {
        clear_git_root();
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        // Include a type declaration so that the extractor finds "MyType".
        fs::write(&todo_file, "class MyType {}\n   // TODO: - Fix bug").unwrap();
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Fix bug");

        env::set_var("GET_INSTRUCTION_FILE", &todo_file);

        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TypeA").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
        let def_file = format!("{}/Definition.swift", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_definition_files", &def_file);
        create_dummy_executable(&temp_dir, "filter_files_singular", &todo_file);
        create_dummy_executable(&temp_dir, "find_referencing_files", &format!("{}/Ref1.swift", fake_git_root_path));
        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--include-references");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Including files that reference the enclosing type"))
            .stdout(predicate::str::contains("Enclosing type: MyType"))
            .stdout(predicate::str::contains("Searching for files referencing MyType"))
            .stdout(predicate::str::contains("Warning: The --include-references option is experimental."))
            .stdout(predicate::str::contains("Prompt has been copied to clipboard."));
        
        clear_git_root();
    }
}

#[cfg(test)]
mod additional_tests {
    use super::*;
    use assert_cmd::prelude::*;
    use std::env;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    /// Test that when the --force-global flag is passed the global context is used.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_force_global() {
        // This test requires its own fake Git root.
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        // Set GET_GIT_ROOT to our fake_git_root.
        env::set_var("GET_GIT_ROOT", fake_git_root_path);

        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        // Include a type declaration for extraction.
        fs::write(&todo_file, "class TypeForce {}\n   // TODO: - Force global test").unwrap();
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "NonEmptyValue");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Force global test");
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TypeForce").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
        let def_files_output = format!("{}/Definition.swift", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_definition_files", &def_files_output);
        create_dummy_executable(&temp_dir, "filter_excluded_files", &def_files_output);
        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

        // Also force GET_INSTRUCTION_FILE so that extraction works.
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);

        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--force-global");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Force global enabled: using Git root for context"));

        env::remove_var("GET_GIT_ROOT");
    }

    /// Test that when exclusion flags are provided the exclusion branch is taken.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_exclude() {
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        env::set_var("GET_GIT_ROOT", fake_git_root_path);

        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        fs::write(&todo_file, "class TypeExclude {}\n   // TODO: - Exclude test").unwrap();
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Exclude test");
        let types_file = temp_dir.path().join("types.txt");
        fs::write(&types_file, "TypeExclude").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file.to_str().unwrap());
        let def_file1 = fake_git_root.path().join("Definition1.swift");
        fs::write(&def_file1, "class TypeExclude {}").unwrap();
        let def_file2 = fake_git_root.path().join("Definition2.swift");
        fs::write(&def_file2, "class TypeExclude {}").unwrap();
        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.args(&["--exclude", "ExcludePattern", "--exclude", "AnotherPattern"]);

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Excluding files matching:"))
            .stdout(predicate::str::contains("Definition1.swift"))
            .stdout(predicate::str::contains("Definition2.swift"));

        env::remove_var("GET_GIT_ROOT");
    }
    
    /// Test that generate_prompt exits with an error when multiple markers are present.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_multiple_markers() {
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        env::set_var("GET_GIT_ROOT", fake_git_root_path);

        let instruction_path = format!("{}/Instruction.swift", fake_git_root_path);
        let multi_marker_content = "\
            // TODO: - Marker One\n\
            Some content here\n\
            // TODO: - Marker Two\n\
            More content here\n\
            // TODO: -\n";
        fs::write(&instruction_path, multi_marker_content).unwrap();
        env::set_var("GET_INSTRUCTION_FILE", &instruction_path);
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &instruction_path);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        let types_file = temp_dir.path().join("types.txt");
        fs::write(&types_file, "TypeA").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file.to_str().unwrap());
        create_dummy_executable(&temp_dir, "find_definition_files", "dummy_definitions");
        create_dummy_executable(&temp_dir, "filter_files_singular", &instruction_path);

        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("Multiple // TODO: - markers found. Exiting."));

        env::remove_var("GET_GIT_ROOT");
    }

    
    /// New Test: Final Prompt is copied to clipboard.
    ///
    /// This test creates a dummy pbcopy executable that writes its stdin to a file,
    /// allowing us to assert that the final prompt (with the fixed instruction) is produced.
    #[test]
    #[cfg(unix)]
    fn test_final_prompt_copied_to_clipboard() {
        let temp_dir = TempDir::new().unwrap();
        // Path to capture clipboard output.
        let clipboard_file = temp_dir.path().join("dummy_clipboard.txt");
        // Create dummy pbcopy: it reads from stdin and writes to clipboard_file.
        let pbcopy_script = format!("cat > \"{}\"", clipboard_file.display());
        create_dummy_executable(&temp_dir, "pbcopy", &pbcopy_script);

        // Set up a fake Git root and environment.
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();
        env::set_var("GET_GIT_ROOT", fake_git_root_path);

        // Create a dummy TODO file.
        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        fs::write(&todo_file, "class TypeFixBug {}\n   // TODO: - Fix bug").unwrap();
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Fix bug");

        // Create dummy types file and definition files.
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TypeFixBug").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
        let def_file1 = fake_git_root.path().join("Definition1.swift");
        fs::write(&def_file1, "class TypeFixBug {}").unwrap();
        let def_file2 = fake_git_root.path().join("Definition2.swift");
        fs::write(&def_file2, "class TypeFixBug {}").unwrap();

        // Dummy assemble_prompt returns a simulated final prompt that includes the fixed instruction.
        let simulated_prompt = "\
The contents of Definition1.swift is as follows:

class TypeFixBug {}

--------------------------------------------------
The contents of Definition2.swift is as follows:

class TypeFixBug {}

--------------------------------------------------

Can you do the TODO:- in the above code? But ignoring all FIXMEs and other TODOs...";
        create_dummy_executable(&temp_dir, "assemble_prompt", simulated_prompt);

        // Force GET_INSTRUCTION_FILE to point to our TODO file.
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);

        // Prepend our temp_dir (which contains our dummy pbcopy and other commands) to PATH.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        // Unset DISABLE_PBCOPY so that clipboard copy occurs.
        env::remove_var("DISABLE_PBCOPY");

        // Run generate_prompt.
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.assert().success();

        // Read the dummy clipboard file.
        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Assert that the clipboard content contains the expected fixed instruction.
        assert!(clipboard_content.contains("Can you do the TODO:- in the above code? But ignoring all FIXMEs"),
                "Clipboard content did not contain the expected fixed instruction: {}", clipboard_content);

        env::remove_var("GET_GIT_ROOT");
    }
        
    #[test]
    #[cfg(unix)]
    fn test_final_prompt_formatting_with_multiple_files() {
        use std::env;
        use std::fs;
        use std::process::Command;
        use tempfile::TempDir;

        // Create a temporary directory to host our dummy executables.
        let temp_dir = TempDir::new().unwrap();

        // Create a dummy pbcopy that writes its stdin to a file (simulate clipboard).
        let clipboard_file = temp_dir.path().join("dummy_clipboard.txt");
        let pbcopy_script = format!("cat > \"{}\"", clipboard_file.display());
        create_dummy_executable(&temp_dir, "pbcopy", &pbcopy_script);

        // Set up a fake Git root.
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();
        env::set_var("GET_GIT_ROOT", fake_git_root_path);

        // Create a TODO file with known content.
        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        fs::write(&todo_file, "class TestClass {}\n   // TODO: - Refactor code").unwrap();

        // Set up dummy executables needed by generate_prompt.
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Refactor code");

        // Create a dummy types file so that extract_types_from_file produces "TestClass".
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TestClass").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());

        // Create two definition files in the fake Git root.
        let def_file1 = fake_git_root.path().join("Definition1.swift");
        fs::write(&def_file1, "class TestClass {}").unwrap();
        let def_file2 = fake_git_root.path().join("Definition2.swift");
        fs::write(&def_file2, "class TestClass {}").unwrap();

        // Create a dummy find_definition_files that echoes both definition file paths.
        let find_def_script = format!("echo \"{}\\n{}\"", def_file1.display(), def_file2.display());
        create_dummy_executable(&temp_dir, "find_definition_files", &find_def_script);

        // Create a dummy filter_excluded_files (can simply echo input).
        create_dummy_executable(&temp_dir, "filter_excluded_files", "");

        // Simulate an assemble_prompt command that returns a predictable final prompt.
        let simulated_prompt = format!(
            "The contents of {} is as follows:\n\n{}\n\n--------------------------------------------------\nThe contents of {} is as follows:\n\n{}\n\n--------------------------------------------------\n\nCan you do the TODO:- in the above code? But ignoring all FIXMEs and other TODOs...",
            def_file1.file_name().unwrap().to_string_lossy(),
            fs::read_to_string(&def_file1).unwrap(),
            def_file2.file_name().unwrap().to_string_lossy(),
            fs::read_to_string(&def_file2).unwrap()
        );
        create_dummy_executable(&temp_dir, "assemble_prompt", &simulated_prompt);

        // Force GET_INSTRUCTION_FILE to point to our TODO file.
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);

        // Prepend our dummy executables directory to the PATH.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        // Ensure clipboard copy is enabled.
        env::remove_var("DISABLE_PBCOPY");

        // Run the generate_prompt binary.
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.assert().success();

        // Read the simulated clipboard file.
        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Assert that the prompt contains headers for both definition files.
        assert!(clipboard_content.contains("The contents of Definition1.swift is as follows:"),
                "Missing header for Definition1.swift: {}", clipboard_content);
        assert!(clipboard_content.contains("The contents of Definition2.swift is as follows:"),
                "Missing header for Definition2.swift: {}", clipboard_content);
        // Assert that the fixed instruction is appended.
        assert!(clipboard_content.contains("Can you do the TODO:- in the above code?"),
                "Missing fixed instruction: {}", clipboard_content);

        env::remove_var("GET_GIT_ROOT");
    }
}

// FIXME: I don't trust these integration tests as they require setting internal environment variables and therefore they can't be trusted as adding coverage or being valid tests. For now, integration tests will be done through bats files (which I also don't trust and will need to be fixed...).
#[cfg(test)]
mod integration_tests {
    use assert_cmd::Command;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use filetime::{set_file_mtime, FileTime};

    /// Sets up a dummy Git project that is inside a Swift package,
    /// but with one extra file outside the package.
    ///
    /// Dummy Project Structure:
    ///
    /// git_root/
    /// ├── my_package/
    /// │   ├── Package.swift           // Marks this directory as a Swift package.
    /// │   ├── Instruction.swift       // Contains the main instruction.
    /// │   │                           // It has a TODO trigger for "TriggerCommentType"
    /// │   │                           // and a non-trigger comment mentioning "CommentReferencedType".
    /// │   ├── OldTodo.swift           // Contains an old TODO marker.
    /// │   ├── Ref.swift               // A referencing file that should be excluded.
    /// │   └── Sources/
    /// │       ├── Definition1.swift   // Defines DummyType1.
    /// │       ├── Definition2.swift   // Defines DummyType2.
    /// │       ├── TriggerCommentReferenced.swift   // Defines TriggerCommentType.
    /// │       └── CommentReferenced.swift            // Defines CommentReferencedType.
    /// └── Outside.swift               // (Outside the package) Defines DummyType3.
    ///
    /// The Instruction.swift file (inside my_package) contains:
    ///     class SomeClass {
    ///         var foo: DummyType1? = nil
    ///         var bar: DummyType2? = nil
    ///         var dummy: DummyType3? = nil
    ///     }
    ///     // TODO: - Let's fix TriggerCommentType
    ///     // Note: CommentReferencedType is mentioned here but is not a trigger.
    ///
    /// Returns a tuple (git_root_dir, instruction_file_path) where git_root_dir is the TempDir for the project,
    /// and instruction_file_path points to my_package/Instruction.swift.
    fn setup_dummy_project() -> (TempDir, PathBuf) {
        // Create the git root (which is not a Swift package by itself)
        let git_root_dir = TempDir::new().unwrap();
        let git_root_path = git_root_dir.path();

        // Set GET_GIT_ROOT to the git root.
        env::set_var("GET_GIT_ROOT", git_root_path.to_str().unwrap());

        // Create the package directory inside the git root.
        let package_dir = git_root_path.join("my_package");
        fs::create_dir_all(&package_dir).unwrap();

        // Create Package.swift inside the package directory.
        let package_file_path = package_dir.join("Package.swift");
        fs::write(&package_file_path, "// swift package").unwrap();

        // Create the main Instruction.swift file inside the package.
        // Note: The TODO trigger now directly references "TriggerCommentType".
        // Also, a non-trigger comment mentions "CommentReferencedType".
        let instruction_file_path = package_dir.join("Instruction.swift");
        fs::write(
            &instruction_file_path,
            "class SomeClass {
    var foo: DummyType1? = nil
    var bar: DummyType2? = nil
    var dummy: DummyType3? = nil
}
 // TODO: - Let's fix TriggerCommentType
 // Note: CommentReferencedType is mentioned here but should not trigger inclusion.",
        )
        .unwrap();

        // Create an extra file with an old TODO marker inside the package.
        let old_todo_path = package_dir.join("OldTodo.swift");
        fs::write(&old_todo_path, "class OldClass { } // TODO: - Old marker").unwrap();
        set_file_mtime(&old_todo_path, FileTime::from_unix_time(1000, 0)).unwrap();

        // Create a Sources directory inside the package with definition files.
        let sources_dir = package_dir.join("Sources");
        fs::create_dir_all(&sources_dir).unwrap();
        let def1_path = sources_dir.join("Definition1.swift");
        let def2_path = sources_dir.join("Definition2.swift");
        fs::write(&def1_path, "class DummyType1 { }").unwrap();
        fs::write(&def2_path, "class DummyType2 { }").unwrap();

        let trigger_file = sources_dir.join("TriggerCommentReferenced.swift");
        fs::write(&trigger_file, "class TriggerCommentType { }").unwrap();

        let comment_file = sources_dir.join("CommentReferenced.swift");
        fs::write(&comment_file, "class CommentReferencedType { }").unwrap();

        // Create a referencing file in the package root.
        let ref_file_path = package_dir.join("Ref.swift");
        fs::write(&ref_file_path, "let instance = SomeClass()").unwrap();

        // Now add a file outside the package (at the Git root) that defines another type.
        let outside_file = git_root_path.join("Outside.swift");
        fs::write(&outside_file, "class DummyType3 { }").unwrap();

        (git_root_dir, instruction_file_path)
    }

    /// Sets up a dummy pbcopy executable that writes its stdin to a temporary file.
    /// Returns a tuple (pbcopy_dir, clipboard_file) where pbcopy_dir is the TempDir
    /// containing the dummy pbcopy and clipboard_file is the PathBuf to the file where output is captured.
    fn setup_dummy_pbcopy() -> (TempDir, PathBuf) {
        let pbcopy_dir = TempDir::new().unwrap();
        let clipboard_file = pbcopy_dir.path().join("clipboard.txt");
        let dummy_pbcopy_path = pbcopy_dir.path().join("pbcopy");
        fs::write(
            &dummy_pbcopy_path,
            format!("#!/bin/sh\ncat > \"{}\"", clipboard_file.display()),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dummy_pbcopy_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dummy_pbcopy_path, perms).unwrap();
        }
        (pbcopy_dir, clipboard_file)
    }

    /// Integration test for normal mode.
    /// Expects that generate_prompt (without --singular or --include-references)
    /// will include the Instruction.swift file and both definition files,
    /// while excluding the referencing file (Ref.swift) and OldTodo.swift.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_normal_mode_includes_all_files() {
        let (project_dir, instruction_file_path) = setup_dummy_project();
        let project_path = project_dir.path();

        env::set_var("GET_INSTRUCTION_FILE", instruction_file_path.to_str().unwrap()); // FIXME: hack workaround
        env::remove_var("DISABLE_PBCOPY");

        let (pbcopy_dir, clipboard_file) = setup_dummy_pbcopy();
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.assert().success();

        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        assert!(
            clipboard_content.contains("The contents of Instruction.swift is as follows:"),
            "Expected clipboard to include the Instruction.swift file header"
        );
        assert!(
            clipboard_content.contains("The contents of Definition1.swift is as follows:"),
            "Expected clipboard to include Definition1.swift header"
        );
        assert!(
            clipboard_content.contains("The contents of Definition2.swift is as follows:"),
            "Expected clipboard to include Definition2.swift header"
        );
        assert!(
            clipboard_content.contains("class DummyType1 { }"),
            "Expected clipboard to contain the declaration of DummyType1"
        );
        assert!(
            clipboard_content.contains("class DummyType2 { }"),
            "Expected clipboard to contain the declaration of DummyType2"
        );
        assert!(
            clipboard_content.contains("DummyType1"),
            "Expected the Instruction.swift file to reference DummyType1"
        );
        assert!(
            clipboard_content.contains("DummyType2"),
            "Expected the Instruction.swift file to reference DummyType2"
        );
        assert!(
            clipboard_content.contains("// TODO: - Let's fix TriggerCommentType"),
            "Expected the TODO comment to appear in the prompt"
        );
        assert!(
            !clipboard_content.contains("The contents of Ref.swift is as follows:"),
            "Did not expect Ref.swift to be included in the prompt"
        );
        assert!(
            !clipboard_content.contains("The contents of OldTodo.swift is as follows:"),
            "Did not expect OldTodo.swift to be included in the prompt"
        );
        assert!(
            !clipboard_content.contains("Old marker"),
            "Did not expect the old TODO marker to appear in the prompt"
        );
    }

    /// Integration test for singular mode.
    /// Expects that generate_prompt (with --singular) will include only the Instruction.swift file,
    /// excluding definition files, Ref.swift, and OldTodo.swift.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_singular_mode_includes_only_todo_file() {
        let (project_dir, instruction_file_path) = setup_dummy_project();
        let project_path = project_dir.path();

        env::set_var("GET_INSTRUCTION_FILE", instruction_file_path.to_str().unwrap()); // FIXME: hack workaround
        env::remove_var("DISABLE_PBCOPY");

        let (pbcopy_dir, clipboard_file) = setup_dummy_pbcopy();
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--singular");
        cmd.assert().success();

        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        assert!(
            clipboard_content.contains("The contents of Instruction.swift is as follows:"),
            "Expected clipboard to include the Instruction.swift file header"
        );
        assert!(
            !clipboard_content.contains("The contents of Definition1.swift is as follows:"),
            "Expected Definition1.swift header to be absent in singular mode"
        );
        assert!(
            !clipboard_content.contains("The contents of Definition2.swift is as follows:"),
            "Expected Definition2.swift header to be absent in singular mode"
        );
        assert!(
            clipboard_content.contains("DummyType1"),
            "Expected the Instruction.swift file to reference DummyType1"
        );
        assert!(
            clipboard_content.contains("DummyType2"),
            "Expected the Instruction.swift file to reference DummyType2"
        );
        assert!(
            clipboard_content.contains("// TODO: - Let's fix TriggerCommentType"),
            "Expected the TODO comment to appear in the prompt"
        );
        assert!(
            !clipboard_content.contains("The contents of Ref.swift is as follows:"),
            "Did not expect Ref.swift to be included in singular mode"
        );
        assert!(
            !clipboard_content.contains("The contents of OldTodo.swift is as follows:"),
            "Did not expect OldTodo.swift to be included in singular mode"
        );
        assert!(
            !clipboard_content.contains("Old marker"),
            "Did not expect the old TODO marker to appear in the prompt"
        );
    }

    /// Integration test for include-references mode.
    /// Expects that generate_prompt (with --include-references) will include Ref.swift,
    /// in addition to the Instruction.swift and definition files, while still excluding OldTodo.swift.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_include_references_includes_ref_file() {
        let (project_dir, instruction_file_path) = setup_dummy_project();
        let project_path = project_dir.path();

        env::set_var("GET_INSTRUCTION_FILE", instruction_file_path.to_str().unwrap()); // FIXME: hack workaround
        env::remove_var("DISABLE_PBCOPY");

        let (pbcopy_dir, clipboard_file) = setup_dummy_pbcopy();
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--include-references");
        cmd.assert().success();

        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        assert!(
            clipboard_content.contains("The contents of Instruction.swift is as follows:"),
            "Expected clipboard to include the Instruction.swift file header"
        );
        assert!(
            clipboard_content.contains("The contents of Definition1.swift is as follows:"),
            "Expected clipboard to include Definition1.swift header"
        );
        assert!(
            clipboard_content.contains("The contents of Definition2.swift is as follows:"),
            "Expected clipboard to include Definition2.swift header"
        );
        assert!(
            clipboard_content.contains("// TODO: - Let's fix TriggerCommentType"),
            "Expected the TODO comment to appear in the prompt"
        );
        assert!(
            clipboard_content.contains("The contents of Ref.swift is as follows:"),
            "Expected Ref.swift to be included with --include-references"
        );
        assert!(
            clipboard_content.contains("let instance = SomeClass()"),
            "Expected the content of Ref.swift to appear in the prompt"
        );
        assert!(
            !clipboard_content.contains("The contents of OldTodo.swift is as follows:"),
            "Did not expect OldTodo.swift to be included in the prompt"
        );
        assert!(
            !clipboard_content.contains("Old marker"),
            "Did not expect the old TODO marker to appear in the prompt"
        );
    }
    
    /// Integration test for exclusion flags.
    /// Here we run generate_prompt with the --exclude flag for "Definition1.swift".
    /// We expect that the final prompt includes the Instruction.swift file and Definition2.swift,
    /// but does NOT include Definition1.swift.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_excludes_definition1() {
        let (project_dir, instruction_file_path) = setup_dummy_project();
        let project_path = project_dir.path();

        env::set_var("GET_INSTRUCTION_FILE", instruction_file_path.to_str().unwrap()); // FIXME: hack workaround
        env::remove_var("DISABLE_PBCOPY");

        let (pbcopy_dir, clipboard_file) = setup_dummy_pbcopy();
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        // Run generate_prompt with the exclusion flag for "Definition1.swift"
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--exclude").arg("Definition1.swift");
        cmd.assert().success();

        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Assert that the prompt still includes the Instruction.swift file header.
        assert!(
            clipboard_content.contains("The contents of Instruction.swift is as follows:"),
            "Expected clipboard to include the Instruction.swift file header"
        );
        // Assert that the prompt does NOT include the header for Definition1.swift.
        assert!(
            !clipboard_content.contains("The contents of Definition1.swift is as follows:"),
            "Expected Definition1.swift to be excluded"
        );
        // Assert that the prompt still includes the header for Definition2.swift.
        assert!(
            clipboard_content.contains("The contents of Definition2.swift is as follows:"),
            "Expected Definition2.swift to be included"
        );
        // Verify that the Instruction.swift file's content (including the TODO comment) is present.
        assert!(
            clipboard_content.contains("// TODO: - Let's fix TriggerCommentType"),
            "Expected the TODO comment to appear in the prompt"
        );
    }

    /// Integration test for force-global mode.
    /// Expects that generate_prompt (with --force-global) will include the file Outside.swift,
    /// which defines DummyType3, in addition to the Instruction.swift and definition files.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_force_global_includes_outside_file() {
        let (project_dir, instruction_file_path) = setup_dummy_project();
        let project_path = project_dir.path();

        env::set_var("GET_INSTRUCTION_FILE", instruction_file_path.to_str().unwrap()); // FIXME: hack workaround
        env::remove_var("DISABLE_PBCOPY");

        let (pbcopy_dir, clipboard_file) = setup_dummy_pbcopy();
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--force-global");
        cmd.assert().success();

        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Verify that the global search has included Outside.swift.
        assert!(
            clipboard_content.contains("The contents of Outside.swift is as follows:"),
            "Expected clipboard to include the Outside.swift file header in force-global mode"
        );
        assert!(
            clipboard_content.contains("class DummyType3 { }"),
            "Expected clipboard to contain the declaration of DummyType3"
        );
    }
    
    /// New Integration test to verify that when the TODO trigger references "TriggerCommentType",
    /// the file TriggerCommentReferenced.swift (which defines TriggerCommentType) is included in the final prompt.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_includes_trigger_referenced_file() {
        let (project_dir, instruction_file_path) = setup_dummy_project();
        let project_path = project_dir.path();

        // Set the GET_INSTRUCTION_FILE to point to Instruction.swift.
        env::set_var("GET_INSTRUCTION_FILE", instruction_file_path.to_str().unwrap());
        env::remove_var("DISABLE_PBCOPY");

        let (pbcopy_dir, clipboard_file) = setup_dummy_pbcopy();
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        // Run generate_prompt (normal mode).
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.assert().success();

        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Verify that TriggerCommentReferenced.swift (defining TriggerCommentType) is included in the final prompt.
        assert!(
            clipboard_content.contains("The contents of TriggerCommentReferenced.swift is as follows:"),
            "Expected TriggerCommentReferenced.swift header in prompt. Got:\n{}",
            clipboard_content
        );
        assert!(
            clipboard_content.contains("class TriggerCommentType { }"),
            "Expected TriggerCommentType declaration in prompt. Got:\n{}",
            clipboard_content
        );
    }
    
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_excludes_comment_referenced_file() {
        let (project_dir, instruction_file_path) = setup_dummy_project();
        let project_path = project_dir.path();

        // Set the GET_INSTRUCTION_FILE to point to Instruction.swift.
        env::set_var("GET_INSTRUCTION_FILE", instruction_file_path.to_str().unwrap());
        env::remove_var("DISABLE_PBCOPY");

        let (pbcopy_dir, clipboard_file) = setup_dummy_pbcopy();
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        // Run generate_prompt (normal mode).
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.assert().success();

        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Assert that CommentReferenced.swift is NOT included in the final prompt.
        assert!(
            !clipboard_content.contains("The contents of CommentReferenced.swift is as follows:"),
            "Expected CommentReferenced.swift to be excluded from prompt, but it was found."
        );
        assert!(
            !clipboard_content.contains("class CommentReferencedType { }"),
            "Expected CommentReferencedType declaration to be excluded from prompt, but it was found."
        );
    }
}

#[cfg(test)]
mod integration_tests_js {
    use assert_cmd::Command;
    use std::env;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_js_singular_mode_cloud_code() {
        // Create a temporary directory for our dummy JS project.
        let temp_dir = TempDir::new().unwrap();
        let main_js_path = temp_dir.path().join("main.js");
        let index_js_path = temp_dir.path().join("index.js");

        // Write main.js with marker lines and a Parse.Cloud.define function.
        let main_js_content = r#"
// v
const foo = "include";
// ^

const bar = "exclude";

Parse.Cloud.define("someCloudCodeFunction", async (request) => {
    
    const someObjectId = "12345";
    const someOtherObjectId = "67890";
    
    return somePromise(someObjectId).then(function(someObject) {
        
        return someOtherPromise(someOtherObjectId);
        
    }).then(function(someOtherObject) {
        
        // TODO: - example only

        return true;
        
    });
    
});
"#;
        fs::write(&main_js_path, main_js_content).unwrap();

        // Write index.js (this file should not be included in singular mode).
        let index_js_content = r#"const example = "example";"#;
        fs::write(&index_js_path, index_js_content).unwrap();

        // Set GET_INSTRUCTION_FILE so that main.js is used as the instruction file.
        env::set_var("GET_INSTRUCTION_FILE", main_js_path.to_str().unwrap());
        // Also set GET_GIT_ROOT to our temporary directory.
        env::set_var("GET_GIT_ROOT", temp_dir.path().to_str().unwrap());

        // Set up a dummy pbcopy executable that writes its stdin to a temporary clipboard file.
        let pbcopy_dir = TempDir::new().unwrap();
        let clipboard_file = pbcopy_dir.path().join("clipboard.txt");
        let dummy_pbcopy_path = pbcopy_dir.path().join("pbcopy");
        fs::write(
            &dummy_pbcopy_path,
            format!("#!/bin/sh\ncat > \"{}\"", clipboard_file.display())
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dummy_pbcopy_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dummy_pbcopy_path, perms).unwrap();
        }
        // Prepend the dummy pbcopy directory to the PATH.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        // Ensure that clipboard copying is enabled.
        env::remove_var("DISABLE_PBCOPY");

        // Run the generate_prompt binary in singular mode.
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--singular");
        cmd.assert().success();

        // Read the dummy clipboard file.
        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Assert that the prompt includes the function definition and TODO line from main.js,
        // includes the marked line "const foo = \"include\";",
        // and does not include the unmarked line "const bar = \"exclude\";" nor the content from index.js.
        assert!(clipboard_content.contains("someCloudCodeFunction"),
                "Expected the prompt to include the function name");
        assert!(clipboard_content.contains("// TODO: - example only"),
                "Expected the prompt to include the TODO comment");
        assert!(clipboard_content.contains("const foo = \"include\";"),
                "Expected the prompt to include the 'include' line");
        assert!(!clipboard_content.contains("const bar = \"exclude\";"),
                "Expected the prompt to not include the 'exclude' line");
        assert!(!clipboard_content.contains("const example = \"example\";"),
                "Expected the prompt to not include index.js content");
    }
    
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_js_singular_mode_function_style_1() {
        // Create a temporary directory for our dummy JS project.
        let temp_dir = TempDir::new().unwrap();
        let main_js_path = temp_dir.path().join("main.js");
        let index_js_path = temp_dir.path().join("index.js");

        // Write main.js with marker lines and a function definition.
        let main_js_content = r#"
// v
const foo = "include";
// ^

const bar = "exclude";

someFunction = function(someParameter) {
    
    const someObjectId = "12345";
    const someOtherObjectId = "67890";
    
    return somePromise(someObjectId).then(function(someObject) {
        
        return someOtherPromise(someOtherObjectId);
        
    }).then(function(someOtherObject) {
        
        // TODO: - example only

        return ParsePromiseas(true);
        
    });
}
"#;
        fs::write(&main_js_path, main_js_content).unwrap();

        // Write index.js (this file should not be included in singular mode).
        let index_js_content = r#"const example = "example";"#;
        fs::write(&index_js_path, index_js_content).unwrap();

        // Set GET_INSTRUCTION_FILE so that main.js is used as the instruction file.
        env::set_var("GET_INSTRUCTION_FILE", main_js_path.to_str().unwrap());
        // Also set GET_GIT_ROOT to our temporary directory.
        env::set_var("GET_GIT_ROOT", temp_dir.path().to_str().unwrap());

        // Set up a dummy pbcopy executable that writes its stdin to a temporary clipboard file.
        let pbcopy_dir = TempDir::new().unwrap();
        let clipboard_file = pbcopy_dir.path().join("clipboard.txt");
        let dummy_pbcopy_path = pbcopy_dir.path().join("pbcopy");
        fs::write(
            &dummy_pbcopy_path,
            format!("#!/bin/sh\ncat > \"{}\"", clipboard_file.display())
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dummy_pbcopy_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dummy_pbcopy_path, perms).unwrap();
        }
        // Prepend the dummy pbcopy directory to the PATH.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        // Ensure that clipboard copying is enabled.
        env::remove_var("DISABLE_PBCOPY");

        // Run the generate_prompt binary in singular mode.
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--singular");
        cmd.assert().success();

        // Read the dummy clipboard file.
        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Assert that the prompt includes the function definition and TODO line from main.js,
        // includes the marked line "const foo = \"include\";",
        // and does not include the unmarked line "const bar = \"exclude\";" nor the content from index.js.
        assert!(clipboard_content.contains("someFunction"),
                "Expected the prompt to include the function name");
        assert!(clipboard_content.contains("// TODO: - example only"),
                "Expected the prompt to include the TODO comment");
        assert!(clipboard_content.contains("const foo = \"include\";"),
                "Expected the prompt to include the 'include' line");
        assert!(!clipboard_content.contains("const bar = \"exclude\";"),
                "Expected the prompt to not include the 'exclude' line");
        assert!(!clipboard_content.contains("const example = \"example\";"),
                "Expected the prompt to not include index.js content");
    }
    
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_js_singular_mode_async() {
        // Create a temporary directory for our dummy JS project.
        let temp_dir = TempDir::new().unwrap();
        let main_js_path = temp_dir.path().join("main.js");
        let index_js_path = temp_dir.path().join("index.js");

        // Write main.js with marker lines and an async function declaration.
        let main_js_content = r#"
// v
const foo = "include";
// ^

const bar = "exclude";

async function someFunction(someParameter) {

    const someObjectId = "12345";
    const someOtherObjectId = "67890";
    
    const someObject = await someAsyncFunction(someObjectId);
    const someOtherObject = await someOtherAsyncFunction(someOtherObjectId);

    // TODO: - example only
    return true;
}
"#;
        fs::write(&main_js_path, main_js_content).unwrap();

        // Write index.js (this file should not be included in singular mode).
        let index_js_content = r#"const example = "example";"#;
        fs::write(&index_js_path, index_js_content).unwrap();

        // Set GET_INSTRUCTION_FILE so that main.js is used as the instruction file.
        env::set_var("GET_INSTRUCTION_FILE", main_js_path.to_str().unwrap());
        // Also set GET_GIT_ROOT to our temporary directory.
        env::set_var("GET_GIT_ROOT", temp_dir.path().to_str().unwrap());

        // Set up a dummy pbcopy executable that writes its stdin to a temporary clipboard file.
        let pbcopy_dir = TempDir::new().unwrap();
        let clipboard_file = pbcopy_dir.path().join("clipboard.txt");
        let dummy_pbcopy_path = pbcopy_dir.path().join("pbcopy");
        fs::write(
            &dummy_pbcopy_path,
            format!("#!/bin/sh\ncat > \"{}\"", clipboard_file.display())
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dummy_pbcopy_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dummy_pbcopy_path, perms).unwrap();
        }
        // Prepend the dummy pbcopy directory to the PATH.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        // Ensure that clipboard copying is enabled.
        env::remove_var("DISABLE_PBCOPY");

        // Run the generate_prompt binary in singular mode.
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--singular");
        cmd.assert().success();

        // Read the dummy clipboard file.
        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Assert that the prompt includes the async function definition and TODO line from main.js,
        // includes the marked line "const foo = \"include\";",
        // and does not include the unmarked line "const bar = \"exclude\";" nor the content from index.js.
        assert!(clipboard_content.contains("someFunction"),
                "Expected the prompt to include the async function name");
        assert!(clipboard_content.contains("// TODO: - example only"),
                "Expected the prompt to include the TODO comment");
        assert!(clipboard_content.contains("const foo = \"include\";"),
                "Expected the prompt to include the 'include' line");
        assert!(!clipboard_content.contains("const bar = \"exclude\";"),
                "Expected the prompt to not include the 'exclude' line");
        assert!(!clipboard_content.contains("const example = \"example\";"),
                "Expected the prompt to not include index.js content");
    }
}

#[cfg(test)]
mod integration_tests_substring_markers_swift {
    use assert_cmd::Command;
    use std::env;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_swift_enclosing_function_outside_markers() {
        // Create a temporary directory for our dummy Swift project.
        let temp_dir = TempDir::new().unwrap();
        let main_swift_path = temp_dir.path().join("main.swift");

        // Write a Swift file that contains:
        // - A substring markers block (only content between "// v" and "// ^" is normally included)
        // - A function 'importantFunction' that is not inside any markers and which contains a TODO marker.
        //
        // In normal processing, only the marker block would be included. However, because the TODO
        // marker ("// TODO: - Correct the computation here") appears outside of any marker block,
        // the enclosing function block (i.e. the entire 'importantFunction' function) is automatically
        // appended to the final prompt.
        let main_swift_content = r#"
import Foundation

// v
// This content is included via substring markers.
print("Included marker content")
// ^

func unimportantFunction() {
    print("This is not inside markers.")
}

func importantFunction() {
    print("This is not inside markers normally.")
    // TODO: - Correct the computation here
    print("Computation ends.")
}
"#;
        fs::write(&main_swift_path, main_swift_content).unwrap();

        // Set environment variables so generate_prompt uses our Swift file.
        env::set_var("GET_INSTRUCTION_FILE", main_swift_path.to_str().unwrap());
        env::set_var("GET_GIT_ROOT", temp_dir.path().to_str().unwrap());

        // Set up a dummy pbcopy executable that writes its stdin to a temporary clipboard file.
        let pbcopy_dir = TempDir::new().unwrap();
        let clipboard_file = pbcopy_dir.path().join("clipboard.txt");
        let dummy_pbcopy_path = pbcopy_dir.path().join("pbcopy");
        fs::write(
            &dummy_pbcopy_path,
            format!("#!/bin/sh\ncat > \"{}\"", clipboard_file.display())
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dummy_pbcopy_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dummy_pbcopy_path, perms).unwrap();
        }
        // Prepend the dummy pbcopy directory to the PATH.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        // Ensure that clipboard copying is enabled.
        env::remove_var("DISABLE_PBCOPY");

        // Run the generate_prompt binary in singular mode.
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--singular");
        cmd.assert().success();

        // Read the output from the dummy clipboard file.
        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Assert that the final prompt includes the content from the substring markers.
        assert!(
            clipboard_content.contains("Included marker content"),
            "Expected marker content to appear in prompt; got:\n{}",
            clipboard_content
        );

        // Assert that the final prompt includes the function definition of 'importantFunction'
        // and the TODO marker comment.
        assert!(
            clipboard_content.contains("importantFunction"),
            "Expected the prompt to include the function 'importantFunction'; got:\n{}",
            clipboard_content
        );
        assert!(
            clipboard_content.contains("// TODO: - Correct the computation here"),
            "Expected the prompt to include the TODO comment; got:\n{}",
            clipboard_content
        );

        // Assert that the final prompt includes an appended enclosing function context.
        // (Typically indicated by a string like "Enclosing function context:" in the output.)
        assert!(
            clipboard_content.contains("Enclosing function context:"),
            "Expected the prompt to include the enclosing function context; got:\n{}",
            clipboard_content
        );
    }
}

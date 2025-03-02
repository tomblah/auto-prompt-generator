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

#[cfg(test)]
mod integration_tests {
    use assert_cmd::Command;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Sets up a dummy Git project with a realistic structure.
    /// - The project root contains a TODO.swift file with content:
    ///       "class SomeClass {\nvar foo: DummyType1? = nil\nvar bar: DummyType2? = nil\n}\n// TODO: - Fix bug"
    /// - In a "Sources" subdirectory, two definition files are created:
    ///       - Definition1.swift defines DummyType1 with a simple declaration.
    ///       - Definition2.swift defines DummyType2 with a simple declaration.
    /// - Additionally, a referencing file (Ref.swift) is created in the project root with content:
    ///       "let instance = SomeClass()"
    ///
    /// Returns a tuple (project_dir, todo_file_path) where project_dir is the TempDir for the project.
    fn setup_dummy_project() -> (TempDir, PathBuf) {
        let project_dir = TempDir::new().unwrap();
        let project_path = project_dir.path();

        // Create the TODO file in the project root with the exact content you specified.
        let todo_file_path = project_path.join("TODO.swift");
        fs::write(
            &todo_file_path,
            "class SomeClass {\nvar foo: DummyType1? = nil\nvar bar: DummyType2? = nil\n}\n// TODO: - Fix bug",
        )
        .unwrap();

        // Create a Sources directory with two definition files.
        let sources_dir = project_path.join("Sources");
        fs::create_dir_all(&sources_dir).unwrap();
        let def1_path = sources_dir.join("Definition1.swift");
        let def2_path = sources_dir.join("Definition2.swift");
        // Definition1.swift defines DummyType1 with a simple declaration.
        fs::write(&def1_path, "class DummyType1 { }").unwrap();
        // Definition2.swift defines DummyType2 with a simple declaration.
        fs::write(&def2_path, "class DummyType2 { }").unwrap();

        // Create an additional referencing file in the project root.
        let ref_file_path = project_path.join("Ref.swift");
        fs::write(&ref_file_path, "let instance = SomeClass()").unwrap();

        (project_dir, todo_file_path)
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
    /// Expects that generate_prompt (when run without --singular or --include-references)
    /// will include the TODO file plus both definition files in the final prompt,
    /// and that the referencing file (Ref.swift) is not included.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_normal_mode_includes_all_files() {
        let (project_dir, todo_file_path) = setup_dummy_project();
        let project_path = project_dir.path();

        env::set_var("GET_GIT_ROOT", project_path.to_str().unwrap());
        env::set_var("GET_INSTRUCTION_FILE", todo_file_path.to_str().unwrap());
        env::remove_var("DISABLE_PBCOPY");

        let (pbcopy_dir, clipboard_file) = setup_dummy_pbcopy();
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.assert().success();

        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Assert that the prompt contains the header for the TODO file.
        assert!(
            clipboard_content.contains("The contents of TODO.swift is as follows:"),
            "Expected clipboard to include the TODO file header"
        );
        // Assert that it also includes headers for both definition files.
        assert!(
            clipboard_content.contains("The contents of Definition1.swift is as follows:"),
            "Expected clipboard to include Definition1.swift header"
        );
        assert!(
            clipboard_content.contains("The contents of Definition2.swift is as follows:"),
            "Expected clipboard to include Definition2.swift header"
        );
        // Verify that the definitions contain the expected declarations.
        assert!(
            clipboard_content.contains("class DummyType1 { }"),
            "Expected clipboard to contain the declaration of DummyType1"
        );
        assert!(
            clipboard_content.contains("class DummyType2 { }"),
            "Expected clipboard to contain the declaration of DummyType2"
        );
        // And check that the TODO file's content references the dummy types and includes the TODO comment.
        assert!(
            clipboard_content.contains("DummyType1"),
            "Expected the TODO file to reference DummyType1"
        );
        assert!(
            clipboard_content.contains("DummyType2"),
            "Expected the TODO file to reference DummyType2"
        );
        assert!(
            clipboard_content.contains("// TODO: - Fix bug"),
            "Expected the TODO comment to appear in the prompt"
        );
        // Assert that the referencing file (Ref.swift) is not included.
        assert!(
            !clipboard_content.contains("The contents of Ref.swift is as follows:"),
            "Did not expect Ref.swift to be included in the prompt"
        );
    }

    /// Integration test for singular mode.
    /// Expects that generate_prompt (when run with --singular) will include only the TODO file
    /// in the final prompt, and that neither the definition files nor the referencing file (Ref.swift) appear.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_singular_mode_includes_only_todo_file() {
        let (project_dir, todo_file_path) = setup_dummy_project();
        let project_path = project_dir.path();

        env::set_var("GET_GIT_ROOT", project_path.to_str().unwrap());
        env::set_var("GET_INSTRUCTION_FILE", todo_file_path.to_str().unwrap());
        env::remove_var("DISABLE_PBCOPY");

        let (pbcopy_dir, clipboard_file) = setup_dummy_pbcopy();
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--singular");
        cmd.assert().success();

        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Assert that the prompt contains only the header for the TODO file.
        assert!(
            clipboard_content.contains("The contents of TODO.swift is as follows:"),
            "Expected clipboard to include the TODO file header"
        );
        // Assert that it does NOT contain headers for the definition files.
        assert!(
            !clipboard_content.contains("The contents of Definition1.swift is as follows:"),
            "Expected Definition1.swift header to be absent in singular mode"
        );
        assert!(
            !clipboard_content.contains("The contents of Definition2.swift is as follows:"),
            "Expected Definition2.swift header to be absent in singular mode"
        );
        // Verify that the TODO file's content still references the dummy types and includes the TODO comment.
        assert!(
            clipboard_content.contains("DummyType1"),
            "Expected the TODO file to reference DummyType1"
        );
        assert!(
            clipboard_content.contains("DummyType2"),
            "Expected the TODO file to reference DummyType2"
        );
        assert!(
            clipboard_content.contains("// TODO: - Fix bug"),
            "Expected the TODO comment to appear in the prompt"
        );
        // Assert that the referencing file (Ref.swift) is not included.
        assert!(
            !clipboard_content.contains("The contents of Ref.swift is as follows:"),
            "Did not expect Ref.swift to be included in the prompt"
        );
    }

    /// Integration test for include-references mode.
    /// With the same project setup but run with --include-references,
    /// we expect the referencing file (Ref.swift) to be included in the final prompt.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_include_references_includes_ref_file() {
        let (project_dir, todo_file_path) = setup_dummy_project();
        let project_path = project_dir.path();

        env::set_var("GET_GIT_ROOT", project_path.to_str().unwrap());
        env::set_var("GET_INSTRUCTION_FILE", todo_file_path.to_str().unwrap());
        env::remove_var("DISABLE_PBCOPY");

        let (pbcopy_dir, clipboard_file) = setup_dummy_pbcopy();
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        // Run generate_prompt with the --include-references flag.
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--include-references");
        cmd.assert().success();

        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Assert that the prompt contains the header for the TODO file.
        assert!(
            clipboard_content.contains("The contents of TODO.swift is as follows:"),
            "Expected clipboard to include the TODO file header"
        );
        // Assert that the prompt includes headers for both definition files.
        assert!(
            clipboard_content.contains("The contents of Definition1.swift is as follows:"),
            "Expected clipboard to include Definition1.swift header"
        );
        assert!(
            clipboard_content.contains("The contents of Definition2.swift is as follows:"),
            "Expected clipboard to include Definition2.swift header"
        );
        // And now assert that the referencing file (Ref.swift) is included.
        assert!(
            clipboard_content.contains("The contents of Ref.swift is as follows:"),
            "Expected Ref.swift to be included in the prompt with --include-references"
        );
        // Also, verify that the content of Ref.swift is present.
        assert!(
            clipboard_content.contains("let instance = SomeClass()"),
            "Expected the content of Ref.swift to appear in the prompt"
        );
    }
}

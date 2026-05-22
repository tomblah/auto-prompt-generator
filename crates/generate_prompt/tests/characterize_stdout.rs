// crates/generate_prompt/tests/characterize_stdout.rs

//
// Characterization tests that lock down the diagnostic stdout/stderr output
// currently emitted by library crates during pipeline execution.
// These tests exist to detect regressions while extracting I/O coupling
// from library code into the binary crate.

#[cfg(test)]
mod characterize_stdout_tests {
    use assert_cmd::Command;
    use filetime::{set_file_mtime, FileTime};
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn setup_swift_project() -> (TempDir, PathBuf) {
        let git_root_dir = TempDir::new().unwrap();
        let git_root_path = git_root_dir.path();

        env::set_var("GET_GIT_ROOT", git_root_path.to_str().unwrap());
        env::remove_var("DIFF_WITH_BRANCH");

        let package_dir = git_root_path.join("my_package");
        fs::create_dir_all(&package_dir).unwrap();
        fs::write(package_dir.join("Package.swift"), "// swift package").unwrap();

        let instruction_file_path = package_dir.join("Instruction.swift");
        fs::write(
            &instruction_file_path,
            "class SomeClass {\n    var foo: DummyType1? = nil\n}\n// TODO: - Fix SomeClass\n",
        )
        .unwrap();
        set_file_mtime(&instruction_file_path, FileTime::from_unix_time(3000, 0)).unwrap();

        let sources_dir = package_dir.join("Sources");
        fs::create_dir_all(&sources_dir).unwrap();
        fs::write(
            sources_dir.join("Definition1.swift"),
            "class DummyType1 { }",
        )
        .unwrap();

        (git_root_dir, instruction_file_path)
    }

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

    /// Characterizes the diagnostic stdout lines emitted during a normal
    /// (non-singular) pipeline run. These lines currently originate from
    /// library code in generate_prompt_core and assemble_prompt.
    #[test]
    #[cfg(unix)]
    fn test_stdout_contains_library_diagnostic_lines() {
        let (_project_dir, instruction_file_path) = setup_swift_project();

        env::set_var(
            "GET_INSTRUCTION_FILE",
            instruction_file_path.to_str().unwrap(),
        );
        env::remove_var("DISABLE_PBCOPY");

        let (pbcopy_dir, _clipboard_file) = setup_dummy_pbcopy();
        let original_path = env::var("PATH").unwrap();
        env::set_var(
            "PATH",
            format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path),
        );

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        let output = cmd.output().expect("failed to run generate_prompt");
        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(
            stdout.contains("Search root:"),
            "Expected 'Search root:' from prompt_generator.rs; got:\n{stdout}"
        );
        assert!(
            stdout.contains("Instruction content:"),
            "Expected 'Instruction content:' from prompt_generator.rs; got:\n{stdout}"
        );
        assert!(
            stdout.contains("Types found:"),
            "Expected 'Types found:' from file_selector.rs; got:\n{stdout}"
        );
        assert!(
            stdout.contains("Files (final list):"),
            "Expected 'Files (final list):' from file_selector.rs; got:\n{stdout}"
        );
        assert!(
            stdout.contains("Prompt has been copied to clipboard."),
            "Expected clipboard confirmation from main.rs; got:\n{stdout}"
        );
    }

    /// Characterizes that singular mode emits a specific diagnostic line.
    #[test]
    #[cfg(unix)]
    fn test_stdout_singular_mode_diagnostic() {
        let (_project_dir, instruction_file_path) = setup_swift_project();

        env::set_var(
            "GET_INSTRUCTION_FILE",
            instruction_file_path.to_str().unwrap(),
        );

        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        let output = cmd.arg("--singular").output().expect("failed to run");
        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(
            stdout.contains("Singular mode enabled: only including the TODO file"),
            "Expected singular mode diagnostic from file_selector.rs; got:\n{stdout}"
        );
        assert!(
            !stdout.contains("Types found:"),
            "Singular mode should NOT emit 'Types found:'; got:\n{stdout}"
        );
    }

    /// Characterizes that force-global mode emits its diagnostic line.
    #[test]
    #[cfg(unix)]
    fn test_stdout_force_global_diagnostic() {
        let (_project_dir, instruction_file_path) = setup_swift_project();

        env::set_var(
            "GET_INSTRUCTION_FILE",
            instruction_file_path.to_str().unwrap(),
        );

        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        let output = cmd.arg("--force-global").output().expect("failed to run");
        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(
            stdout.contains("Force global enabled: using Git root for context"),
            "Expected force-global diagnostic from prompt_generator.rs; got:\n{stdout}"
        );
    }
}

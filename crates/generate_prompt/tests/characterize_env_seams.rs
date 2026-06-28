// crates/generate_prompt/tests/characterize_env_seams.rs

//
// Characterization tests that pin the binary-level contract around the
// environment-variable seams (GET_GIT_ROOT, GET_INSTRUCTION_FILE,
// DISABLE_PBCOPY), verbose logging, and current-directory independence.
//
// These tests exist to detect regressions while the env seams and the global
// process mutation (set_current_dir / RUST_LOG set_var) are moved to the
// binary edge. They drive the compiled binary and scope every environment
// variable to the spawned command so they do not depend on process-global
// state or test execution order.

#[cfg(test)]
mod characterize_env_seams_tests {
    use assert_cmd::Command;
    use predicates::prelude::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Builds a minimal Swift package with a single instruction file and one
    /// definition file. Returns the git-root TempDir and the instruction path.
    fn setup_swift_project() -> (TempDir, PathBuf) {
        let git_root_dir = TempDir::new().unwrap();
        let git_root_path = git_root_dir.path();

        let package_dir = git_root_path.join("my_package");
        fs::create_dir_all(&package_dir).unwrap();
        fs::write(package_dir.join("Package.swift"), "// swift package").unwrap();

        let instruction_file_path = package_dir.join("Instruction.swift");
        fs::write(
            &instruction_file_path,
            "class SomeClass {\n    var foo: DummyType1? = nil\n}\n// TODO: - Fix SomeClass\n",
        )
        .unwrap();

        let sources_dir = package_dir.join("Sources");
        fs::create_dir_all(&sources_dir).unwrap();
        fs::write(
            sources_dir.join("Definition1.swift"),
            "class DummyType1 { }",
        )
        .unwrap();

        (git_root_dir, instruction_file_path)
    }

    /// Pins that GET_GIT_ROOT and GET_INSTRUCTION_FILE are honored as edge
    /// overrides: the binary uses both without performing real discovery.
    #[test]
    fn test_env_overrides_are_honored() {
        let (git_root_dir, instruction_file_path) = setup_swift_project();

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.env("GET_GIT_ROOT", git_root_dir.path())
            .env("GET_INSTRUCTION_FILE", &instruction_file_path)
            .env("DISABLE_PBCOPY", "1")
            .env_remove("RUST_LOG");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Found exactly one instruction in"))
            .stdout(predicate::str::contains(
                instruction_file_path.display().to_string(),
            ))
            .stdout(predicate::str::contains(
                "Instruction content: // TODO: - Fix SomeClass",
            ));
    }

    /// Pins that DISABLE_PBCOPY skips the clipboard copy and emits the
    /// skip notice on stderr, while the success diagnostics still print.
    #[test]
    fn test_disable_pbcopy_skips_clipboard_and_warns() {
        let (git_root_dir, instruction_file_path) = setup_swift_project();

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.env("GET_GIT_ROOT", git_root_dir.path())
            .env("GET_INSTRUCTION_FILE", &instruction_file_path)
            .env("DISABLE_PBCOPY", "1");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains(
                "Prompt has been copied to clipboard.",
            ))
            .stderr(predicate::str::contains(
                "DISABLE_PBCOPY is set; skipping clipboard copy.",
            ));
    }

    /// Pins that the binary produces the prompt regardless of the directory it
    /// is launched from, given explicit GET_GIT_ROOT / GET_INSTRUCTION_FILE
    /// overrides. This protects the removal of the global set_current_dir.
    #[test]
    fn test_runs_from_unrelated_working_directory() {
        let (git_root_dir, instruction_file_path) = setup_swift_project();
        let unrelated_dir = TempDir::new().unwrap();

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.current_dir(unrelated_dir.path())
            .env("GET_GIT_ROOT", git_root_dir.path())
            .env("GET_INSTRUCTION_FILE", &instruction_file_path)
            .env("DISABLE_PBCOPY", "1");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Success:"))
            .stdout(predicate::str::contains("// TODO: - Fix SomeClass"));
    }

    /// Pins that --verbose enables debug logging on stderr (a line that is only
    /// emitted via the `debug!` macro), and that a normal run does not.
    #[test]
    fn test_verbose_enables_debug_logging() {
        let (git_root_dir, instruction_file_path) = setup_swift_project();

        let mut verbose_cmd = Command::cargo_bin("generate_prompt").unwrap();
        verbose_cmd
            .arg("--verbose")
            .env("GET_GIT_ROOT", git_root_dir.path())
            .env("GET_INSTRUCTION_FILE", &instruction_file_path)
            .env("DISABLE_PBCOPY", "1")
            .env_remove("RUST_LOG");
        let verbose_output = verbose_cmd.output().expect("failed to run with --verbose");
        assert!(
            verbose_output.status.success(),
            "expected --verbose run to succeed"
        );
        let verbose_stderr = String::from_utf8_lossy(&verbose_output.stderr);
        assert!(
            verbose_stderr.contains("Search root:"),
            "Expected --verbose to emit the 'Search root:' debug line on stderr; got:\n{verbose_stderr}"
        );

        let mut quiet_cmd = Command::cargo_bin("generate_prompt").unwrap();
        quiet_cmd
            .env("GET_GIT_ROOT", git_root_dir.path())
            .env("GET_INSTRUCTION_FILE", &instruction_file_path)
            .env("DISABLE_PBCOPY", "1")
            .env_remove("RUST_LOG");
        let quiet_output = quiet_cmd.output().expect("failed to run without --verbose");
        assert!(
            quiet_output.status.success(),
            "expected non-verbose run to succeed"
        );
        let quiet_stderr = String::from_utf8_lossy(&quiet_output.stderr);
        assert!(
            !quiet_stderr.contains("Search root:"),
            "Expected non-verbose run to omit the 'Search root:' debug line on stderr; got:\n{quiet_stderr}"
        );
    }
}

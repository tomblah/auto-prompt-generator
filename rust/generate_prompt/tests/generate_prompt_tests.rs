use assert_cmd::Command;
use predicates::prelude::*;
use std::env;
use std::fs;
use std::io::Write;
use tempfile::TempDir;

/// A helper that sets an environment variable to a new value and restores
/// its previous value (or unsets it) when dropped.
struct EnvVarGuard<'a> {
    key: &'a str,
    old_value: Option<String>,
}

impl<'a> EnvVarGuard<'a> {
    fn new(key: &'a str, value: &str) -> Self {
        let old_value = env::var(key).ok();
        env::set_var(key, value);
        Self { key, old_value }
    }
}

impl<'a> Drop for EnvVarGuard<'a> {
    fn drop(&mut self) {
        if let Some(ref val) = self.old_value {
            env::set_var(self.key, val);
        } else {
            env::remove_var(self.key);
        }
    }
}

/// On Unix systems, creates a dummy executable (a shell script) in the given temporary directory.
/// The script simply echoes the provided output.
#[cfg(unix)]
fn create_dummy_executable(dir: &TempDir, name: &str, output: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, format!("#!/bin/sh\necho \"{}\"", output)).unwrap();
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

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
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        // Force the Git root to our fake directory.
        let _git_root_guard = EnvVarGuard::new("GET_GIT_ROOT", fake_git_root_path);

        // Create a TODO file in the fake Git root.
        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        fs::write(&todo_file, "   // TODO: - Force global test").unwrap();
        let _instruction_guard = EnvVarGuard::new("GET_INSTRUCTION_FILE", &todo_file);

        // Set up dummy executables.
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "NonEmptyValue");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Force global test");
        // Create a dummy types file.
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TypeForce").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
        // Dummy definition file.
        let def_files_output = format!("{}/Definition.swift", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_definition_files", &def_files_output);
        create_dummy_executable(&temp_dir, "filter_excluded_files", &def_files_output);
        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--force-global");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Force global enabled: using Git root for context"));
    }

    /// Test that when exclusion flags are provided the exclusion branch is taken.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_exclude() {
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        // Set our fake Git root.
        let _git_root_guard = EnvVarGuard::new("GET_GIT_ROOT", fake_git_root_path);

        // Create a valid Swift TODO file containing a type declaration.
        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        fs::write(
            &todo_file,
            "class TypeExclude {\n    // TODO: - Exclude test\n}"
        ).unwrap();
        let _instruction_guard = EnvVarGuard::new("GET_INSTRUCTION_FILE", &todo_file);
        let _disable_pbcopy_guard = EnvVarGuard::new("DISABLE_PBCOPY", "1");

        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "class TypeExclude {\n    // TODO: - Exclude test\n}");

        // Create two Swift definition files in the fake Git root.
        let def_file1 = format!("{}/Definition1.swift", fake_git_root_path);
        let def_file2 = format!("{}/Definition2.swift", fake_git_root_path);
        fs::write(&def_file1, "class TypeExclude {}").unwrap();
        fs::write(&def_file2, "struct TypeExclude {}").unwrap();

        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.args(&["--exclude", "ExcludePattern", "--exclude", "AnotherPattern"]);

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Excluding files matching:"))
            .stdout(predicate::str::contains("Definition1.swift"))
            .stdout(predicate::str::contains("Definition2.swift"));
    }

    /// Test that generate_prompt exits with an error when multiple markers are present.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_multiple_markers() {
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        let _git_root_guard = EnvVarGuard::new("GET_GIT_ROOT", fake_git_root_path);

        let instruction_path = format!("{}/Instruction.swift", fake_git_root_path);
        fs::write(&instruction_path, "   // TODO: - Fix issue").unwrap();
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &instruction_path);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Fix issue");

        let types_file = temp_dir.path().join("types.txt");
        fs::write(&types_file, "TypeA").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file.to_str().unwrap());

        create_dummy_executable(&temp_dir, "find_definition_files", "dummy_definitions");

        create_dummy_executable(&temp_dir, "filter_files_singular", &instruction_path);

        let multi_marker_prompt = "\
The contents of Instruction.swift is as follows:\n\n\
// TODO: - Marker One\nSome content here\n\n\
// TODO: - Marker Two\nMore content here\n\n\
 // TODO: -\n";
        create_dummy_executable(&temp_dir, "assemble_prompt", multi_marker_prompt);

        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.assert().failure()
            .stderr(predicate::str::contains("Multiple // TODO: - markers found. Exiting."));
    }
}

mod tests {
    use super::*;
    use assert_cmd::prelude::*;
    use std::env;
    use std::fs;
    use tempfile::TempDir;
    use predicates::prelude::*;

    /// In singular mode, only the TODO file should be included.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_singular_mode() {
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        let _git_root_guard = EnvVarGuard::new("GET_GIT_ROOT", fake_git_root_path);

        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        fs::write(&todo_file, "   // TODO: - Fix issue").unwrap();
        let _instruction_guard = EnvVarGuard::new("GET_INSTRUCTION_FILE", &todo_file);

        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Fix issue");
        create_dummy_executable(&temp_dir, "filter_files_singular", &todo_file);
        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--singular");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Found exactly one instruction in"))
            .stdout(predicate::str::contains("Singular mode enabled: only including the TODO file"))
            .stdout(predicate::str::contains("// TODO: - Fix issue"))
            .stdout(predicate::str::contains("Prompt has been copied to clipboard."));
    }

    /// Test that when --include-references is used but the TODO file isn’t a Swift file, we error out.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_include_references_error_for_non_swift() {
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        let _git_root_guard = EnvVarGuard::new("GET_GIT_ROOT", fake_git_root_path);

        // TODO file with .js extension.
        let todo_file = format!("{}/TODO.js", fake_git_root_path);
        fs::write(&todo_file, "   // TODO: - Fix issue").unwrap();
        let _instruction_guard = EnvVarGuard::new("GET_INSTRUCTION_FILE", &todo_file);

        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
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

        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--include-references");

        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("--include-references is only supported for Swift files"));
    }

    /// Test inclusion of referencing files for Swift.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_include_references_for_swift() {
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        let _git_root_guard = EnvVarGuard::new("GET_GIT_ROOT", fake_git_root_path);

        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        fs::write(&todo_file, "   // TODO: - Fix bug").unwrap();
        let _instruction_guard = EnvVarGuard::new("GET_INSTRUCTION_FILE", &todo_file);

        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Fix bug");

        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TypeA").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
        let def_file = format!("{}/Definition.swift", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_definition_files", &def_file);
        create_dummy_executable(&temp_dir, "filter_files_singular", &todo_file);

        create_dummy_executable(&temp_dir, "extract_enclosing_type", "MyType");
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
            .stdout(predicate::str::contains("Success:"))
            .stdout(predicate::str::contains("Prompt has been copied to clipboard."));
    }

    /// Test a normal (non‑singular) run.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_normal_mode() {
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        let _git_root_guard = EnvVarGuard::new("GET_GIT_ROOT", fake_git_root_path);

        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        fs::write(&todo_file, "   // TODO: - Fix bug").unwrap();
        let _instruction_guard = EnvVarGuard::new("GET_INSTRUCTION_FILE", &todo_file);

        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Fix bug");
        
        // Create a dummy types file with a type.
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TypeA").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
        
        // Create two definition files in the fake Git root.
        let def_file1 = format!("{}/Definition1.swift", fake_git_root_path);
        let def_file2 = format!("{}/Definition2.swift", fake_git_root_path);
        fs::write(&def_file1, "class TypeA {}").unwrap();
        fs::write(&def_file2, "struct TypeA {}").unwrap();

        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

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
            .stdout(predicate::str::contains("Success:"))
            .stdout(predicate::str::contains("Prompt has been copied to clipboard."));
    }
}

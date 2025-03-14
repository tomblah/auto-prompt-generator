// tests/generate_prompt_integration.rs

use assert_cmd::Command;
use predicates::prelude::*;
use std::env;
use std::fs;
use tempfile::TempDir;

/// Helper to remove GET_GIT_ROOT (so that stale values don’t affect tests)
fn clear_git_root() {
    env::remove_var("GET_GIT_ROOT");
}

/// On Unix systems, creates a dummy executable (a shell script) in the given temporary directory.
/// The script simply echoes the provided output.
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

/// --- Test: Singular Mode ---
/// In singular mode, only the TODO file should be included.
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
    // Create a TODO file with expected content.
    fs::write(&todo_file, "   // TODO: - Fix critical bug").unwrap();
    create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
    create_dummy_executable(&temp_dir, "get_package_root", "");
    create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Fix critical bug");
    create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

    // Override the instruction file.
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

/// --- Test: Include References Error for Non‑Swift Input ---
/// When using --include-references on a non‑Swift file (e.g. a .js file),
/// the program should exit with an error.
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
    // Set up a dummy types file and dummy definition file command.
    let types_file_path = temp_dir.path().join("types.txt");
    fs::write(&types_file_path, "DummyType").unwrap();
    create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
    let def_file = format!("{}/Definition.swift", fake_git_root_path);
    create_dummy_executable(&temp_dir, "find_definition_files", &def_file);
    create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

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

/// --- Test: Normal Mode ---
/// Verifies that when not in singular mode the prompt includes the instruction
/// plus the definitions (and that the clipboard copy occurs).
#[test]
#[cfg(unix)]
fn test_generate_prompt_normal_mode() {
    let temp_dir = TempDir::new().unwrap();
    let fake_git_root = TempDir::new().unwrap();
    let fake_git_root_path = fake_git_root.path().to_str().unwrap();

    env::set_var("GET_GIT_ROOT", fake_git_root_path);

    let todo_file = format!("{}/TODO.swift", fake_git_root_path);
    // Create a TODO file that also contains a type declaration.
    fs::write(&todo_file, "class TypeFixBug {}\n   // TODO: - Fix bug").unwrap();
    create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
    create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
    create_dummy_executable(&temp_dir, "get_package_root", "");
    create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Fix bug");

    env::set_var("GET_INSTRUCTION_FILE", &todo_file);
    // Create a dummy types file.
    let types_file_path = temp_dir.path().join("types.txt");
    fs::write(&types_file_path, "TypeFixBug").unwrap();
    create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());

    // Create two dummy definition files.
    let def_file1 = fake_git_root.path().join("Definition1.swift");
    fs::write(&def_file1, "class TypeFixBug {}").unwrap();
    let def_file2 = fake_git_root.path().join("Definition2.swift");
    fs::write(&def_file2, "class TypeFixBug {}").unwrap();

    // Dummy assemble_prompt (clipboard copy occurs).
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

/// --- Test: Include References for Swift ---
/// With --include-references on a Swift file, extra files (e.g. those referencing the enclosing type)
/// should be included.
#[test]
#[cfg(unix)]
fn test_generate_prompt_include_references_for_swift() {
    clear_git_root();
    let temp_dir = TempDir::new().unwrap();
    let fake_git_root = TempDir::new().unwrap();
    let fake_git_root_path = fake_git_root.path().to_str().unwrap();

    create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
    let todo_file = format!("{}/TODO.swift", fake_git_root_path);
    // The TODO file contains a type declaration so that the extractor finds "MyType".
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

/// --- Test: Force Global Mode ---
/// When --force-global is passed, the Git root should be used as the context even if it is not a Swift package.
#[test]
#[cfg(unix)]
fn test_generate_prompt_force_global() {
    let temp_dir = TempDir::new().unwrap();
    let fake_git_root = TempDir::new().unwrap();
    let fake_git_root_path = fake_git_root.path().to_str().unwrap();

    // Set GET_GIT_ROOT to the fake Git root.
    env::set_var("GET_GIT_ROOT", fake_git_root_path);

    let todo_file = format!("{}/TODO.swift", fake_git_root_path);
    fs::write(&todo_file, "class TypeForce {}\n   // TODO: - Force global test").unwrap();
    create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
    create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
    // Simulate a non‑empty package root.
    create_dummy_executable(&temp_dir, "get_package_root", "NonEmptyValue");
    create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Force global test");
    let types_file_path = temp_dir.path().join("types.txt");
    fs::write(&types_file_path, "TypeForce").unwrap();
    create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
    let def_files_output = format!("{}/Definition.swift", fake_git_root_path);
    create_dummy_executable(&temp_dir, "find_definition_files", &def_files_output);
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

/// --- Test: Exclusion Flags ---
/// Verify that when using --exclude, files whose basenames match the given patterns are removed.
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

/// --- Test: Multiple Marker Scrubbing ---
/// When multiple TODO markers appear, only the primary (first) marker and the final CTA marker should be present.
#[test]
#[cfg(unix)]
fn test_generate_prompt_multiple_markers() {
    // Set up dummy pbcopy to capture clipboard output.
    let temp_dir = TempDir::new().unwrap();
    let clipboard_file = temp_dir.path().join("dummy_clipboard.txt");
    let pbcopy_script = format!("cat > \"{}\"", clipboard_file.display());
    create_dummy_executable(&temp_dir, "pbcopy", &pbcopy_script);

    let fake_git_root = TempDir::new().unwrap();
    let fake_git_root_path = fake_git_root.path().to_str().unwrap();

    create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);

    // Create an instruction file with three markers.
    let instruction_path = format!("{}/Instruction.swift", fake_git_root_path);
    let multi_marker_content = "\
        // TODO: - Marker One\n\
        Some content here\n\
        // TODO: - Marker Two\n\
        More content here\n\
        // TODO: - CTA Marker\n";
    fs::write(&instruction_path, multi_marker_content).unwrap();
    env::set_var("GET_INSTRUCTION_FILE", &instruction_path);

    create_dummy_executable(&temp_dir, "find_prompt_instruction", &instruction_path);
    create_dummy_executable(&temp_dir, "extract_instruction_content", "// TODO: - Marker One");
    create_dummy_executable(&temp_dir, "get_package_root", "");
    create_dummy_executable(&temp_dir, "assemble_prompt", multi_marker_content);

    let original_path = env::var("PATH").unwrap();
    env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
    env::remove_var("DISABLE_PBCOPY");

    let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
    cmd.assert().success();

    let clipboard_content = fs::read_to_string(&clipboard_file)
        .expect("Failed to read dummy clipboard file");

    assert!(clipboard_content.contains("// TODO: - Marker One"),
            "Clipboard missing primary marker: {}", clipboard_content);
    assert!(clipboard_content.contains("Can you do the TODO:- in the above code?"),
            "Clipboard missing CTA marker: {}", clipboard_content);
    assert!(!clipboard_content.contains("// TODO: - Marker Two"),
            "Clipboard should not contain extra marker: {}", clipboard_content);

    env::remove_var("GET_GIT_ROOT");
}

/// --- Test: Diff With Main Branch ---
/// When diff mode is enabled (e.g. via --diff-with "main"), a diff section should be appended.
#[test]
#[cfg(unix)]
fn test_generate_prompt_diff_with_main() {
    let temp_dir = TempDir::new().unwrap();
    // Create a dummy git executable that simulates diff behavior.
    let git_script = r#"#!/bin/sh
case "$@" in
    *rev-parse*--verify*main*)
        exit 0
        ;;
    *ls-files*)
        exit 0
        ;;
    *diff*)
        echo "dummy diff output"
        exit 0
        ;;
    *)
        exit 1
        ;;
esac
"#;
    create_dummy_executable(&temp_dir, "git", git_script);

    let clipboard_file = temp_dir.path().join("dummy_clipboard.txt");
    let pbcopy_script = format!("cat > \"{}\"", clipboard_file.display());
    create_dummy_executable(&temp_dir, "pbcopy", &pbcopy_script);

    let fake_git_root = TempDir::new().unwrap();
    let fake_git_root_path = fake_git_root.path().to_str().unwrap();
    env::set_var("GET_GIT_ROOT", fake_git_root_path);

    let todo_file = format!("{}/TODO.swift", fake_git_root_path);
    fs::write(&todo_file, "class TestDiff {}\n   // TODO: - Diff test").unwrap();
    env::set_var("GET_INSTRUCTION_FILE", &todo_file);
    create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
    create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
    create_dummy_executable(&temp_dir, "get_package_root", "");
    create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Diff test");
    let types_file_path = temp_dir.path().join("types.txt");
    fs::write(&types_file_path, "TestDiff").unwrap();
    create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
    let def_file = format!("{}/Definition.swift", fake_git_root_path);
    fs::write(&def_file, "class TestDiff {}").unwrap();
    let find_def_script = format!("echo \"{}\"", def_file);
    create_dummy_executable(&temp_dir, "find_definition_files", &find_def_script);
    create_dummy_executable(&temp_dir, "filter_excluded_files", "");
    create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

    env::remove_var("DISABLE_PBCOPY");
    let original_path = env::var("PATH").unwrap();
    env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));

    let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
    cmd.args(&["--diff-with", "main"]);
    cmd.assert().success();

    let clipboard_content = fs::read_to_string(&clipboard_file)
        .expect("Failed to read dummy clipboard file");

    assert!(clipboard_content.contains("The diff for"),
            "Clipboard content missing diff header: {}", clipboard_content);
    assert!(clipboard_content.contains("against branch main"),
            "Clipboard content missing branch info: {}", clipboard_content);
    assert!(clipboard_content.contains("dummy diff output"),
            "Clipboard content missing dummy diff output: {}", clipboard_content);

    env::remove_var("GET_GIT_ROOT");
}

/// --- Test: Diff With Non‑existent Branch ---
/// When a branch specified by DIFF_WITH_BRANCH does not exist, the program should fail.
#[test]
#[cfg(unix)]
fn test_generate_prompt_diff_with_nonexistent_branch() {
    let temp_dir = TempDir::new().unwrap();
    let git_script = r#"#!/bin/sh
case "$@" in
    *rev-parse*--verify*nonexistent*)
        echo "fatal: ambiguous argument 'nonexistent': unknown revision or path not in the working tree." >&2
        exit 1
        ;;
    *)
        exit 0
        ;;
esac
"#;
    create_dummy_executable(&temp_dir, "git", git_script);

    let clipboard_file = temp_dir.path().join("dummy_clipboard.txt");
    let pbcopy_script = format!("cat > \"{}\"", clipboard_file.display());
    create_dummy_executable(&temp_dir, "pbcopy", &pbcopy_script);

    let fake_git_root = TempDir::new().unwrap();
    let fake_git_root_path = fake_git_root.path().to_str().unwrap();
    env::set_var("GET_GIT_ROOT", fake_git_root_path);

    let todo_file = format!("{}/TODO.swift", fake_git_root_path);
    fs::write(&todo_file, "class TestDiff {}\n   // TODO: - Diff test").unwrap();
    env::set_var("GET_INSTRUCTION_FILE", &todo_file);
    create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
    create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
    create_dummy_executable(&temp_dir, "get_package_root", "");
    create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Diff test");
    let types_file_path = temp_dir.path().join("types.txt");
    fs::write(&types_file_path, "TestDiff").unwrap();
    create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
    let def_file = format!("{}/Definition.swift", fake_git_root_path);
    fs::write(&def_file, "class TestDiff {}").unwrap();
    let find_def_script = format!("echo \"{}\"", def_file);
    create_dummy_executable(&temp_dir, "find_definition_files", &find_def_script);
    create_dummy_executable(&temp_dir, "filter_excluded_files", "");
    create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

    let original_path = env::var("PATH").unwrap();
    env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
    env::remove_var("DISABLE_PBCOPY");

    let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
    cmd.args(&["--diff-with", "nonexistent"]);

    cmd.assert()
       .failure()
       .stderr(predicate::str::contains("Error: Branch 'nonexistent' does not exist."));

    env::remove_var("GET_GIT_ROOT");
}

/// --- Test: Final Prompt Copied to Clipboard ---
/// Ensure that when the program runs, the final prompt is actually copied to the clipboard.
#[test]
#[cfg(unix)]
fn test_final_prompt_copied_to_clipboard() {
    let temp_dir = TempDir::new().unwrap();
    let clipboard_file = temp_dir.path().join("dummy_clipboard.txt");
    let pbcopy_script = format!("cat > \"{}\"", clipboard_file.display());
    create_dummy_executable(&temp_dir, "pbcopy", &pbcopy_script);

    let fake_git_root = TempDir::new().unwrap();
    let fake_git_root_path = fake_git_root.path().to_str().unwrap();
    env::set_var("GET_GIT_ROOT", fake_git_root_path);

    let todo_file = format!("{}/TODO.swift", fake_git_root_path);
    fs::write(&todo_file, "class TypeFixBug {}\n   // TODO: - Fix bug").unwrap();
    create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
    create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
    create_dummy_executable(&temp_dir, "get_package_root", "");
    create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Fix bug");

    let types_file_path = temp_dir.path().join("types.txt");
    fs::write(&types_file_path, "TypeFixBug").unwrap();
    create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
    let def_file1 = fake_git_root.path().join("Definition1.swift");
    fs::write(&def_file1, "class TypeFixBug {}").unwrap();
    let def_file2 = fake_git_root.path().join("Definition2.swift");
    fs::write(&def_file2, "class TypeFixBug {}").unwrap();
    let simulated_prompt = "\
The contents of Definition1.swift is as follows:\n\nclass TypeFixBug {}\n\n--------------------------------------------------\nThe contents of Definition2.swift is as follows:\n\nclass TypeFixBug {}\n\n--------------------------------------------------\n\nCan you do the TODO:- in the above code? But ignoring all FIXMEs and other TODOs...";
    create_dummy_executable(&temp_dir, "assemble_prompt", simulated_prompt);

    env::set_var("GET_INSTRUCTION_FILE", &todo_file);
    let original_path = env::var("PATH").unwrap();
    env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
    env::remove_var("DISABLE_PBCOPY");

    let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
    cmd.assert().success();

    let clipboard_content = fs::read_to_string(&clipboard_file)
        .expect("Failed to read dummy clipboard file");

    assert!(clipboard_content.contains("Can you do the TODO:- in the above code? But ignoring all FIXMEs"),
            "Clipboard content did not contain the expected fixed instruction: {}", clipboard_content);

    env::remove_var("GET_GIT_ROOT");
}

/// --- Test: Final Prompt Formatting with Multiple Files ---
/// Check that when multiple definition files are included, the final prompt contains headers for each,
/// plus the fixed instruction appended at the end.
#[test]
#[cfg(unix)]
fn test_final_prompt_formatting_with_multiple_files() {
    let temp_dir = TempDir::new().unwrap();
    let clipboard_file = temp_dir.path().join("dummy_clipboard.txt");
    let pbcopy_script = format!("cat > \"{}\"", clipboard_file.display());
    create_dummy_executable(&temp_dir, "pbcopy", &pbcopy_script);

    let fake_git_root = TempDir::new().unwrap();
    let fake_git_root_path = fake_git_root.path().to_str().unwrap();
    env::set_var("GET_GIT_ROOT", fake_git_root_path);

    let todo_file = format!("{}/TODO.swift", fake_git_root_path);
    fs::write(&todo_file, "class TestClass {}\n   // TODO: - Refactor code").unwrap();

    create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
    create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
    create_dummy_executable(&temp_dir, "get_package_root", "");
    create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Refactor code");

    let types_file_path = temp_dir.path().join("types.txt");
    fs::write(&types_file_path, "TestClass").unwrap();
    create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());

    let def_file1 = fake_git_root.path().join("Definition1.swift");
    fs::write(&def_file1, "class TestClass {}").unwrap();
    let def_file2 = fake_git_root.path().join("Definition2.swift");
    fs::write(&def_file2, "class TestClass {}").unwrap();

    let find_def_script = format!("echo \"{}\\n{}\"", def_file1.display(), def_file2.display());
    create_dummy_executable(&temp_dir, "find_definition_files", &find_def_script);
    create_dummy_executable(&temp_dir, "filter_excluded_files", "");
    let simulated_prompt = format!(
        "The contents of {} is as follows:\n\n{}\n\n--------------------------------------------------\nThe contents of {} is as follows:\n\n{}\n\n--------------------------------------------------\n\nCan you do the TODO:- in the above code? But ignoring all FIXMEs and other TODOs...",
        def_file1.file_name().unwrap().to_string_lossy(),
        fs::read_to_string(&def_file1).unwrap(),
        def_file2.file_name().unwrap().to_string_lossy(),
        fs::read_to_string(&def_file2).unwrap()
    );
    create_dummy_executable(&temp_dir, "assemble_prompt", &simulated_prompt);

    env::set_var("GET_INSTRUCTION_FILE", &todo_file);
    let original_path = env::var("PATH").unwrap();
    env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
    env::remove_var("DISABLE_PBCOPY");

    let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
    cmd.assert().success();

    let clipboard_content = fs::read_to_string(&clipboard_file)
        .expect("Failed to read dummy clipboard file");

    assert!(clipboard_content.contains("The contents of Definition1.swift is as follows:"), "Missing header for Definition1.swift: {}", clipboard_content);
    assert!(clipboard_content.contains("The contents of Definition2.swift is as follows:"), "Missing header for Definition2.swift: {}", clipboard_content);
    assert!(clipboard_content.contains("Can you do the TODO:- in the above code?"), "Missing fixed instruction: {}", clipboard_content);

    env::remove_var("GET_GIT_ROOT");
}

/// --- Test: Scrub Extra TODO Markers ---
/// If extra TODO marker lines appear, only the primary marker and the final CTA marker should remain.
#[test]
#[cfg(unix)]
fn test_generate_prompt_scrubs_extra_todo_markers() {
    let temp_dir = TempDir::new().unwrap();
    let clipboard_file = temp_dir.path().join("dummy_clipboard.txt");
    let pbcopy_script = format!("cat > \"{}\"", clipboard_file.display());
    create_dummy_executable(&temp_dir, "pbcopy", &pbcopy_script);

    let fake_git_root = TempDir::new().unwrap();
    let fake_git_root_path = fake_git_root.path().to_str().unwrap();

    create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);

    // Create a simulated instruction file that contains three TODO markers.
    let todo_file = format!("{}/TODO.swift", fake_git_root_path);
    let simulated_prompt = "\
The contents of Definition.swift is as follows:\n\nclass DummyType {}\n\n--------------------------------------------------\n// TODO: - Primary Marker\nSome extra content here\n// TODO: - Extra Marker\nMore extra content here\n// TODO: - CTA Marker\n";
    fs::write(&todo_file, simulated_prompt).unwrap();
    env::set_var("GET_INSTRUCTION_FILE", &todo_file);

    create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
    create_dummy_executable(&temp_dir, "extract_instruction_content", "// TODO: - Primary Marker");
    create_dummy_executable(&temp_dir, "get_package_root", "");
    create_dummy_executable(&temp_dir, "assemble_prompt", simulated_prompt);

    let original_path = env::var("PATH").unwrap();
    env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
    env::remove_var("DISABLE_PBCOPY");

    let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
    cmd.assert().success();

    let clipboard_content = fs::read_to_string(&clipboard_file)
        .expect("Failed to read dummy clipboard file");

    assert!(clipboard_content.contains("// TODO: - Primary Marker"),
            "Clipboard missing primary marker: {}", clipboard_content);
    assert!(clipboard_content.contains("Can you do the TODO:- in the above code?"),
            "Clipboard missing CTA marker: {}", clipboard_content);
    assert!(!clipboard_content.contains("// TODO: - Extra Marker"),
            "Extra marker was not scrubbed from final prompt: {}", clipboard_content);

    env::remove_var("GET_GIT_ROOT");
}


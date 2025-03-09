use std::env;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use anyhow::{Result, Context};
use substring_marker_snippet_extractor;
use unescape_newlines::unescape_newlines;
// Use the diff_with_branch crate function.
use diff_with_branch::run_diff;

/// Public API: assembles the final prompt from the found files and instruction content.
/// Instead of printing to stdout or copying to clipboard, it returns the prompt as a String.
pub fn assemble_prompt(found_files_file: &str, _instruction_content: &str) -> Result<String> {
    // Read the found_files list.
    let file = File::open(found_files_file)
        .with_context(|| format!("Error opening {}", found_files_file))?;
    let reader = BufReader::new(file);
    let mut files: Vec<String> = reader
        .lines()
        .filter_map(|l| l.ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    files.sort();
    files.dedup();

    let mut final_prompt = String::new();
    // Retrieve TODO file basename from environment.
    let todo_file_basename = env::var("TODO_FILE_BASENAME").unwrap_or_default();

    for file_path in files {
        if !Path::new(&file_path).exists() {
            eprintln!("Warning: file {} does not exist, skipping", file_path);
            continue;
        }
        let basename = Path::new(&file_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&file_path)
            .to_string();

        // Always use the library processing since we no longer have an external prompt processor.
        let processed_content = match substring_marker_snippet_extractor::process_file(&file_path, Some(&todo_file_basename)) {
            Ok(content) => content,
            Err(err) => {
                eprintln!("Error processing {}: {}. Falling back to raw file contents.", file_path, err);
                fs::read_to_string(&file_path).unwrap_or_default()
            }
        };

        final_prompt.push_str(&format!(
            "\nThe contents of {} is as follows:\n\n{}\n\n",
            basename, processed_content
        ));

        // If DIFF_WITH_BRANCH is set, append a diff report using the diff-with crate.
        if let Ok(diff_branch) = env::var("DIFF_WITH_BRANCH") {
            let diff_output = match run_diff(&file_path) {
                Ok(Some(diff)) => diff,
                Ok(None) => String::new(),
                Err(err) => {
                    eprintln!("Error running diff on {}: {}", file_path, err);
                    String::new()
                }
            };
            if !diff_output.trim().is_empty() {
                final_prompt.push_str(&format!(
                    "\n--------------------------------------------------\nThe diff for {} (against branch {}) is as follows:\n\n{}\n\n",
                    basename, diff_branch, diff_output
                ));
            }
        }

        final_prompt.push_str("\n--------------------------------------------------\n");
    }

    // Append the fixed instruction.
    let fixed_instruction = "Can you do the TODO:- in the above code? But ignoring all FIXMEs and other TODOs...i.e. only do the one and only one TODO that is marked by \"// TODO: - \", i.e. ignore things like \"// TODO: example\" because it doesn't have the hyphen";
    final_prompt.push_str(&format!("\n\n{}", fixed_instruction));

    // Unescape literal \"\\n\" sequences.
    let final_prompt = unescape_newlines(&final_prompt);

    Ok(final_prompt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{NamedTempFile, tempdir};
    use std::fs;
    use std::env;
    use std::io::Write;

    fn fixed_instruction() -> &'static str {
        "Can you do the TODO:- in the above code? But ignoring all FIXMEs and other TODOs...i.e. only do the one and only one TODO that is marked by \"// TODO: - \", i.e. ignore things like \"// TODO: example\" because it doesn't have the hyphen"
    }

    #[test]
    fn test_fixed_instruction_appended() {
        // Create a temporary file listing one file to process.
        let mut found_files = NamedTempFile::new().unwrap();
        let mut file1 = NamedTempFile::new().unwrap();
        writeln!(file1, "class Dummy {{}}").unwrap();
        let file1_path = file1.path().to_str().unwrap();
        writeln!(found_files, "{}", file1_path).unwrap();

        env::remove_var("DIFF_WITH_BRANCH"); // ensure diff is not added

        let output = assemble_prompt(found_files.path().to_str().unwrap(), "ignored")
            .expect("assemble_prompt failed");

        // Verify that the output contains the file header and the fixed instruction.
        assert!(output.contains("The contents of"));
        assert!(output.contains("Can you do the TODO:- in the above code? But ignoring all FIXMEs"));
    }

    #[test]
    fn test_formatting_output_with_fixed_instruction() {
        // Create two temporary source files.
        let mut file1 = NamedTempFile::new().expect("Failed to create file1");
        let file1_content = "class MyClass {\n    // TODO: - Do something important\n}\n";
        file1.write_all(file1_content.as_bytes()).expect("Failed to write file1");
        let file1_path = file1.path().to_owned();

        let mut file2 = NamedTempFile::new().expect("Failed to create file2");
        let file2_content = "struct MyStruct {}\n";
        file2.write_all(file2_content.as_bytes()).expect("Failed to write file2");
        let file2_path = file2.path().to_owned();

        // Create a temporary found_files list that includes file1, file2 and a duplicate of file1.
        let mut found_files = NamedTempFile::new().expect("Failed to create found_files file");
        writeln!(found_files, "{}", file1_path.display()).unwrap();
        writeln!(found_files, "{}", file2_path.display()).unwrap();
        writeln!(found_files, "{}", file1_path.display()).unwrap();
        let found_files_path = found_files.into_temp_path().keep().unwrap();

        // Use an arbitrary instruction content (ignored by the library).
        let instruction_content = "This instruction content is ignored.";

        let output = assemble_prompt(found_files_path.to_str().unwrap(), instruction_content)
            .expect("assemble_prompt failed");

        // Verify that headers for both files are present.
        assert!(output.contains(&format!(
            "The contents of {} is as follows:",
            file1_path.file_name().unwrap().to_string_lossy()
        )));
        assert!(output.contains(&format!(
            "The contents of {} is as follows:",
            file2_path.file_name().unwrap().to_string_lossy()
        )));
        // Verify that file contents and the fixed instruction are included.
        assert!(output.contains("class MyClass {"));
        assert!(output.contains("struct MyStruct {}"));
        assert!(output.contains(fixed_instruction()));
    }

    #[test]
    fn test_process_files_with_substring_markers() {
        // Create a temporary Swift file with substring markers.
        let mut marked_file = NamedTempFile::new().expect("Failed to create MarkedFile.swift");
        let marked_content = "\
    import Foundation
    // v
    func secretFunction() {
        print(\"This is inside the markers.\")
    }
    // ^
    func publicFunction() {
        print(\"This is outside the markers.\")
    }
    ";
        marked_file.write_all(marked_content.as_bytes()).expect("Failed to write marked file");
        let marked_file_path = marked_file.path().to_owned();

        // Create a temporary found_files list listing this file.
        let mut found_files = NamedTempFile::new().expect("Failed to create found_files file");
        writeln!(found_files, "{}", marked_file_path.display()).unwrap();
        let found_files_path = found_files.into_temp_path().keep().unwrap();

        let output = assemble_prompt(found_files_path.to_str().unwrap(), "ignored instruction")
            .expect("assemble_prompt failed");

        // Verify that the header is present and only the marked content is included.
        assert!(output.contains(&format!(
            "The contents of {} is as follows:",
            marked_file_path.file_name().unwrap().to_string_lossy()
        )));
        assert!(output.contains("func secretFunction() {"));
        assert!(output.contains("print(\"This is inside the markers.\")"));
        assert!(!output.contains("func publicFunction() {"));
    }

    #[test]
    fn test_extracts_enclosing_function_context_for_todo_outside_markers() {
        // Create a temporary JS file with markers and a TODO outside them.
        let mut file_js = NamedTempFile::new().expect("Failed to create TestFile.js");
        let js_content = "\
    const someExampleConstant = 42;

    // v

    const anotherExampleConstant = 99;

    // ^

    Parse.Cloud.define(\"getDashboardData\", async (request) => {
        
        // TODO: - helllo
        
        var environment = require(\"./environment.js\");
        var _ = getUnderscore();
        
        var currentUserObjectId = request.params.currentUserObjectId;
        var currentUserGlobal;
        var hiddenPeopleGlobal;
        var timeAgoGlobal = new Date(new Date().getTime() - (24 * 60 * 60 * 1000));
        var resultDictionaryGlobal;
        
    });
    ";
        file_js.write_all(js_content.as_bytes()).expect("Failed to write JS file");
        let file_js_path = file_js.path().to_owned();

        // Set the TODO file basename so the library recognizes this as the TODO file.
        env::set_var("TODO_FILE_BASENAME", file_js_path.file_name().unwrap().to_string_lossy().to_string());

        // Create a temporary found_files list containing the JS file.
        let mut found_files = NamedTempFile::new().expect("Failed to create found_files file");
        writeln!(found_files, "{}", file_js_path.display()).unwrap();
        let found_files_path = found_files.into_temp_path().keep().unwrap();

        let output = assemble_prompt(found_files_path.to_str().unwrap(), "ignored instruction")
            .expect("assemble_prompt failed");

        assert!(output.contains(&format!(
            "The contents of {} is as follows:",
            file_js_path.file_name().unwrap().to_string_lossy()
        )));
        assert!(output.contains("Parse.Cloud.define(\"getDashboardData\", async (request) => {"));
        assert!(output.contains("// TODO: - helllo"));
        assert!(output.contains("// Enclosing function context:"));
        assert!(!output.contains("const someExampleConstant = 42;"));

        env::remove_var("TODO_FILE_BASENAME");
    }

    #[test]
    fn test_missing_file_in_found_files() {
        let mut found_files = NamedTempFile::new().unwrap();
        writeln!(found_files, "/path/to/nonexistent/file.swift").unwrap();
        let found_files_path = found_files.into_temp_path().keep().unwrap();

        let output = assemble_prompt(found_files_path.to_str().unwrap(), "ignored")
            .expect("assemble_prompt failed");

        assert!(!output.contains("nonexistent"));
        assert!(output.contains(fixed_instruction()));
    }

    #[test]
    fn test_empty_found_files_list() {
        let found_files = NamedTempFile::new().unwrap();
        let found_files_path = found_files.into_temp_path().keep().unwrap();

        let output = assemble_prompt(found_files_path.to_str().unwrap(), "ignored")
            .expect("assemble_prompt failed");

        assert!(output.trim().ends_with(fixed_instruction()));
    }

    #[test]
    fn test_diff_with_branch_no_diff_output() {
        let mut file = NamedTempFile::new().unwrap();
        let content = "class NoDiff {}";
        file.write_all(content.as_bytes()).unwrap();
        let file_path = file.path().to_owned();

        let mut found_files = NamedTempFile::new().unwrap();
        writeln!(found_files, "{}", file_path.display()).unwrap();
        let found_files_path = found_files.into_temp_path().keep().unwrap();

        env::set_var("DIFF_WITH_BRANCH", "dummy-branch");

        let dummy_dir = tempdir().unwrap();
        let dummy_path = dummy_dir.path().join("diff_with_branch");
        fs::write(&dummy_path, "#!/bin/sh\necho \"\"\n").expect("Failed to write dummy diff script");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dummy_path).expect("Failed to get metadata").permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dummy_path, perms).expect("Failed to set permissions");
        }
        let original_path = env::var("PATH").unwrap_or_default();
        let new_path = format!("{}:{}", dummy_dir.path().to_str().unwrap(), original_path);
        env::set_var("PATH", new_path);

        let output = assemble_prompt(found_files_path.to_str().unwrap(), "ignored")
            .expect("assemble_prompt failed");

        assert!(!output.contains("against branch dummy-branch"));
        assert!(!output.contains("Dummy diff output"));

        env::remove_var("DIFF_WITH_BRANCH");
        env::set_var("PATH", original_path);
    }

    #[test]
    fn test_missing_closing_marker_in_substring_markers() {
        let mut file = NamedTempFile::new().unwrap();
        let content = "\
    import Foundation
    // v
    func incompleteMarker() {
        print(\"This is inside an unclosed marker.\")
    }
    func outsideFunction() {
        print(\"This should not be inside the marker.\")
    }
    ";
        file.write_all(content.as_bytes()).unwrap();
        let file_path = file.path().to_owned();

        let mut found_files = NamedTempFile::new().unwrap();
        writeln!(found_files, "{}", file_path.display()).unwrap();
        let found_files_path = found_files.into_temp_path().keep().unwrap();

        let output = assemble_prompt(found_files_path.to_str().unwrap(), "ignored")
            .expect("assemble_prompt failed");

        assert!(output.contains("print(\"This is inside an unclosed marker.\")"));
    }
    
    #[test]
    fn test_diff_inclusion() {
        // Create temporary file for found_files and a file to process.
        let mut found_files = NamedTempFile::new().unwrap();
        let mut file_diff = NamedTempFile::new().unwrap();
        writeln!(file_diff, "class DummyDiff {{}}").unwrap();
        let file_diff_path = file_diff.path().to_str().unwrap();
        writeln!(found_files, "{}", file_diff_path).unwrap();

        // Activate diff logic.
        env::set_var("DIFF_WITH_BRANCH", "dummy-branch");

        // Create a dummy "git" executable that simulates a successful ls-files and returns a diff.
        let temp_dir = tempdir().unwrap();
        let dummy_git = temp_dir.path().join("git");
        fs::write(
            &dummy_git,
            "#!/bin/sh
    case \"$@\" in
        *ls-files*)
            exit 0
            ;;
        *diff*)
            echo -n \"Dummy diff output for file\"
            exit 0
            ;;
        *)
            exit 1
            ;;
    esac
    ",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dummy_git).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dummy_git, perms).unwrap();
        }
        // Prepend the temporary directory (containing dummy git) to PATH.
        let current_path = env::var("PATH").unwrap();
        let new_path = format!("{}:{}", temp_dir.path().to_str().unwrap(), current_path);
        env::set_var("PATH", new_path);

        let output = assemble_prompt(found_files.path().to_str().unwrap(), "ignored")
            .expect("assemble_prompt failed");

        assert!(output.contains("Dummy diff output for file"), "Output did not include diff output: {}", output);
        assert!(output.contains("against branch dummy-branch"));

        env::remove_var("DIFF_WITH_BRANCH");
    }

    #[test]
    fn test_includes_diff_output_when_diff_with_branch_set() {
        // Create a temporary Swift file.
        let mut file_diff = NamedTempFile::new().expect("Failed to create FileDiff.swift");
        let diff_content = "class NoDiff {}";
        file_diff.write_all(diff_content.as_bytes()).unwrap();
        let file_diff_path = file_diff.path().to_owned();

        // Create a temporary found_files list.
        let mut found_files = NamedTempFile::new().expect("Failed to create found_files file");
        writeln!(found_files, "{}", file_diff_path.display()).unwrap();
        let found_files_path = found_files.into_temp_path().keep().unwrap();

        // Set DIFF_WITH_BRANCH to activate diff logic.
        env::set_var("DIFF_WITH_BRANCH", "dummy-branch");

        // Create a dummy "git" executable that simulates diff output.
        let dummy_dir = tempdir().unwrap();
        let dummy_git = dummy_dir.path().join("git");
        fs::write(
            &dummy_git,
            "#!/bin/sh
    case \"$@\" in
        *ls-files*)
            exit 0
            ;;
        *diff*)
            echo -n \"Dummy diff output for file\"
            exit 0
            ;;
        *)
            exit 1
            ;;
    esac
    ",
        )
        .expect("Failed to write dummy git script");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dummy_git).expect("Failed to get metadata").permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dummy_git, perms).expect("Failed to set permissions");
        }
        // Prepend dummy_dir to PATH.
        let original_path = env::var("PATH").unwrap_or_default();
        let new_path = format!("{}:{}", dummy_dir.path().to_str().unwrap(), original_path);
        env::set_var("PATH", new_path);

        let output = assemble_prompt(found_files_path.to_str().unwrap(), "ignored")
            .expect("assemble_prompt failed");

        let expected_diff = "Dummy diff output for file";
        assert!(output.contains(expected_diff), "Expected diff output missing: {}", output);
        assert!(output.contains("against branch dummy-branch"));

        env::remove_var("DIFF_WITH_BRANCH");
        env::set_var("PATH", original_path);
    }

    #[test]
    fn test_assemble_prompt_marker_count_with_diff() {
        // Create a temporary found_files list with one file that contains two TODO markers.
        let mut found_files = NamedTempFile::new().unwrap();
        let mut file = NamedTempFile::new().unwrap();
        let file_content = "\
                // TODO: - Marker One\n\
                Some code here\n\
                // TODO: - Marker Two\n";
        writeln!(file, "{}", file_content).unwrap();
        let file_path = file.path().to_str().unwrap();
        writeln!(found_files, "{}", file_path).unwrap();

        // Activate diff logic.
        env::set_var("DIFF_WITH_BRANCH", "dummy-branch");

        // Create a dummy "git" executable that returns a diff.
        let dummy_dir = tempdir().expect("Failed to create dummy dir");
        let dummy_git = dummy_dir.path().join("git");
        fs::write(
            &dummy_git,
            "#!/bin/sh
    case \"$@\" in
        *ls-files*)
            exit 0
            ;;
        *diff*)
            echo -n \"Diff output\"
            exit 0
            ;;
        *)
            exit 1
            ;;
    esac
    ",
        )
        .expect("Failed to write dummy git script");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dummy_git).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dummy_git, perms).unwrap();
        }
        // Prepend dummy_dir to PATH.
        let original_path = env::var("PATH").unwrap();
        let new_path = format!("{}:{}", dummy_dir.path().to_str().unwrap(), original_path);
        env::set_var("PATH", new_path);

        let output = assemble_prompt(found_files.path().to_str().unwrap(), "ignored")
            .expect("assemble_prompt failed");

        // Check that the diff output is included.
        assert!(output.contains("Diff output"), "Output did not include diff output: {}", output);
        assert!(output.contains("against branch dummy-branch"), "Missing branch info in output");

        // Count the lines that contain the TODO marker.
        let marker_count = output.lines().filter(|l| l.contains("// TODO: -")).count();
        // Acceptable marker counts are 2 (normal case) or 3 (if diff section adds an extra marker).
        assert!(marker_count == 2 || marker_count == 3, "Unexpected marker count: {}", marker_count);

        env::remove_var("DIFF_WITH_BRANCH");
        env::set_var("PATH", original_path);
    }
}

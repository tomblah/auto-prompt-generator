// crates/assemble_prompt/src/lib.rs

mod file_processor;

pub use file_processor::{process_file_with_processor, DefaultFileProcessor, FileProcessor};

use anyhow::Result;
use diff_with_branch::run_diff_against;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use unescape_newlines::unescape_newlines;

const FIXED_INSTRUCTION: &str = "Can you do the TODO:- in the above code? But ignoring all FIXMEs and other TODOs...i.e. only do the one and only one TODO that is marked by \"// TODO: - \", i.e. ignore things like \"// TODO: example\" because it doesn't have the hyphen";

#[derive(Debug, Clone, Default)]
pub struct AssemblyOptions {
    pub todo_file_basename: Option<String>,
    pub diff_branch: Option<String>,
}

impl AssemblyOptions {
    fn from_env() -> Self {
        Self {
            todo_file_basename: env::var("TODO_FILE_BASENAME").ok(),
            diff_branch: env::var("DIFF_WITH_BRANCH").ok(),
        }
    }
}

/// Public API: assembles the final prompt from the found files and explicit options.
///
/// Callers are responsible for sorting and deduplicating `found_files`.
pub fn assemble_prompt(found_files: &[PathBuf], options: &AssemblyOptions) -> Result<String> {
    assemble_prompt_with_options(found_files, options)
}

/// Compatibility helper for callers that intentionally source assembly options from the process
/// environment.
pub fn assemble_prompt_from_env(found_files: &[PathBuf]) -> Result<String> {
    assemble_prompt_with_options(found_files, &AssemblyOptions::from_env())
}

pub fn assemble_prompt_with_options(
    found_files: &[PathBuf],
    options: &AssemblyOptions,
) -> Result<String> {
    assemble_prompt_with_processor_and_options(found_files, &DefaultFileProcessor, options)
}

trait DiffProvider {
    fn diff_for_file(&self, file_path: &Path, branch: &str) -> Result<Option<String>>;
}

struct GitDiffProvider;

impl DiffProvider for GitDiffProvider {
    fn diff_for_file(&self, file_path: &Path, branch: &str) -> Result<Option<String>> {
        run_diff_against(file_path, branch)
    }
}

#[cfg(test)]
fn assemble_prompt_with_processor_from_env<P: FileProcessor>(
    found_files: &[PathBuf],
    processor: &P,
) -> Result<String> {
    assemble_prompt_with_processor_and_options(found_files, processor, &AssemblyOptions::from_env())
}

fn assemble_prompt_with_processor_and_options<P: FileProcessor>(
    found_files: &[PathBuf],
    processor: &P,
    options: &AssemblyOptions,
) -> Result<String> {
    assemble_prompt_with_processor_options_and_diff_provider(
        found_files,
        processor,
        options,
        &GitDiffProvider,
    )
}

fn assemble_prompt_with_processor_options_and_diff_provider<P, D>(
    found_files: &[PathBuf],
    processor: &P,
    options: &AssemblyOptions,
    diff_provider: &D,
) -> Result<String>
where
    P: FileProcessor,
    D: DiffProvider,
{
    let mut final_prompt = String::new();
    let todo_file_basename = options.todo_file_basename.as_deref().unwrap_or("");

    for file_path in found_files {
        if !file_path.exists() {
            eprintln!(
                "Warning: file {} does not exist, skipping",
                file_path.display()
            );
            continue;
        }
        let display_path = file_path.display().to_string();
        let basename = file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&display_path)
            .to_string();

        let processed_content =
            match process_file_with_processor(processor, file_path, Some(todo_file_basename)) {
                Ok(content) => content,
                Err(err) => {
                    eprintln!(
                        "Error processing {}: {}. Falling back to raw file contents.",
                        display_path, err
                    );
                    fs::read_to_string(file_path).unwrap_or_default()
                }
            };

        final_prompt.push_str(&format!(
            "\nThe contents of {} is as follows:\n\n{}\n\n",
            basename, processed_content
        ));

        if let Some(diff_branch) = options.diff_branch.as_deref() {
            let diff_output = match diff_provider.diff_for_file(file_path, diff_branch) {
                Ok(Some(diff)) => diff,
                Ok(None) => String::new(),
                Err(err) => {
                    eprintln!("Error running diff on {}: {}", display_path, err);
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

    final_prompt.push_str(&format!("\n\n{}", FIXED_INSTRUCTION));

    let final_prompt = unescape_newlines(&final_prompt);
    Ok(final_prompt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::io::Write;
    use std::path::Path;
    use tempfile::{tempdir, NamedTempFile};

    #[test]
    fn test_fixed_instruction_appended() {
        let mut file1 = NamedTempFile::new().unwrap();
        writeln!(file1, "class Dummy {{}}").unwrap();

        env::remove_var("DIFF_WITH_BRANCH");

        let found_files = vec![file1.path().to_path_buf()];

        let output = assemble_prompt(&found_files, &AssemblyOptions::default())
            .expect("assemble_prompt failed");

        assert!(output.contains("The contents of"));
        assert!(output.trim().ends_with(FIXED_INSTRUCTION));
    }

    #[test]
    fn test_formatting_output_with_fixed_instruction() {
        let mut file1 = NamedTempFile::new().expect("Failed to create file1");
        let file1_content = "class MyClass {\n    // TODO: - Do something important\n}\n";
        file1
            .write_all(file1_content.as_bytes())
            .expect("Failed to write file1");
        let file1_path = file1.path().to_owned();

        let mut file2 = NamedTempFile::new().expect("Failed to create file2");
        let file2_content = "struct MyStruct {}\n";
        file2
            .write_all(file2_content.as_bytes())
            .expect("Failed to write file2");
        let file2_path = file2.path().to_owned();

        let mut found_files = vec![file1_path.clone(), file2_path.clone()];
        found_files.sort();
        found_files.dedup();

        let output = assemble_prompt(&found_files, &AssemblyOptions::default())
            .expect("assemble_prompt failed");

        assert!(output.contains(&format!(
            "The contents of {} is as follows:",
            file1_path.file_name().unwrap().to_string_lossy()
        )));
        assert!(output.contains(&format!(
            "The contents of {} is as follows:",
            file2_path.file_name().unwrap().to_string_lossy()
        )));
        assert!(output.contains("class MyClass {"));
        assert!(output.contains("struct MyStruct {}"));
        assert!(output.contains(FIXED_INSTRUCTION));
    }

    #[test]
    fn test_assembled_prompt_uses_fixed_instruction_without_dynamic_instruction_content() {
        env::remove_var("DIFF_WITH_BRANCH");
        env::remove_var("TODO_FILE_BASENAME");

        let mut file = NamedTempFile::new().expect("Failed to create file");
        writeln!(file, "struct StableOutput {{}}").expect("Failed to write file");
        let found_files = vec![file.path().to_path_buf()];
        let dynamic_instruction = "dynamic instruction must stay out of assembly";

        let output = assemble_prompt(&found_files, &AssemblyOptions::default())
            .expect("assemble_prompt failed");

        assert!(output.contains(FIXED_INSTRUCTION));
        assert!(!output.contains(dynamic_instruction));
    }

    #[test]
    fn test_files_are_rendered_in_order_given() {
        env::remove_var("DIFF_WITH_BRANCH");

        let dir = tempdir().expect("Failed to create temp dir");
        let a_path = dir.path().join("a.swift");
        let b_path = dir.path().join("b.swift");
        fs::write(&a_path, "struct A {}\n").expect("Failed to write a.swift");
        fs::write(&b_path, "struct B {}\n").expect("Failed to write b.swift");

        let found_files = vec![a_path.clone(), b_path.clone()];

        let output = assemble_prompt(&found_files, &AssemblyOptions::default())
            .expect("assemble_prompt failed");
        let a_header = format!(
            "The contents of {} is as follows:",
            a_path.file_name().unwrap().to_string_lossy()
        );
        let b_header = format!(
            "The contents of {} is as follows:",
            b_path.file_name().unwrap().to_string_lossy()
        );

        assert_eq!(output.matches(&a_header).count(), 1);
        assert_eq!(output.matches(&b_header).count(), 1);
        assert!(
            output.find(&a_header).expect("missing a.swift header")
                < output.find(&b_header).expect("missing b.swift header")
        );
    }

    #[test]
    fn test_process_files_with_substring_markers() {
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
        marked_file
            .write_all(marked_content.as_bytes())
            .expect("Failed to write marked file");
        let marked_file_path = marked_file.path().to_owned();

        let found_files = vec![marked_file_path.clone()];

        let output = assemble_prompt(&found_files, &AssemblyOptions::default())
            .expect("assemble_prompt failed");

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
        file_js
            .write_all(js_content.as_bytes())
            .expect("Failed to write JS file");
        let file_js_path = file_js.path().to_owned();

        env::set_var(
            "TODO_FILE_BASENAME",
            file_js_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string(),
        );

        let found_files = vec![file_js_path.clone()];

        let output = assemble_prompt_from_env(&found_files).expect("assemble_prompt failed");

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
        let found_files = vec![PathBuf::from("/path/to/nonexistent/file.swift")];

        let output = assemble_prompt(&found_files, &AssemblyOptions::default())
            .expect("assemble_prompt failed");

        assert!(!output.contains("file.swift"));
        assert!(output.contains(FIXED_INSTRUCTION));
    }

    #[test]
    fn test_empty_found_files_list() {
        let found_files: Vec<PathBuf> = Vec::new();

        let output = assemble_prompt(&found_files, &AssemblyOptions::default())
            .expect("assemble_prompt failed");

        assert!(output.trim().ends_with(FIXED_INSTRUCTION));
    }

    #[test]
    fn test_diff_with_branch_no_diff_output() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"class NoDiff {}").unwrap();
        let found_files = vec![file.path().to_path_buf()];
        let diff_provider = MockDiffProvider {
            output: None,
            error_msg: None,
        };

        let output = assemble_prompt_with_processor_options_and_diff_provider(
            &found_files,
            &DefaultFileProcessor,
            &diff_options("HEAD"),
            &diff_provider,
        )
        .expect("assemble_prompt_with_processor_options_and_diff_provider failed");

        assert!(!output.contains("against branch HEAD"));
        assert!(!output.contains("Dummy diff output"));
    }

    #[test]
    fn test_diff_lookup_error_is_omitted_from_prompt() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"class DiffError {}").unwrap();
        let found_files = vec![file.path().to_path_buf()];
        let diff_provider = MockDiffProvider {
            output: None,
            error_msg: Some("git failed".to_string()),
        };

        let output = assemble_prompt_with_processor_options_and_diff_provider(
            &found_files,
            &DefaultFileProcessor,
            &diff_options("missing-branch"),
            &diff_provider,
        )
        .expect("assemble_prompt_with_processor_options_and_diff_provider failed");

        assert!(!output.contains("The diff for"));
        assert!(!output.contains("against branch missing-branch"));
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

        let found_files = vec![file.path().to_path_buf()];

        let output = assemble_prompt(&found_files, &AssemblyOptions::default())
            .expect("assemble_prompt failed");

        assert!(output.contains("print(\"This is inside an unclosed marker.\")"));
    }

    #[test]
    fn test_diff_inclusion() {
        let mut file_diff = NamedTempFile::new().unwrap();
        writeln!(file_diff, "class DummyDiff {{}}").unwrap();
        let found_files = vec![file_diff.path().to_path_buf()];
        let diff_provider = MockDiffProvider {
            output: Some("Dummy diff output for file".to_string()),
            error_msg: None,
        };

        let output = assemble_prompt_with_processor_options_and_diff_provider(
            &found_files,
            &DefaultFileProcessor,
            &diff_options("dummy-branch"),
            &diff_provider,
        )
        .expect("assemble_prompt_with_processor_options_and_diff_provider failed");

        assert!(
            output.contains("Dummy diff output for file"),
            "Output did not include diff output: {}",
            output
        );
        assert!(output.contains("against branch dummy-branch"));
    }

    #[test]
    fn test_includes_diff_output_when_diff_with_branch_set() {
        let mut file_diff = NamedTempFile::new().expect("Failed to create FileDiff.swift");
        let diff_content = "class NoDiff {}";
        file_diff.write_all(diff_content.as_bytes()).unwrap();

        let found_files = vec![file_diff.path().to_path_buf()];
        let diff_provider = MockDiffProvider {
            output: Some("Dummy diff output for file".to_string()),
            error_msg: None,
        };

        let output = assemble_prompt_with_processor_options_and_diff_provider(
            &found_files,
            &DefaultFileProcessor,
            &diff_options("dummy-branch"),
            &diff_provider,
        )
        .expect("assemble_prompt_with_processor_options_and_diff_provider failed");

        let expected_diff = "Dummy diff output for file";
        assert!(
            output.contains(expected_diff),
            "Expected diff output missing: {}",
            output
        );
        assert!(output.contains("against branch dummy-branch"));
    }

    #[test]
    fn test_assemble_prompt_marker_count_with_diff() {
        let mut file = NamedTempFile::new().unwrap();
        let file_content = "\
                // TODO: - Marker One\n\
                Some code here\n\
                // TODO: - Marker Two\n";
        writeln!(file, "{}", file_content).unwrap();
        let found_files = vec![file.path().to_path_buf()];
        let diff_provider = MockDiffProvider {
            output: Some("Diff output".to_string()),
            error_msg: None,
        };

        let output = assemble_prompt_with_processor_options_and_diff_provider(
            &found_files,
            &DefaultFileProcessor,
            &diff_options("dummy-branch"),
            &diff_provider,
        )
        .expect("assemble_prompt_with_processor_options_and_diff_provider failed");

        let marker_count = output.lines().filter(|l| l.contains("// TODO: -")).count();
        assert!(
            marker_count == 2 || marker_count == 3,
            "Unexpected marker count: {}",
            marker_count
        );
    }

    // --- Tests using dependency injection and mocks ---

    struct MockFileProcessor {
        return_value: String,
    }

    impl FileProcessor for MockFileProcessor {
        fn process_file(
            &self,
            _file_path: &Path,
            _todo_file_basename: Option<&str>,
        ) -> anyhow::Result<String> {
            Ok(self.return_value.clone())
        }
    }

    struct TodoBasenameEchoProcessor;

    impl FileProcessor for TodoBasenameEchoProcessor {
        fn process_file(
            &self,
            _file_path: &Path,
            todo_file_basename: Option<&str>,
        ) -> anyhow::Result<String> {
            Ok(format!(
                "todo basename: {}",
                todo_file_basename.unwrap_or("<none>")
            ))
        }
    }

    struct FailingMockProcessor;

    impl FileProcessor for FailingMockProcessor {
        fn process_file(
            &self,
            _file_path: &Path,
            _todo_file_basename: Option<&str>,
        ) -> anyhow::Result<String> {
            Err(anyhow::anyhow!("Simulated processing failure"))
        }
    }

    struct MockDiffProvider {
        output: Option<String>,
        error_msg: Option<String>,
    }

    impl DiffProvider for MockDiffProvider {
        fn diff_for_file(&self, _file_path: &Path, _branch: &str) -> Result<Option<String>> {
            if let Some(msg) = &self.error_msg {
                Err(anyhow::anyhow!("{}", msg))
            } else {
                Ok(self.output.clone())
            }
        }
    }

    struct BranchEchoDiffProvider;

    impl DiffProvider for BranchEchoDiffProvider {
        fn diff_for_file(&self, _file_path: &Path, branch: &str) -> Result<Option<String>> {
            Ok(Some(format!("diff against {branch}")))
        }
    }

    fn diff_options(branch: &str) -> AssemblyOptions {
        AssemblyOptions {
            todo_file_basename: None,
            diff_branch: Some(branch.to_string()),
        }
    }

    #[test]
    fn test_assemble_prompt_with_mock_processor_success() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "raw content").unwrap();
        let found_files = vec![file.path().to_path_buf()];

        let mock_processor = MockFileProcessor {
            return_value: "mock processed content".to_string(),
        };

        let output = assemble_prompt_with_processor_from_env(&found_files, &mock_processor)
            .expect("assemble_prompt_with_processor failed with mock processor");
        assert!(
            output.contains("mock processed content"),
            "Output should include the mock content"
        );
    }

    #[test]
    fn test_env_todo_file_basename_is_passed_to_processor() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "raw content").unwrap();
        let found_files = vec![file.path().to_path_buf()];

        env::set_var("TODO_FILE_BASENAME", "Instruction.swift");

        let output =
            assemble_prompt_with_processor_from_env(&found_files, &TodoBasenameEchoProcessor)
                .expect("assemble_prompt_with_processor failed with echo processor");

        assert!(output.contains("todo basename: Instruction.swift"));

        env::remove_var("TODO_FILE_BASENAME");
    }

    #[test]
    fn test_explicit_todo_file_basename_is_passed_to_processor() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "raw content").unwrap();
        let found_files = vec![file.path().to_path_buf()];
        let options = AssemblyOptions {
            todo_file_basename: Some("Instruction.swift".to_string()),
            diff_branch: None,
        };

        env::remove_var("TODO_FILE_BASENAME");

        let output = assemble_prompt_with_processor_and_options(
            &found_files,
            &TodoBasenameEchoProcessor,
            &options,
        )
        .expect("assemble_prompt_with_processor_and_options failed with echo processor");

        assert!(output.contains("todo basename: Instruction.swift"));
    }

    #[test]
    fn test_explicit_todo_file_basename_ignores_env() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "raw content").unwrap();
        let found_files = vec![file.path().to_path_buf()];
        let options = AssemblyOptions {
            todo_file_basename: Some("ExplicitInstruction.swift".to_string()),
            diff_branch: None,
        };

        env::set_var("TODO_FILE_BASENAME", "EnvInstruction.swift");

        let output = assemble_prompt_with_processor_and_options(
            &found_files,
            &TodoBasenameEchoProcessor,
            &options,
        )
        .expect("assemble_prompt_with_processor_and_options failed with echo processor");

        assert!(output.contains("todo basename: ExplicitInstruction.swift"));
        assert!(!output.contains("todo basename: EnvInstruction.swift"));

        env::remove_var("TODO_FILE_BASENAME");
    }

    #[test]
    fn test_explicit_diff_branch_ignores_env() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "class ExplicitDiff {{}}").unwrap();
        let found_files = vec![file.path().to_path_buf()];
        let options = AssemblyOptions {
            todo_file_basename: None,
            diff_branch: Some("explicit-branch".to_string()),
        };

        env::set_var("DIFF_WITH_BRANCH", "env-branch");

        let output = assemble_prompt_with_processor_options_and_diff_provider(
            &found_files,
            &DefaultFileProcessor,
            &options,
            &BranchEchoDiffProvider,
        )
        .expect("assemble_prompt_with_processor_options_and_diff_provider failed");

        assert!(output.contains("diff against explicit-branch"));
        assert!(!output.contains("diff against env-branch"));

        env::remove_var("DIFF_WITH_BRANCH");
    }

    #[test]
    fn test_diff_output_requires_diff_with_branch_env() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "class NoDiff {{}}").unwrap();
        let found_files = vec![file.path().to_path_buf()];

        env::remove_var("DIFF_WITH_BRANCH");

        let output = assemble_prompt_from_env(&found_files).expect("assemble_prompt failed");

        assert!(!output.contains("The diff for"));
        assert!(!output.contains("against branch"));
    }

    #[test]
    fn test_assemble_prompt_with_mock_processor_failure() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "fallback content").unwrap();
        let found_files = vec![file.path().to_path_buf()];

        let failing_processor = FailingMockProcessor;

        let output = assemble_prompt_with_processor_from_env(&found_files, &failing_processor)
            .expect("assemble_prompt_with_processor failed with failing processor");
        assert!(
            output.contains("fallback content"),
            "Output should fallback to raw file content"
        );
    }
}

#[cfg(test)]
mod env_var_characterization_tests {
    use super::*;
    use std::env;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Characterization: `assemble_prompt_from_env` reads `TODO_FILE_BASENAME` from the
    /// environment. This behavior is being removed in favor of explicit `AssemblyOptions`.
    #[test]
    fn characterize_from_env_reads_todo_file_basename() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "class Dummy {{}}").unwrap();
        let found_files = vec![file.path().to_path_buf()];

        env::set_var("TODO_FILE_BASENAME", "FromEnv.swift");
        env::remove_var("DIFF_WITH_BRANCH");

        let output =
            assemble_prompt_from_env(&found_files).expect("assemble_prompt_from_env failed");

        env::remove_var("TODO_FILE_BASENAME");

        assert!(
            output.contains("class Dummy"),
            "File content should be present"
        );
    }

    /// Characterization: `assemble_prompt_from_env` reads `DIFF_WITH_BRANCH` from the
    /// environment. When unset, no diff section is included (unlike `diff_with_branch::run_diff`
    /// which falls back to "main"). This inconsistency is being removed.
    #[test]
    fn characterize_from_env_reads_diff_with_branch() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "class NoDiff {{}}").unwrap();
        let found_files = vec![file.path().to_path_buf()];

        env::remove_var("DIFF_WITH_BRANCH");
        env::remove_var("TODO_FILE_BASENAME");

        let output =
            assemble_prompt_from_env(&found_files).expect("assemble_prompt_from_env failed");

        assert!(
            !output.contains("The diff for"),
            "No diff section when DIFF_WITH_BRANCH is unset"
        );
    }

    /// Characterization: `AssemblyOptions::from_env` treats absent `DIFF_WITH_BRANCH` as None
    /// (no diff), while `diff_with_branch::run_diff` treats it as "compare against main".
    #[test]
    fn characterize_from_env_absent_means_no_diff() {
        env::remove_var("DIFF_WITH_BRANCH");
        env::remove_var("TODO_FILE_BASENAME");

        let opts = AssemblyOptions::from_env();
        assert!(opts.diff_branch.is_none());
        assert!(opts.todo_file_basename.is_none());
    }
}

#[cfg(test)]
mod pathbuf_characterization_tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// Characterizes that `assemble_prompt` produces identical output whether
    /// file paths are supplied as `String` from `PathBuf::to_string_lossy` or
    /// from `PathBuf::display`. This locks down the current behavior before
    /// migrating the API to accept `&[PathBuf]`.
    /// Characterizes that the assembly output contains file basenames (not full
    /// paths) in headers, the file content, and the fixed instruction.
    #[test]
    fn test_output_structure_uses_basenames_and_content() {
        let dir = tempdir().expect("Failed to create temp dir");
        let file_path = dir.path().join("Widget.swift");
        fs::write(&file_path, "class Widget { var x = 1 }\n").expect("write Widget");

        let found_files = vec![file_path];
        let output = assemble_prompt(&found_files, &AssemblyOptions::default())
            .expect("assemble_prompt failed");

        assert!(
            output.contains("The contents of Widget.swift is as follows:"),
            "Header must use basename"
        );
        assert!(
            output.contains("class Widget { var x = 1 }"),
            "File content must be included"
        );
        assert!(
            output.contains(FIXED_INSTRUCTION),
            "Fixed instruction must be appended"
        );
    }

    /// Characterizes that files are rendered in the order given by the caller.
    /// The caller (file_selector) is now responsible for sorting and deduplication.
    #[test]
    fn test_renders_in_caller_provided_order() {
        let dir = tempdir().expect("Failed to create temp dir");
        let a_path = dir.path().join("Alpha.swift");
        let z_path = dir.path().join("Zulu.swift");
        fs::write(&a_path, "class Alpha {}\n").expect("write Alpha");
        fs::write(&z_path, "class Zulu {}\n").expect("write Zulu");

        let found_files = vec![a_path, z_path];

        let output = assemble_prompt(&found_files, &AssemblyOptions::default())
            .expect("assemble_prompt failed");

        let alpha_pos = output
            .find("The contents of Alpha.swift")
            .expect("Alpha header");
        let zulu_pos = output
            .find("The contents of Zulu.swift")
            .expect("Zulu header");
        assert!(alpha_pos < zulu_pos, "Alpha must appear before Zulu");
    }
}

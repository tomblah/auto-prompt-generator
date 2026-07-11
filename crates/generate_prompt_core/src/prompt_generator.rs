// crates/generate_prompt_core/src/prompt_generator.rs

use anyhow::{anyhow, Context, Result};
use log::{debug, info};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::file_selector;
use crate::search_root;
use extract_instruction_content::extract_instruction_content;

#[derive(Debug, Clone, Default)]
pub struct GeneratePromptOptions {
    pub singular: bool,
    pub force_global: bool,
    pub include_references: bool,
    pub excludes: Vec<String>,
    pub diff_branch: Option<String>,
    pub targeted: bool,
}

#[derive(Debug)]
pub struct GeneratePromptOutput {
    pub final_prompt: String,
    pub instruction_content: String,
    pub search_root: PathBuf,
    pub found_files: Vec<PathBuf>,
    pub types_found: BTreeSet<String>,
}

pub fn generate_prompt_with_options(
    git_root: &str,
    file_path: &Path,
    options: &GeneratePromptOptions,
) -> Result<GeneratePromptOutput> {
    let todo_file_basename = file_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    if options.include_references {
        let extension = file_path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let supports_references = lang_support::for_extension(extension)
            .is_some_and(|lang| lang.supports_enclosing_type());
        if !supports_references {
            return Err(anyhow!(
                "--include-references is only supported for Swift files"
            ));
        }
    }

    let base_dir = PathBuf::from(git_root);

    let search_root_path = if options.force_global {
        info!("Force global enabled: using Git root for context");
        base_dir.clone()
    } else {
        search_root::determine_search_root(&base_dir, file_path)
    };
    debug!("Search root: {}", search_root_path.display());

    let instruction_content =
        extract_instruction_content(file_path).context("Failed to extract instruction content")?;
    debug!("Instruction content: {}", instruction_content.trim());
    debug!("--------------------------------------------------");

    let selection = file_selector::determine_files_to_include_with_options(
        file_path,
        options.singular,
        &search_root_path,
        &options.excludes,
        &file_selector::FileSelectionOptions {
            include_references: options.include_references,
            targeted: options.targeted,
        },
    )?;

    let assembly_options = assemble_prompt::AssemblyOptions {
        todo_file_basename: Some(todo_file_basename),
        diff_branch: options.diff_branch.clone(),
    };
    let assembled_prompt =
        assemble_prompt::assemble_prompt_with_options(&selection.files, &assembly_options)
            .context("Failed to assemble prompt")?;

    let diff_enabled = options.diff_branch.is_some();

    let final_prompt = post_processing::scrub_extra_todo_markers(
        &assembled_prompt,
        diff_enabled,
        instruction_content.trim(),
    )?;

    crate::prompt_validation::validate_marker_count(&final_prompt, diff_enabled)?;

    Ok(GeneratePromptOutput {
        final_prompt,
        instruction_content: instruction_content.trim().to_string(),
        search_root: search_root_path,
        found_files: selection.files,
        types_found: selection.types_found,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::File;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    fn write_temp_file(dir: &Path, filename: &str, contents: &str) -> PathBuf {
        let file_path = dir.join(filename);
        let mut file = File::create(&file_path).expect("Failed to create temp file");
        file.write_all(contents.as_bytes())
            .expect("Failed to write to temp file");
        file_path
    }

    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_with_options_targeted_ignores_env() {
        env::remove_var("TARGETED");

        let temp_dir = tempdir().expect("Failed to create temp dir");
        let git_root = temp_dir.path().to_str().unwrap();

        let instruction_file = write_temp_file(
            temp_dir.path(),
            "Instruction.swift",
            r#"
class OuterType {}
func testFunction() {
    class InnerType {}
    // TODO: - Perform action
}
"#,
        );
        let _outer_def_path = write_temp_file(
            temp_dir.path(),
            "OuterDefinition.swift",
            "class OuterType {}\n",
        );
        let inner_def_path = write_temp_file(
            temp_dir.path(),
            "InnerDefinition.swift",
            "class InnerType {}\n",
        );

        let result = generate_prompt_with_options(
            git_root,
            &instruction_file,
            &GeneratePromptOptions {
                singular: false,
                force_global: false,
                include_references: false,
                excludes: vec![],
                diff_branch: None,
                targeted: true,
            },
        );

        assert!(
            result.is_ok(),
            "Expected explicit targeted prompt generation"
        );
        let output = result.unwrap();
        assert!(output
            .final_prompt
            .contains(inner_def_path.file_name().and_then(|s| s.to_str()).unwrap()));
    }

    #[test]
    fn test_generate_prompt_singular_success() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let git_root = temp_dir.path().to_str().unwrap();

        let file_content = r#"
// Some initial code
// TODO: - Test instruction
// Some more code
"#;
        let instruction_file = write_temp_file(temp_dir.path(), "instruction.txt", file_content);

        let result = generate_prompt_with_options(
            git_root,
            &instruction_file,
            &GeneratePromptOptions {
                singular: true,
                force_global: false,
                include_references: false,
                excludes: vec![],
                diff_branch: None,
                targeted: false,
            },
        );
        assert!(
            result.is_ok(),
            "Expected generate_prompt to succeed in singular mode"
        );
    }

    #[test]
    fn test_generate_prompt_include_references_success() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let git_root = temp_dir.path().to_str().unwrap();

        let file_content = r#"
class Dummy {}
// TODO: - Test instruction
"#;
        let instruction_file = write_temp_file(temp_dir.path(), "instruction.swift", file_content);

        let result = generate_prompt_with_options(
            git_root,
            &instruction_file,
            &GeneratePromptOptions {
                singular: false,
                force_global: false,
                include_references: true,
                excludes: vec![],
                diff_branch: None,
                targeted: false,
            },
        );
        assert!(
            result.is_ok(),
            "Expected generate_prompt to succeed for Swift file with references"
        );
    }

    #[test]
    fn test_generate_prompt_include_references_non_swift_returns_error() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let git_root = temp_dir.path().to_str().unwrap();
        let file_content = r#"
// Some JS code
// TODO: - JS Test instruction
"#;
        let instruction_file = write_temp_file(temp_dir.path(), "instruction.js", file_content);
        let result = generate_prompt_with_options(
            git_root,
            &instruction_file,
            &GeneratePromptOptions {
                singular: false,
                force_global: false,
                include_references: true,
                excludes: vec![],
                diff_branch: None,
                targeted: false,
            },
        );

        let err = result.expect_err("Expected non-Swift include_references to return an error");
        assert!(
            err.to_string()
                .contains("--include-references is only supported for Swift files"),
            "Unexpected error: {err}"
        );
    }

    #[test]
    fn test_generate_prompt_force_global() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let git_root = temp_dir.path().to_str().unwrap();

        let file_content = r#"
// Some code
// TODO: - Force global test
"#;
        let instruction_file = write_temp_file(temp_dir.path(), "instruction.txt", file_content);

        let result = generate_prompt_with_options(
            git_root,
            &instruction_file,
            &GeneratePromptOptions {
                singular: true,
                force_global: true,
                include_references: false,
                excludes: vec![],
                diff_branch: None,
                targeted: false,
            },
        );
        assert!(
            result.is_ok(),
            "Expected generate_prompt to succeed in force global mode"
        );
    }

    #[test]
    fn test_generate_prompt_js_warning() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let git_root = temp_dir.path().to_str().unwrap();

        let file_content = r#"
// Some JS code
// TODO: - JS Test instruction
"#;
        let instruction_file = write_temp_file(temp_dir.path(), "instruction.js", file_content);

        let result = generate_prompt_with_options(
            git_root,
            &instruction_file,
            &GeneratePromptOptions {
                singular: false,
                force_global: false,
                include_references: false,
                excludes: vec![],
                diff_branch: None,
                targeted: false,
            },
        );
        assert!(
            result.is_ok(),
            "Expected generate_prompt to succeed for a JS file (with warning)"
        );
    }
}

#[cfg(test)]
mod enclosing_block_pipeline_characterization_tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    fn write_temp_file(dir: &Path, filename: &str, contents: &str) -> PathBuf {
        let file_path = dir.join(filename);
        let mut file = File::create(&file_path).expect("Failed to create temp file");
        file.write_all(contents.as_bytes())
            .expect("Failed to write to temp file");
        file_path
    }

    /// Characterizes that when a Swift file has markers and the TODO is outside
    /// the markers, the assembled prompt contains "Enclosing function context"
    /// with the function body.
    #[test]
    fn char_markers_with_todo_outside_produces_enclosing_function_context() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let git_root = temp_dir.path().to_str().unwrap();

        let file_content = concat!(
            "import Foundation\n",
            "// v\n",
            "let headerConstant = 42\n",
            "// ^\n",
            "func processData() {\n",
            "    let result = compute()\n",
            "    // TODO: - Handle edge case\n",
            "    return result\n",
            "}\n",
        );
        let instruction_file = write_temp_file(temp_dir.path(), "Processor.swift", file_content);

        let result = generate_prompt_with_options(
            git_root,
            &instruction_file,
            &GeneratePromptOptions {
                singular: true,
                force_global: false,
                include_references: false,
                excludes: vec![],
                diff_branch: None,
                targeted: false,
            },
        );

        assert!(
            result.is_ok(),
            "Pipeline should succeed: {:?}",
            result.err()
        );
        let output = result.unwrap();
        assert!(
            output
                .final_prompt
                .contains("// Enclosing function context:"),
            "Should contain enclosing function context header"
        );
        assert!(
            output.final_prompt.contains("func processData()"),
            "Should contain the function declaration"
        );
        assert!(
            output.final_prompt.contains("let result = compute()"),
            "Should contain the function body"
        );
    }

    /// Characterizes that when TODO is INSIDE markers, no enclosing function
    /// context is appended (gating logic suppresses it).
    #[test]
    fn char_todo_inside_markers_suppresses_enclosing_context() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let git_root = temp_dir.path().to_str().unwrap();

        let file_content = concat!(
            "import Foundation\n",
            "func outerFunc() {\n",
            "    let x = 1\n",
            "}\n",
            "// v\n",
            "func markedFunc() {\n",
            "    // TODO: - Inside markers\n",
            "}\n",
            "// ^\n",
        );
        let instruction_file = write_temp_file(temp_dir.path(), "Marked.swift", file_content);

        let result = generate_prompt_with_options(
            git_root,
            &instruction_file,
            &GeneratePromptOptions {
                singular: true,
                force_global: false,
                include_references: false,
                excludes: vec![],
                diff_branch: None,
                targeted: false,
            },
        );

        assert!(
            result.is_ok(),
            "Pipeline should succeed: {:?}",
            result.err()
        );
        let output = result.unwrap();
        assert!(
            !output
                .final_prompt
                .contains("// Enclosing function context:"),
            "Should NOT contain enclosing context when TODO is inside markers"
        );
    }

    /// Characterizes that a file with markers and a class-level TODO where a
    /// function also appears does get enclosing context from the FUNCTION (not
    /// the class). This proves the assembly path uses function-only candidates.
    #[test]
    fn char_class_only_no_enclosing_context_in_assembly() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let git_root = temp_dir.path().to_str().unwrap();

        let file_content = concat!(
            "import Foundation\n",
            "// v\n",
            "let constant = 99\n",
            "// ^\n",
            "class Widget {\n",
            "    func doWork() {\n",
            "        let x = 1\n",
            "        // TODO: - Add validation\n",
            "    }\n",
            "}\n",
        );
        let instruction_file = write_temp_file(temp_dir.path(), "Widget.swift", file_content);

        let result = generate_prompt_with_options(
            git_root,
            &instruction_file,
            &GeneratePromptOptions {
                singular: true,
                force_global: false,
                include_references: false,
                excludes: vec![],
                diff_branch: None,
                targeted: false,
            },
        );

        assert!(
            result.is_ok(),
            "Pipeline should succeed: {:?}",
            result.err()
        );
        let output = result.unwrap();
        assert!(
            output
                .final_prompt
                .contains("// Enclosing function context:"),
            "Should contain enclosing context from the function"
        );
        assert!(
            output.final_prompt.contains("func doWork()"),
            "Enclosing block should be the function, not the class"
        );
    }

    /// Characterizes that extract_types finds class types from enclosing-block
    /// extraction when the class is the last candidate before the TODO. The
    /// type-predicate extension allows classes to be candidates (divergence from
    /// the assembly path which only recognizes functions).
    #[test]
    fn char_extract_types_finds_class_with_markers() {
        let temp_dir = tempdir().expect("Failed to create temp dir");

        // Class is the LAST candidate before TODO (no intervening function)
        let file_content = concat!(
            "import Foundation\n",
            "// v\n",
            "let constant = 99\n",
            "// ^\n",
            "class Widget {\n",
            "    var name = \"hello\"\n",
            "    // TODO: - Add validation\n",
            "}\n",
        );
        let instruction_file = write_temp_file(temp_dir.path(), "Widget.swift", file_content);

        let types = extract_types::extract_types_from_file(&instruction_file)
            .expect("extract_types should succeed");

        assert!(
            types.contains("Widget"),
            "extract_types should find Widget via enclosing-block with type predicate, got: {:?}",
            types
        );
    }
}

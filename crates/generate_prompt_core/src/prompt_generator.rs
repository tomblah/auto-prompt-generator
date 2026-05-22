// crates/generate_prompt_core/src/prompt_generator.rs

use anyhow::{anyhow, Context, Result};
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

    let file_path_str = file_path.to_string_lossy();

    if options.include_references && !file_path_str.ends_with(".swift") {
        return Err(anyhow!(
            "--include-references is only supported for Swift files"
        ));
    }

    let base_dir = PathBuf::from(git_root);

    let search_root_path = if options.force_global {
        println!("Force global enabled: using Git root for context");
        base_dir.clone()
    } else {
        search_root::determine_search_root(&base_dir, file_path)
    };
    println!("Search root: {}", search_root_path.display());

    let instruction_content =
        extract_instruction_content(file_path).context("Failed to extract instruction content")?;
    println!("Instruction content: {}", instruction_content.trim());
    println!("--------------------------------------------------");

    let found_files = file_selector::determine_files_to_include_with_options(
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
        assemble_prompt::assemble_prompt_with_options(&found_files, &assembly_options)
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
        found_files,
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

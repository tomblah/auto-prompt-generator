// crates/generate_prompt/src/lib.rs

use anyhow::{Context, Result};
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use extract_instruction_content::extract_instruction_content;
use get_search_roots::get_search_roots;
use get_git_root::get_git_root;
use find_prompt_instruction::find_prompt_instruction_in_dir;
use extract_types::extract_types_from_file;
use extract_enclosing_type::extract_enclosing_type;
use find_referencing_files;
use assemble_prompt::assemble_prompt;
use find_definition_files::find_definition_files;
use post_processing;

/// Configuration struct for generating the prompt.
pub struct PromptConfig {
    /// Only include the TODO file.
    pub singular: bool,
    /// Force global context inclusion.
    pub force_global: bool,
    /// Include files that reference the enclosing type.
    pub include_references: bool,
    /// Optional branch name for diff mode.
    pub diff_with: Option<String>,
    /// List of file basenames to exclude.
    pub excludes: Vec<String>,
    /// Enable verbose logging.
    pub verbose: bool,
}

/// Generates the final prompt string based on the provided configuration.
/// This function encapsulates the bulk of the business logic previously found in main.rs.
pub fn generate_prompt(config: PromptConfig) -> Result<String> {
    // 1. Save the current directory and determine the Git root.
    let _current_dir = env::current_dir().context("Failed to get current directory")?;
    let git_root = if let Ok(git_root_override) = env::var("GET_GIT_ROOT") {
        git_root_override
    } else {
        get_git_root().expect("Failed to determine Git root")
    };

    // 2. Set up diff mode: if DIFF_WITH_BRANCH is not set, use the provided diff branch.
    if env::var("DIFF_WITH_BRANCH").is_err() {
        if let Some(ref diff_branch) = config.diff_with {
            env::set_var("DIFF_WITH_BRANCH", diff_branch);
        }
    }
    // Verify the diff branch (if set) exists.
    if let Ok(diff_branch) = env::var("DIFF_WITH_BRANCH") {
        let verify_status = Command::new("git")
            .args(&["rev-parse", "--verify", &diff_branch])
            .current_dir(&git_root)
            .stderr(Stdio::null())
            .status()
            .unwrap_or_else(|err| {
                panic!("Error executing git rev-parse: {}", err);
            });
        if !verify_status.success() {
            return Err(anyhow::anyhow!(
                "Error: Branch '{}' does not exist.",
                diff_branch
            ));
        }
    }

    // 3. Change directory to the Git root.
    env::set_current_dir(&git_root).context("Failed to change directory to Git root")?;

    // 4. Locate the TODO instruction file.
    let file_path = if let Ok(instruction_override) = env::var("GET_INSTRUCTION_FILE") {
        instruction_override
    } else {
        let instruction_path_buf = find_prompt_instruction_in_dir(&git_root, false)
            .context("Failed to locate the TODO instruction")?;
        instruction_path_buf.to_string_lossy().into_owned()
    };

    // 5. Set the environment variable TODO_FILE_BASENAME.
    let todo_file_basename = PathBuf::from(&file_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    env::set_var("TODO_FILE_BASENAME", &todo_file_basename);

    // Check JavaScript support and --include-references option.
    if file_path.ends_with(".js") && !config.singular {
        eprintln!("WARNING: JavaScript support is beta – enforcing singular mode.");
    }
    if config.include_references && !file_path.ends_with(".swift") {
        return Err(anyhow::anyhow!(
            "Error: --include-references is only supported for Swift files."
        ));
    }

    // 6. Determine package scope.
    let base_dir = PathBuf::from(&git_root);
    let candidate_roots = get_search_roots(&base_dir).unwrap_or_else(|_| vec![base_dir.clone()]);
    let search_root = if candidate_roots.len() == 1 {
        candidate_roots[0].clone()
    } else {
        let todo_path = PathBuf::from(&file_path);
        candidate_roots
            .into_iter()
            .find(|p| todo_path.starts_with(p))
            .unwrap_or(base_dir)
    };

    // 7. Extract the instruction content.
    let instruction_content = extract_instruction_content(&file_path)
        .context("Failed to extract instruction content")?;

    // 8. Determine files to include.
    let mut found_files: Vec<String> = Vec::new();
    if config.singular {
        found_files.push(file_path.clone());
    } else {
        // Extract types from the file.
        let types_content = extract_types_from_file(&file_path)
            .context("Failed to extract types")?;
        // Find definition files based on the extracted types.
        let def_files_set = find_definition_files(types_content.as_str(), &search_root)
            .map_err(|err| anyhow::anyhow!("Failed to find definition files: {}", err))?;
        for path in def_files_set {
            found_files.push(path.to_string_lossy().into_owned());
        }
        // Append the instruction file.
        found_files.push(file_path.clone());
        // Apply initial exclusion filtering.
        if !config.excludes.is_empty() {
            found_files.retain(|line| {
                let basename = Path::new(line)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                !config.excludes.contains(&basename.to_string())
            });
        }
    }

    // 9. Optionally include referencing files.
    if config.include_references {
        let enclosing_type = match extract_enclosing_type(&file_path) {
            Ok(ty) => ty,
            Err(err) => {
                eprintln!("Error extracting enclosing type: {}", err);
                String::new()
            }
        };
        if !enclosing_type.is_empty() {
            let referencing_files = find_referencing_files::find_files_referencing(
                &enclosing_type,
                search_root.to_str().unwrap(),
            )
            .map_err(|e| anyhow::anyhow!("Failed to find referencing files: {}", e))?;
            found_files.extend(referencing_files);
        }
        // Reapply exclusion filtering.
        if !config.excludes.is_empty() {
            found_files.retain(|line| {
                let basename = Path::new(line)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                !config.excludes.contains(&basename.to_string())
            });
        }
    }

    // Sort and deduplicate the file list.
    found_files.sort();
    found_files.dedup();

    // 10. Assemble the final prompt.
    let mut final_prompt =
        assemble_prompt(&found_files, instruction_content.trim()).context("Failed to assemble prompt")?;

    // 11. Post-process the prompt to scrub extra TODO markers.
    let diff_enabled = env::var("DIFF_WITH_BRANCH").is_ok();
    final_prompt = post_processing::scrub_extra_todo_markers(
        &final_prompt,
        diff_enabled,
        instruction_content.trim(),
    )
    .unwrap_or_else(|err| {
        panic!("Error during post-processing: {}", err);
    });

    // 12. Verify the number of marker lines.
    let marker = "// TODO: -";
    let marker_lines: Vec<&str> = final_prompt.lines().filter(|line| line.contains(marker)).collect();
    if diff_enabled {
        if marker_lines.len() != 2 && marker_lines.len() != 3 {
            return Err(anyhow::anyhow!(
                "Expected 2 or 3 {} markers (with diff enabled), but found {}.",
                marker,
                marker_lines.len()
            ));
        }
    } else if marker_lines.len() != 2 {
        return Err(anyhow::anyhow!(
            "Expected exactly 2 {} markers, but found {}.",
            marker,
            marker_lines.len()
        ));
    }

    Ok(final_prompt)
}

// crates/generate_prompt/src/config.rs

#![allow(dead_code)]

/// Centralized runtime configuration composed from CLI + environment.
/// This is introduced with *no behavior change*; the CLI still calls
/// into the existing orchestration, but we can now log a single struct.
#[derive(Clone, Debug)]
pub struct AppConfig {
    pub git_root: String,
    pub instruction_file: String,
    pub singular: bool,
    pub force_global: bool,
    pub include_references: bool,
    pub excludes: Vec<String>,
    pub diff_branch: Option<String>,     // None == no diff
    pub targeted: bool,                  // mirrors TARGETED env / --tgtd
    pub disable_pbcopy: bool,            // mirrors DISABLE_PBCOPY env
    pub todo_file_basename: String,      // derived from instruction file
    pub verbose: bool,
}

// crates/generate_prompt_core/src/file_selector.rs

use anyhow::Result;
use extract_enclosing_type::extract_enclosing_type;
use extract_types::{extract_types_from_file_with_options, ExtractTypesOptions};
use find_definition_files::find_definition_files_from_sources;
use find_referencing_files::find_files_referencing_from_sources;
use get_search_roots::get_search_roots;
use lang_support::walk_source_files;
use log::{debug, info, warn};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Default)]
pub struct FileSelectionOptions {
    pub include_references: bool,
    pub targeted: bool,
}

#[derive(Debug)]
pub struct FileSelectionResult {
    pub files: Vec<PathBuf>,
    pub types_found: std::collections::BTreeSet<String>,
}

/// Determines the list of files to include in the prompt based on the given parameters.
///
/// - If `singular` is true, only the instruction file (TODO file) is included.
/// - Otherwise, it extracts types from the instruction file, uses those to find definition files,
///   appends the instruction file, and applies exclusion filtering.
/// - If `include_references` is enabled, it also searches for files referencing the enclosing type.
///
/// # Arguments
///
/// * `file_path` - The path to the instruction (TODO) file.
/// * `singular` - Whether to operate in singular mode.
/// * `search_root` - The search root directory for definition file lookup.
/// * `excludes` - A slice of file basename strings to exclude.
/// * `options` - Explicit file-selection behavior.
///
/// # Returns
///
/// A sorted, deduplicated vector of file paths that should be included in the final prompt.
pub fn determine_files_to_include_with_options(
    file_path: &Path,
    singular: bool,
    search_root: &Path,
    excludes: &[String],
    options: &FileSelectionOptions,
) -> Result<FileSelectionResult> {
    let mut found_files: Vec<PathBuf> = Vec::new();
    let mut types_found = std::collections::BTreeSet::new();

    let needs_source_walk = !singular || options.include_references;
    let sources = if needs_source_walk {
        walk_all_search_roots(search_root)
    } else {
        Vec::new()
    };

    if singular {
        info!("Singular mode enabled: only including the TODO file");
        found_files.push(file_path.to_path_buf());
    } else {
        let types = extract_types_from_file_with_options(
            file_path,
            &ExtractTypesOptions {
                targeted: options.targeted,
            },
        )?;
        debug!("Types found:");
        for ty in &types {
            debug!("{}", ty);
        }
        debug!("--------------------------------------------------");

        let def_files_set = find_definition_files_from_sources(&types, &sources);
        types_found = types;

        for path in def_files_set {
            found_files.push(path);
        }

        found_files.push(file_path.to_path_buf());
    }

    if options.include_references {
        debug!("Including files that reference the enclosing type");
        let enclosing_type = match extract_enclosing_type(file_path) {
            Ok(ty) => ty,
            Err(err) => {
                warn!("Error extracting enclosing type: {}", err);
                String::new()
            }
        };
        if !enclosing_type.is_empty() {
            debug!("Enclosing type: {}", enclosing_type);
            debug!("Searching for files referencing {}", enclosing_type);
            let referencing_files = find_files_referencing_from_sources(&enclosing_type, &sources)?;
            found_files.extend(referencing_files);
        } else {
            debug!("No enclosing type found; skipping reference search.");
        }
    }

    if !excludes.is_empty() {
        debug!("Excluding files matching: {:?}", excludes);
        found_files.retain(|p| {
            let basename = p.file_name().unwrap_or_default().to_string_lossy();
            !excludes.contains(&basename.to_string())
        });
    }

    found_files.sort();
    found_files.dedup();
    debug!("--------------------------------------------------");
    debug!("Files (final list):");
    for file in &found_files {
        let basename = file.file_name().unwrap_or_default().to_string_lossy();
        debug!("{}", basename);
    }

    Ok(FileSelectionResult {
        files: found_files,
        types_found,
    })
}

/// Walks all search roots once to produce a single source-file collection.
///
/// Mirrors what `find_definition_files` did internally: resolve search roots
/// via `get_search_roots`, then walk each one. Because `walk_source_files`
/// recurses and roots may overlap, duplicates are removed by path.
fn walk_all_search_roots(search_root: &Path) -> Vec<lang_support::SourceFile> {
    let roots = get_search_roots(search_root).unwrap_or_else(|_| vec![search_root.to_path_buf()]);

    if roots.len() == 1 {
        return walk_source_files(&roots[0]);
    }

    let mut seen = std::collections::BTreeSet::new();
    let mut all = Vec::new();
    for root in &roots {
        for sf in walk_source_files(root) {
            if seen.insert(sf.path.clone()) {
                all.push(sf);
            }
        }
    }
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::File;
    use std::io::Write;
    use tempfile::{tempdir, NamedTempFile};

    /// In singular mode, only the instruction file should be returned.
    #[test]
    fn test_determine_files_singular() {
        // Create a temporary instruction file.
        let mut temp_instr = NamedTempFile::new().unwrap();
        writeln!(temp_instr, "// TODO: - Fix TypeA").unwrap();
        let instr_path = temp_instr.path().to_path_buf();

        // Use the file's parent as search root.
        let search_root = temp_instr.path().parent().unwrap().to_path_buf();

        let result = determine_files_to_include_with_options(
            &instr_path,
            true,
            &search_root,
            &[],
            &FileSelectionOptions {
                include_references: false,
                targeted: false,
            },
        )
        .expect("Failed in singular mode");
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files[0], instr_path);
    }

    /// In non-singular mode (without references), if the instruction file contains a TODO that mentions
    /// "TypeA" and a definition file defines "class TypeA { }", both should be returned.
    #[test]
    fn test_determine_files_non_singular_without_references() {
        let temp_dir = tempdir().unwrap();
        let search_root = temp_dir.path().to_path_buf();

        // Create an instruction file that mentions "TypeA".
        // Include a class declaration so that the enclosing type is "TypeA".
        let instr_path = temp_dir.path().join("Instruction.swift");
        {
            let mut f = File::create(&instr_path).unwrap();
            writeln!(f, "class TypeA {{ }}").unwrap();
            writeln!(f, "// TODO: - Fix TypeA").unwrap();
            writeln!(f, "Some extra info").unwrap();
        }

        // Create a definition file that defines "class TypeA { }".
        let def_path = temp_dir.path().join("Def.swift");
        {
            let mut f = File::create(&def_path).unwrap();
            writeln!(f, "class TypeA {{ }}").unwrap();
        }

        let result = determine_files_to_include_with_options(
            &instr_path,
            false,
            &search_root,
            &[],
            &FileSelectionOptions {
                include_references: false,
                targeted: false,
            },
        )
        .expect("Non-singular without references failed");
        assert!(result.files.contains(&instr_path));
        assert!(result.files.contains(&def_path));
        assert_eq!(result.files.len(), 2);
    }

    #[test]
    fn test_explicit_targeted_false_ignores_targeted_env() {
        env::set_var("TARGETED", "1");
        let temp_dir = tempdir().unwrap();
        let search_root = temp_dir.path().to_path_buf();

        let instr_path = temp_dir.path().join("Instruction.swift");
        {
            let mut f = File::create(&instr_path).unwrap();
            writeln!(f, "class OuterType {{ }}").unwrap();
            writeln!(f, "func testFunction() {{").unwrap();
            writeln!(f, "    class InnerType {{ }}").unwrap();
            writeln!(f, "    // TODO: - Perform action").unwrap();
            writeln!(f, "}}").unwrap();
        }

        let outer_def_path = temp_dir.path().join("OuterDefinition.swift");
        {
            let mut f = File::create(&outer_def_path).unwrap();
            writeln!(f, "class OuterType {{ }}").unwrap();
        }

        let inner_def_path = temp_dir.path().join("InnerDefinition.swift");
        {
            let mut f = File::create(&inner_def_path).unwrap();
            writeln!(f, "class InnerType {{ }}").unwrap();
        }

        let result = determine_files_to_include_with_options(
            &instr_path,
            false,
            &search_root,
            &[],
            &FileSelectionOptions {
                include_references: false,
                targeted: false,
            },
        )
        .expect("Explicit non-targeted selection failed");

        env::remove_var("TARGETED");

        assert!(result.files.contains(&instr_path));
        assert!(result.files.contains(&outer_def_path));
        assert!(result.files.contains(&inner_def_path));
    }

    #[test]
    fn test_explicit_targeted_true_ignores_absent_targeted_env() {
        env::remove_var("TARGETED");
        let temp_dir = tempdir().unwrap();
        let search_root = temp_dir.path().to_path_buf();

        let instr_path = temp_dir.path().join("Instruction.swift");
        {
            let mut f = File::create(&instr_path).unwrap();
            writeln!(f, "class OuterType {{ }}").unwrap();
            writeln!(f, "func testFunction() {{").unwrap();
            writeln!(f, "    class InnerType {{ }}").unwrap();
            writeln!(f, "    // TODO: - Perform action").unwrap();
            writeln!(f, "}}").unwrap();
        }

        let outer_def_path = temp_dir.path().join("OuterDefinition.swift");
        {
            let mut f = File::create(&outer_def_path).unwrap();
            writeln!(f, "class OuterType {{ }}").unwrap();
        }

        let inner_def_path = temp_dir.path().join("InnerDefinition.swift");
        {
            let mut f = File::create(&inner_def_path).unwrap();
            writeln!(f, "class InnerType {{ }}").unwrap();
        }

        let result = determine_files_to_include_with_options(
            &instr_path,
            false,
            &search_root,
            &[],
            &FileSelectionOptions {
                include_references: false,
                targeted: true,
            },
        )
        .expect("Explicit targeted selection failed");

        assert!(result.files.contains(&instr_path));
        assert!(!result.files.contains(&outer_def_path));
        assert!(result.files.contains(&inner_def_path));
    }

    /// In non-singular mode with references enabled, if the instruction file declares "RefType"
    /// and a definition file as well as a referencing file both match, all should be returned.
    #[test]
    fn test_determine_files_non_singular_with_references() {
        let temp_dir = tempdir().unwrap();
        let search_root = temp_dir.path().to_path_buf();

        // Create an instruction file that includes a class declaration for RefType.
        let instr_path = temp_dir.path().join("Instruction.swift");
        {
            let mut f = File::create(&instr_path).unwrap();
            writeln!(f, "class RefType {{ }}").unwrap();
            writeln!(f, "// TODO: - Fix RefType").unwrap();
        }

        // Create a definition file that defines RefType.
        let def_path = temp_dir.path().join("Def.swift");
        {
            let mut f = File::create(&def_path).unwrap();
            writeln!(f, "class RefType {{ }}").unwrap();
        }

        // Create a referencing file that mentions RefType.
        let ref_path = temp_dir.path().join("Ref.swift");
        {
            let mut f = File::create(&ref_path).unwrap();
            writeln!(f, "let x = RefType()").unwrap();
        }

        let result = determine_files_to_include_with_options(
            &instr_path,
            false,
            &search_root,
            &[],
            &FileSelectionOptions {
                include_references: true,
                targeted: false,
            },
        )
        .expect("Non-singular with references failed");
        assert!(result.files.contains(&instr_path));
        assert!(result.files.contains(&def_path));
        assert!(result.files.contains(&ref_path));
        assert_eq!(result.files.len(), 3);
    }

    #[test]
    fn test_include_references_skips_search_when_enclosing_type_errors() {
        let temp_dir = tempdir().unwrap();
        let missing_instruction = temp_dir.path().join("MissingInstruction.swift");

        let result = determine_files_to_include_with_options(
            &missing_instruction,
            true,
            temp_dir.path(),
            &[],
            &FileSelectionOptions {
                include_references: true,
                targeted: false,
            },
        )
        .expect("Missing file should only skip reference lookup");

        assert_eq!(result.files, vec![missing_instruction]);
    }

    /// Test exclusion filtering in non-singular mode with references enabled.
    /// In this test, we modify the instruction file to declare "TypeA" so that the enclosing type becomes "TypeA".
    /// Then we create a definition file and a referencing file both mentioning "TypeA". With "Def.swift" excluded,
    /// the final list should contain Instruction.swift and Ref.swift.
    #[test]
    fn test_determine_files_exclusion() {
        let temp_dir = tempdir().unwrap();
        let search_root = temp_dir.path().to_path_buf();

        // Create an instruction file that declares TypeA and includes a TODO marker.
        let instr_path = temp_dir.path().join("Instruction.swift");
        {
            let mut f = File::create(&instr_path).unwrap();
            writeln!(f, "class TypeA {{ }}").unwrap();
            writeln!(f, "// TODO: - Fix TypeA").unwrap();
        }

        // Create a definition file that defines "class TypeA { }".
        let def_path = temp_dir.path().join("Def.swift");
        {
            let mut f = File::create(&def_path).unwrap();
            writeln!(f, "class TypeA {{ }}").unwrap();
        }

        // Create a referencing file that mentions "TypeA".
        let ref_path = temp_dir.path().join("Ref.swift");
        {
            let mut f = File::create(&ref_path).unwrap();
            writeln!(f, "let x = TypeA()").unwrap();
        }

        // Now use include_references = true and exclude "Def.swift".
        let result = determine_files_to_include_with_options(
            &instr_path,
            false,
            &search_root,
            &["Def.swift".to_string()],
            &FileSelectionOptions {
                include_references: true,
                targeted: false,
            },
        )
        .expect("Exclusion test failed");
        assert!(result.files.contains(&instr_path));
        assert!(result.files.contains(&ref_path));
        assert!(!result.files.contains(&def_path));
        assert_eq!(result.files.len(), 2);
    }
}

#[cfg(test)]
mod walk_unification_characterization_tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    fn swift_project(
        types: &[(&str, &str)],
        instruction: &str,
        instruction_filename: &str,
    ) -> (tempfile::TempDir, PathBuf) {
        let dir = tempdir().unwrap();
        for (name, content) in types {
            let path = dir.path().join(name);
            let mut f = File::create(&path).unwrap();
            write!(f, "{}", content).unwrap();
        }
        let instr_path = dir.path().join(instruction_filename);
        {
            let mut f = File::create(&instr_path).unwrap();
            write!(f, "{}", instruction).unwrap();
        }
        (dir, instr_path)
    }

    #[test]
    fn char_references_and_definitions_are_merged_sorted_and_deduped() {
        let (dir, instr_path) = swift_project(
            &[
                ("Alpha.swift", "class Alpha {}\nlet x = Beta()\n"),
                ("Beta.swift", "class Beta {}\n"),
                ("Gamma.swift", "let y = Alpha()\n"),
            ],
            "class Alpha {}\n// TODO: - Fix Alpha\n",
            "Instruction.swift",
        );

        let result = determine_files_to_include_with_options(
            &instr_path,
            false,
            dir.path(),
            &[],
            &FileSelectionOptions {
                include_references: true,
                targeted: false,
            },
        )
        .expect("merged references + definitions failed");

        assert!(result.files.contains(&dir.path().join("Alpha.swift")));
        assert!(result.files.contains(&dir.path().join("Gamma.swift")));
        assert!(result.files.contains(&instr_path));

        let mut sorted = result.files.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(result.files, sorted, "output must be sorted and deduped");
    }

    #[test]
    fn char_exclusion_applies_to_both_definitions_and_references() {
        let (dir, instr_path) = swift_project(
            &[
                ("Def.swift", "class Widget {}\n"),
                ("Ref.swift", "let w = Widget()\n"),
            ],
            "class Widget {}\n// TODO: - Fix Widget\n",
            "Instruction.swift",
        );

        let result = determine_files_to_include_with_options(
            &instr_path,
            false,
            dir.path(),
            &["Ref.swift".to_string()],
            &FileSelectionOptions {
                include_references: true,
                targeted: false,
            },
        )
        .expect("exclusion across both paths failed");

        assert!(
            !result.files.contains(&dir.path().join("Ref.swift")),
            "Ref.swift should be excluded"
        );
        assert!(result.files.contains(&dir.path().join("Def.swift")));
        assert!(result.files.contains(&instr_path));
    }

    #[test]
    fn char_exclusion_of_definition_does_not_suppress_same_file_as_reference() {
        let (dir, instr_path) = swift_project(
            &[("Both.swift", "class Both {}\nlet b = Both()\n")],
            "class Both {}\n// TODO: - Fix Both\n",
            "Instruction.swift",
        );

        let result_without_exclude = determine_files_to_include_with_options(
            &instr_path,
            false,
            dir.path(),
            &[],
            &FileSelectionOptions {
                include_references: true,
                targeted: false,
            },
        )
        .expect("without exclude failed");

        assert!(result_without_exclude
            .files
            .contains(&dir.path().join("Both.swift")));

        let result_with_exclude = determine_files_to_include_with_options(
            &instr_path,
            false,
            dir.path(),
            &["Both.swift".to_string()],
            &FileSelectionOptions {
                include_references: true,
                targeted: false,
            },
        )
        .expect("with exclude failed");

        assert!(
            !result_with_exclude
                .files
                .contains(&dir.path().join("Both.swift")),
            "Both.swift excluded from both definition and reference paths"
        );
    }

    #[test]
    fn char_no_references_without_include_references_flag() {
        let (dir, instr_path) = swift_project(
            &[
                ("Def.swift", "class Gadget {}\n"),
                ("Ref.swift", "let g = Gadget()\n"),
            ],
            "class Gadget {}\n// TODO: - Fix Gadget\n",
            "Instruction.swift",
        );

        let result = determine_files_to_include_with_options(
            &instr_path,
            false,
            dir.path(),
            &[],
            &FileSelectionOptions {
                include_references: false,
                targeted: false,
            },
        )
        .expect("no-references failed");

        assert!(result.files.contains(&dir.path().join("Def.swift")));
        assert!(
            !result.files.contains(&dir.path().join("Ref.swift")),
            "Ref.swift should not appear without include_references"
        );
    }

    #[test]
    fn char_types_found_populated_in_non_singular_mode() {
        let (dir, instr_path) = swift_project(
            &[("Foo.swift", "class Foo {}\n")],
            "class Foo {}\n// TODO: - Fix Foo\n",
            "Instruction.swift",
        );

        let result = determine_files_to_include_with_options(
            &instr_path,
            false,
            dir.path(),
            &[],
            &FileSelectionOptions {
                include_references: false,
                targeted: false,
            },
        )
        .expect("types_found test failed");

        assert!(
            result.types_found.contains("Foo"),
            "types_found should contain Foo"
        );
    }

    #[test]
    fn char_singular_mode_with_references_still_searches_references() {
        let (dir, instr_path) = swift_project(
            &[("Ref.swift", "let v = Singular()\n")],
            "class Singular {}\n// TODO: - Fix Singular\n",
            "Instruction.swift",
        );

        let result = determine_files_to_include_with_options(
            &instr_path,
            true,
            dir.path(),
            &[],
            &FileSelectionOptions {
                include_references: true,
                targeted: false,
            },
        )
        .expect("singular + references failed");

        assert!(result.files.contains(&instr_path));
        assert!(
            result.files.contains(&dir.path().join("Ref.swift")),
            "references should still be found in singular + include_references mode"
        );
    }
}

// crates/generate_prompt/src/file_selector.rs

use anyhow::Result;
use extract_enclosing_type::extract_enclosing_type;
use extract_types::{extract_types_from_file_with_options, ExtractTypesOptions};
use find_definition_files::find_definition_files;
use std::path::Path;

#[derive(Debug, Clone, Copy, Default)]
pub struct FileSelectionOptions {
    pub include_references: bool,
    pub targeted: bool,
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
/// A vector of file paths (as Strings) that should be included in the final prompt.
pub fn determine_files_to_include_with_options(
    file_path: &str,
    singular: bool,
    search_root: &Path,
    excludes: &[String],
    options: &FileSelectionOptions,
) -> Result<Vec<String>> {
    let mut found_files: Vec<String> = Vec::new();

    if singular {
        println!("Singular mode enabled: only including the TODO file");
        found_files.push(file_path.to_string());
    } else {
        let types = extract_types_from_file_with_options(
            file_path,
            &ExtractTypesOptions {
                targeted: options.targeted,
            },
        )?;
        println!("Types found:");
        for ty in &types {
            println!("{}", ty);
        }
        println!("--------------------------------------------------");

        let def_files_set = find_definition_files(&types, search_root)?;

        // Add definition files to the in-memory list.
        for path in def_files_set {
            found_files.push(path.to_string_lossy().into_owned());
        }

        // Append the instruction file.
        found_files.push(file_path.to_string());

        // Apply exclusion filtering.
        if !excludes.is_empty() {
            println!("Excluding files matching: {:?}", excludes);
            found_files.retain(|line| {
                let basename = Path::new(line)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                !excludes.contains(&basename.to_string())
            });
        }
    }

    if options.include_references {
        println!("Including files that reference the enclosing type");
        let enclosing_type = match extract_enclosing_type(file_path) {
            Ok(ty) => ty,
            Err(err) => {
                eprintln!("Error extracting enclosing type: {}", err);
                String::new()
            }
        };
        if !enclosing_type.is_empty() {
            println!("Enclosing type: {}", enclosing_type);
            println!("Searching for files referencing {}", enclosing_type);
            let referencing_files = find_referencing_files::find_files_referencing(
                &enclosing_type,
                search_root.to_str().unwrap(),
            )?;
            found_files.extend(referencing_files);
        } else {
            println!("No enclosing type found; skipping reference search.");
        }
        // Reapply exclusion filtering.
        if !excludes.is_empty() {
            println!("Excluding files matching: {:?}", excludes);
            found_files.retain(|line| {
                let basename = Path::new(line)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                !excludes.contains(&basename.to_string())
            });
        }
    }

    // Sort and deduplicate.
    found_files.sort();
    found_files.dedup();
    println!("--------------------------------------------------");
    println!("Files (final list):");
    for file in &found_files {
        let basename = Path::new(file)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        println!("{}", basename);
    }

    Ok(found_files)
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
        let instr_path = temp_instr.path().to_str().unwrap().to_string();

        // Use the file's parent as search root.
        let search_root = temp_instr.path().parent().unwrap().to_path_buf();

        let files = determine_files_to_include_with_options(
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
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], instr_path);
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

        let files = determine_files_to_include_with_options(
            instr_path.to_str().unwrap(),
            false,
            &search_root,
            &[],
            &FileSelectionOptions {
                include_references: false,
                targeted: false,
            },
        )
        .expect("Non-singular without references failed");
        // Expect both the instruction file and the definition file.
        assert!(files.contains(&instr_path.to_str().unwrap().to_string()));
        assert!(files.contains(&def_path.to_str().unwrap().to_string()));
        assert_eq!(files.len(), 2);
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

        let files = determine_files_to_include_with_options(
            instr_path.to_str().unwrap(),
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

        assert!(files.contains(&instr_path.to_str().unwrap().to_string()));
        assert!(files.contains(&outer_def_path.to_str().unwrap().to_string()));
        assert!(files.contains(&inner_def_path.to_str().unwrap().to_string()));
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

        let files = determine_files_to_include_with_options(
            instr_path.to_str().unwrap(),
            false,
            &search_root,
            &[],
            &FileSelectionOptions {
                include_references: false,
                targeted: true,
            },
        )
        .expect("Explicit targeted selection failed");

        assert!(files.contains(&instr_path.to_str().unwrap().to_string()));
        assert!(!files.contains(&outer_def_path.to_str().unwrap().to_string()));
        assert!(files.contains(&inner_def_path.to_str().unwrap().to_string()));
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

        let files = determine_files_to_include_with_options(
            instr_path.to_str().unwrap(),
            false,
            &search_root,
            &[],
            &FileSelectionOptions {
                include_references: true,
                targeted: false,
            },
        )
        .expect("Non-singular with references failed");
        // Expected: Instruction.swift, Def.swift, and Ref.swift.
        assert!(files.contains(&instr_path.to_str().unwrap().to_string()));
        assert!(files.contains(&def_path.to_str().unwrap().to_string()));
        assert!(files.contains(&ref_path.to_str().unwrap().to_string()));
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_include_references_skips_search_when_enclosing_type_errors() {
        let temp_dir = tempdir().unwrap();
        let missing_instruction = temp_dir.path().join("MissingInstruction.swift");

        let files = determine_files_to_include_with_options(
            missing_instruction.to_str().unwrap(),
            true,
            temp_dir.path(),
            &[],
            &FileSelectionOptions {
                include_references: true,
                targeted: false,
            },
        )
        .expect("Missing file should only skip reference lookup");

        assert_eq!(
            files,
            vec![missing_instruction.to_string_lossy().into_owned()]
        );
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
        let files = determine_files_to_include_with_options(
            instr_path.to_str().unwrap(),
            false,
            &search_root,
            &["Def.swift".to_string()],
            &FileSelectionOptions {
                include_references: true,
                targeted: false,
            },
        )
        .expect("Exclusion test failed");
        // Expected: Instruction.swift and Ref.swift should be present; Def.swift should be excluded.
        assert!(files.contains(&instr_path.to_str().unwrap().to_string()));
        assert!(files.contains(&ref_path.to_str().unwrap().to_string()));
        assert!(!files.contains(&def_path.to_str().unwrap().to_string()));
        // Total count should be 2.
        assert_eq!(files.len(), 2);
    }
}

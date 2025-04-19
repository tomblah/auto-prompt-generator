// crates/generate_prompt/src/file_selector.rs

use std::path::Path;
use anyhow::Result;
use extract_types::extract_types_from_file;
use find_definition_files::find_definition_files;
use extract_enclosing_type::extract_enclosing_type;
use find_referencing_files;

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
/// * `include_references` - Whether to search for and include referencing files.
///
/// # Returns
///
/// A vector of file paths (as Strings) that should be included in the final prompt.
pub fn determine_files_to_include(
    file_path: &str,
    singular: bool,
    search_root: &Path,
    excludes: &[String],
    include_references: bool,
) -> Result<Vec<String>> {
    let mut found_files: Vec<String> = Vec::new();

    if singular {
        log::info!("Singular mode enabled: only including the TODO file");
        found_files.push(file_path.to_string());
    } else {
        // Extract types as a newline-separated string.
        let types_content = extract_types_from_file(file_path)
            .map_err(|e| anyhow::anyhow!("Failed to extract types: {}", e))?;
        log::info!("Types found:");
        log::info!("{}", types_content.trim());
        log::info!("--------------------------------------------------");

        // Find definition files using the extracted types.
        let def_files_set = find_definition_files(types_content.as_str(), search_root)
            .map_err(|err| anyhow::anyhow!("Failed to find definition files: {}", err))?;
        
        // Add definition files to the in-memory list.
        for path in def_files_set {
            found_files.push(path.to_string_lossy().into_owned());
        }
        
        // Append the instruction file.
        found_files.push(file_path.to_string());
        
        // Apply exclusion filtering.
        if !excludes.is_empty() {
            log::info!("Excluding files matching: {:?}", excludes);
            found_files.retain(|line| {
                let basename = Path::new(line)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                !excludes.contains(&basename.to_string())
            });
        }
    }

    if include_references {
        log::info!("Including files that reference the enclosing type");
        let enclosing_type = match extract_enclosing_type(file_path) {
            Ok(ty) => ty,
            Err(err) => {
                log::error!("Error extracting enclosing type: {}", err);
                String::new()
            }
        };
        if !enclosing_type.is_empty() {
            log::info!("Enclosing type: {}", enclosing_type);
            log::info!("Searching for files referencing {}", enclosing_type);
            let referencing_files = find_referencing_files::find_files_referencing(
                &enclosing_type,
                search_root.to_str().unwrap(),
            )
            .map_err(|e| anyhow::anyhow!("Failed to find referencing files: {}", e))?;
            found_files.extend(referencing_files);
        } else {
            log::info!("No enclosing type found; skipping reference search.");
        }
        // Reapply exclusion filtering.
        if !excludes.is_empty() {
            log::info!("Excluding files matching: {:?}", excludes);
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
    log::info!("--------------------------------------------------");
    log::info!("Files (final list):");
    for file in &found_files {
        let basename = Path::new(file)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        log::info!("{}", basename);
    }

    Ok(found_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{tempdir, NamedTempFile};
    use std::fs::File;
    use std::io::Write;

    /// In singular mode, only the instruction file should be returned.
    #[test]
    fn test_determine_files_singular() {
        // Create a temporary instruction file.
        let mut temp_instr = NamedTempFile::new().unwrap();
        writeln!(temp_instr, "// TODO: - Fix TypeA").unwrap();
        let instr_path = temp_instr.path().to_str().unwrap().to_string();

        // Use the file's parent as search root.
        let search_root = temp_instr.path().parent().unwrap().to_path_buf();

        let files = determine_files_to_include(&instr_path, true, &search_root, &[], false)
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

        let files = determine_files_to_include(
            instr_path.to_str().unwrap(),
            false,
            &search_root,
            &[],
            false,
        )
        .expect("Non-singular without references failed");
        // Expect both the instruction file and the definition file.
        assert!(files.contains(&instr_path.to_str().unwrap().to_string()));
        assert!(files.contains(&def_path.to_str().unwrap().to_string()));
        assert_eq!(files.len(), 2);
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

        let files = determine_files_to_include(
            instr_path.to_str().unwrap(),
            false,
            &search_root,
            &[],
            true,
        )
        .expect("Non-singular with references failed");
        // Expected: Instruction.swift, Def.swift, and Ref.swift.
        assert!(files.contains(&instr_path.to_str().unwrap().to_string()));
        assert!(files.contains(&def_path.to_str().unwrap().to_string()));
        assert!(files.contains(&ref_path.to_str().unwrap().to_string()));
        assert_eq!(files.len(), 3);
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
        let files = determine_files_to_include(
            instr_path.to_str().unwrap(),
            false,
            &search_root,
            &["Def.swift".to_string()],
            true,
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

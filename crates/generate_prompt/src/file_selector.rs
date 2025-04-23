// crates/generate_prompt/src/file_selector.rs

use std::path::Path;

use anyhow::Result;
use extract_enclosing_type::extract_enclosing_type;
use extract_types::extract_types_from_file;
use find_definition_files::find_definition_files;
use find_referencing_files;
use lang_support::for_extension;

/// Determines the list of files to include in the prompt based on the given parameters.
///
/// * If `singular` is **true**, only the instruction file (TODO file) is included.
/// * Otherwise we:
///   1. Extract types from the TODO file.
///   2. Locate definition files for those types.
///   3. Optionally walk direct dependencies (per-language).
///   4. Optionally locate files referencing the enclosing type.
///   5. Apply the `excludes` filter.
/// * The resulting list is sorted and deduplicated before returning.
pub fn determine_files_to_include(
    file_path: &str,
    singular: bool,
    search_root: &Path,
    excludes: &[String],
    include_references: bool,
) -> Result<Vec<String>> {
    let mut found_files: Vec<String> = Vec::new();

    // ──────────────────────────────────────────────────────────────────────────
    //  1. Singular mode: nothing but the TODO file
    // ──────────────────────────────────────────────────────────────────────────
    if singular {
        println!("Singular mode enabled: only including the TODO file");
        found_files.push(file_path.to_string());
    } else {
        // ──────────────────────────────────────────────────────────────────────
        //  2. Extract type names and locate their definition files
        // ──────────────────────────────────────────────────────────────────────
        let types_content = extract_types_from_file(file_path)
            .map_err(|e| anyhow::anyhow!("Failed to extract types: {}", e))?;
        println!("Types found:");
        println!("{}", types_content.trim());
        println!("--------------------------------------------------");

        let def_files_set = find_definition_files(types_content.as_str(), search_root)
            .map_err(|err| anyhow::anyhow!("Failed to find definition files: {}", err))?;

        for path in def_files_set {
            found_files.push(path.to_string_lossy().into_owned());
        }

        // Always include the TODO file itself.
        found_files.push(file_path.to_string());

        // ──────────────────────────────────────────────────────────────────────
        //  3. Exclusion filter (initial pass)
        // ──────────────────────────────────────────────────────────────────────
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

    // ──────────────────────────────────────────────────────────────────────────
    //  4. Language-specific dependency walk (NEW)
    // ──────────────────────────────────────────────────────────────────────────
    let mut extra_deps = Vec::new();
    for file in &found_files {
        if let Some(ext) = Path::new(file).extension().and_then(|s| s.to_str()) {
            if let Some(lang) = for_extension(ext) {
                extra_deps.extend(
                    lang.walk_dependencies(Path::new(file), search_root)
                        .into_iter()
                        .map(|p| p.display().to_string()),
                );
            }
        }
    }
    found_files.extend(extra_deps);

    // ──────────────────────────────────────────────────────────────────────────
    //  5. Include files that reference the enclosing type (optional)
    // ──────────────────────────────────────────────────────────────────────────
    if include_references {
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
            )
            .map_err(|e| anyhow::anyhow!("Failed to find referencing files: {}", e))?;
            found_files.extend(referencing_files);
        } else {
            println!("No enclosing type found; skipping reference search.");
        }

        // Re-apply the exclusion filter.
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

    // ──────────────────────────────────────────────────────────────────────────
    //  6. Sort, deduplicate, log
    // ──────────────────────────────────────────────────────────────────────────
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

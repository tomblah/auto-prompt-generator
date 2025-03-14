// crates/generate_prompt/tests/integration_js.rs

#[cfg(test)]
mod integration_tests_js {
    use assert_cmd::Command;
    use std::env;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_js_singular_mode_cloud_code() {
        // Create a temporary directory for our dummy JS project.
        let temp_dir = TempDir::new().unwrap();
        let main_js_path = temp_dir.path().join("main.js");
        let index_js_path = temp_dir.path().join("index.js");

        // Write main.js with marker lines and a Parse.Cloud.define function.
        let main_js_content = r#"
// v
const foo = "include";
// ^

const bar = "exclude";

Parse.Cloud.define("someCloudCodeFunction", async (request) => {
    
    const someObjectId = "12345";
    const someOtherObjectId = "67890";
    
    return somePromise(someObjectId).then(function(someObject) {
        
        return someOtherPromise(someOtherObjectId);
        
    }).then(function(someOtherObject) {
        
        // TODO: - example only

        return true;
        
    });
    
});
"#;
        fs::write(&main_js_path, main_js_content).unwrap();

        // Write index.js (this file should not be included in singular mode).
        let index_js_content = r#"const example = "example";"#;
        fs::write(&index_js_path, index_js_content).unwrap();

        // Set GET_INSTRUCTION_FILE so that main.js is used as the instruction file.
        env::set_var("GET_INSTRUCTION_FILE", main_js_path.to_str().unwrap());
        // Also set GET_GIT_ROOT to our temporary directory.
        env::set_var("GET_GIT_ROOT", temp_dir.path().to_str().unwrap());

        // Set up a dummy pbcopy executable that writes its stdin to a temporary clipboard file.
        let pbcopy_dir = TempDir::new().unwrap();
        let clipboard_file = pbcopy_dir.path().join("clipboard.txt");
        let dummy_pbcopy_path = pbcopy_dir.path().join("pbcopy");
        fs::write(
            &dummy_pbcopy_path,
            format!("#!/bin/sh\ncat > \"{}\"", clipboard_file.display())
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dummy_pbcopy_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dummy_pbcopy_path, perms).unwrap();
        }
        // Prepend the dummy pbcopy directory to the PATH.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        // Ensure that clipboard copying is enabled.
        env::remove_var("DISABLE_PBCOPY");

        // Run the generate_prompt binary in singular mode.
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--singular");
        cmd.assert().success();

        // Read the dummy clipboard file.
        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Assert that the prompt includes the function definition and TODO line from main.js,
        // includes the marked line "const foo = \"include\";",
        // and does not include the unmarked line "const bar = \"exclude\";" nor the content from index.js.
        assert!(clipboard_content.contains("someCloudCodeFunction"),
                "Expected the prompt to include the function name");
        assert!(clipboard_content.contains("// TODO: - example only"),
                "Expected the prompt to include the TODO comment");
        assert!(clipboard_content.contains("const foo = \"include\";"),
                "Expected the prompt to include the 'include' line");
        assert!(!clipboard_content.contains("const bar = \"exclude\";"),
                "Expected the prompt to not include the 'exclude' line");
        assert!(!clipboard_content.contains("const example = \"example\";"),
                "Expected the prompt to not include index.js content");
    }
    
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_js_singular_mode_function_style_1() {
        // Create a temporary directory for our dummy JS project.
        let temp_dir = TempDir::new().unwrap();
        let main_js_path = temp_dir.path().join("main.js");
        let index_js_path = temp_dir.path().join("index.js");

        // Write main.js with marker lines and a function definition.
        let main_js_content = r#"
// v
const foo = "include";
// ^

const bar = "exclude";

someFunction = function(someParameter) {
    
    const someObjectId = "12345";
    const someOtherObjectId = "67890";
    
    return somePromise(someObjectId).then(function(someObject) {
        
        return someOtherPromise(someOtherObjectId);
        
    }).then(function(someOtherObject) {
        
        // TODO: - example only

        return ParsePromiseas(true);
        
    });
}
"#;
        fs::write(&main_js_path, main_js_content).unwrap();

        // Write index.js (this file should not be included in singular mode).
        let index_js_content = r#"const example = "example";"#;
        fs::write(&index_js_path, index_js_content).unwrap();

        // Set GET_INSTRUCTION_FILE so that main.js is used as the instruction file.
        env::set_var("GET_INSTRUCTION_FILE", main_js_path.to_str().unwrap());
        // Also set GET_GIT_ROOT to our temporary directory.
        env::set_var("GET_GIT_ROOT", temp_dir.path().to_str().unwrap());

        // Set up a dummy pbcopy executable that writes its stdin to a temporary clipboard file.
        let pbcopy_dir = TempDir::new().unwrap();
        let clipboard_file = pbcopy_dir.path().join("clipboard.txt");
        let dummy_pbcopy_path = pbcopy_dir.path().join("pbcopy");
        fs::write(
            &dummy_pbcopy_path,
            format!("#!/bin/sh\ncat > \"{}\"", clipboard_file.display())
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dummy_pbcopy_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dummy_pbcopy_path, perms).unwrap();
        }
        // Prepend the dummy pbcopy directory to the PATH.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        // Ensure that clipboard copying is enabled.
        env::remove_var("DISABLE_PBCOPY");

        // Run the generate_prompt binary in singular mode.
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--singular");
        cmd.assert().success();

        // Read the dummy clipboard file.
        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Assert that the prompt includes the function definition and TODO line from main.js,
        // includes the marked line "const foo = \"include\";",
        // and does not include the unmarked line "const bar = \"exclude\";" nor the content from index.js.
        assert!(clipboard_content.contains("someFunction"),
                "Expected the prompt to include the function name");
        assert!(clipboard_content.contains("// TODO: - example only"),
                "Expected the prompt to include the TODO comment");
        assert!(clipboard_content.contains("const foo = \"include\";"),
                "Expected the prompt to include the 'include' line");
        assert!(!clipboard_content.contains("const bar = \"exclude\";"),
                "Expected the prompt to not include the 'exclude' line");
        assert!(!clipboard_content.contains("const example = \"example\";"),
                "Expected the prompt to not include index.js content");
    }
    
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_js_singular_mode_async() {
        // Create a temporary directory for our dummy JS project.
        let temp_dir = TempDir::new().unwrap();
        let main_js_path = temp_dir.path().join("main.js");
        let index_js_path = temp_dir.path().join("index.js");

        // Write main.js with marker lines and an async function declaration.
        let main_js_content = r#"
// v
const foo = "include";
// ^

const bar = "exclude";

async function someFunction(someParameter) {

    const someObjectId = "12345";
    const someOtherObjectId = "67890";
    
    const someObject = await someAsyncFunction(someObjectId);
    const someOtherObject = await someOtherAsyncFunction(someOtherObjectId);

    // TODO: - example only
    return true;
}
"#;
        fs::write(&main_js_path, main_js_content).unwrap();

        // Write index.js (this file should not be included in singular mode).
        let index_js_content = r#"const example = "example";"#;
        fs::write(&index_js_path, index_js_content).unwrap();

        // Set GET_INSTRUCTION_FILE so that main.js is used as the instruction file.
        env::set_var("GET_INSTRUCTION_FILE", main_js_path.to_str().unwrap());
        // Also set GET_GIT_ROOT to our temporary directory.
        env::set_var("GET_GIT_ROOT", temp_dir.path().to_str().unwrap());

        // Set up a dummy pbcopy executable that writes its stdin to a temporary clipboard file.
        let pbcopy_dir = TempDir::new().unwrap();
        let clipboard_file = pbcopy_dir.path().join("clipboard.txt");
        let dummy_pbcopy_path = pbcopy_dir.path().join("pbcopy");
        fs::write(
            &dummy_pbcopy_path,
            format!("#!/bin/sh\ncat > \"{}\"", clipboard_file.display())
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dummy_pbcopy_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dummy_pbcopy_path, perms).unwrap();
        }
        // Prepend the dummy pbcopy directory to the PATH.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        // Ensure that clipboard copying is enabled.
        env::remove_var("DISABLE_PBCOPY");

        // Run the generate_prompt binary in singular mode.
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--singular");
        cmd.assert().success();

        // Read the dummy clipboard file.
        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Assert that the prompt includes the async function definition and TODO line from main.js,
        // includes the marked line "const foo = \"include\";",
        // and does not include the unmarked line "const bar = \"exclude\";" nor the content from index.js.
        assert!(clipboard_content.contains("someFunction"),
                "Expected the prompt to include the async function name");
        assert!(clipboard_content.contains("// TODO: - example only"),
                "Expected the prompt to include the TODO comment");
        assert!(clipboard_content.contains("const foo = \"include\";"),
                "Expected the prompt to include the 'include' line");
        assert!(!clipboard_content.contains("const bar = \"exclude\";"),
                "Expected the prompt to not include the 'exclude' line");
        assert!(!clipboard_content.contains("const example = \"example\";"),
                "Expected the prompt to not include index.js content");
    }
    
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_js_singular_mode_large_complex_function() {
        // Create a temporary directory for our dummy JS project.
        let temp_dir = TempDir::new().unwrap();
        let complex_js_path = temp_dir.path().join("complex.js");
        let other_js_path = temp_dir.path().join("other.js");

        // Write complex.js with a deep, nested function that includes a TODO marker.
        let complex_js_content = r#"
            // Preliminary unrelated content.
            const preliminary = "ignore this line";
            
            // Define a complex function with nested blocks.
            function complexFunction(param) {
                console.log("Start of complexFunction");
                if (param) {
                    for (let i = 0; i < 10; i++) {
                        console.log("Loop iteration", i);
                        if (i % 2 === 0) {
                            console.log("Even iteration");
                        } else {
                            (function nestedFunction() {
                                console.log("Inside nestedFunction");
                                if (i === 5) {
                                    function innerMost() {
                                        console.log("Entering innerMost");
                                        // TODO: - perform complex calculation here
                                        console.log("Exiting innerMost");
                                    }
                                    innerMost();
                                } else {
                                    console.log("No special condition");
                                }
                            })();
                        }
                    }
                }
                console.log("End of complexFunction");
            }
            
            // Another function that should be ignored.
            function otherFunction() {
                console.log("This is other function");
            }
            
            // End of file.
        "#;
        fs::write(&complex_js_path, complex_js_content).unwrap();

        // Write other.js (should be excluded in singular mode).
        let other_js_content = r#"console.log("This is other file");"#;
        fs::write(&other_js_path, other_js_content).unwrap();

        // Set environment variables so that complex.js is the instruction file.
        env::set_var("GET_INSTRUCTION_FILE", complex_js_path.to_str().unwrap());
        env::set_var("GET_GIT_ROOT", temp_dir.path().to_str().unwrap());

        // Set up a dummy pbcopy executable that writes its stdin to a temporary clipboard file.
        let pbcopy_dir = TempDir::new().unwrap();
        let clipboard_file = pbcopy_dir.path().join("clipboard.txt");
        let dummy_pbcopy_path = pbcopy_dir.path().join("pbcopy");
        fs::write(
            &dummy_pbcopy_path,
            format!("#!/bin/sh\ncat > \"{}\"", clipboard_file.display())
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dummy_pbcopy_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dummy_pbcopy_path, perms).unwrap();
        }
        // Prepend the dummy pbcopy directory to the PATH.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));
        env::remove_var("DISABLE_PBCOPY");

        // Run the generate_prompt binary in singular mode.
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--singular");
        cmd.assert().success();

        // Read the content from the dummy clipboard file.
        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Assert that the prompt includes the complete function definition from complex.js,
        // including the TODO marker deep inside the nested function.
        assert!(clipboard_content.contains("complexFunction"),
                "Expected the prompt to include the function name 'complexFunction'");
        assert!(clipboard_content.contains("Entering innerMost"),
                "Expected the prompt to include 'Entering innerMost'");
        assert!(clipboard_content.contains("Exiting innerMost"),
                "Expected the prompt to include 'Exiting innerMost'");
        assert!(clipboard_content.contains("// TODO: - perform complex calculation here"),
                "Expected the prompt to include the TODO marker and its comment");
        assert!(clipboard_content.contains("End of complexFunction"),
                "Expected the prompt to include the end of the complexFunction block");
        // Ensure that the content from other.js is not included.
        assert!(!clipboard_content.contains("This is other file"),
                "Expected the prompt to not include content from other.js");
    }
    
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_js_singular_mode_kitchen_sink() {
        // Create a temporary directory for the dummy JS project.
        let temp_dir = TempDir::new().unwrap();
        let js_file_path = temp_dir.path().join("kitchen_sink.js");
        let other_js_path = temp_dir.path().join("other.js");

        // Build a large JS file by generating many simple functions
        // and one complex function with a deeply nested TODO marker.
        let mut generated_content = String::new();

        // Generate 50 simple functions (they don't contain a TODO marker).
        for i in 0..50 {
            generated_content.push_str(&format!(
                "function simpleFunc_{}() {{\n  console.log('Simple function {}');\n}}\n\n",
                i, i
            ));
        }

        // Generate a complex function that is our target.
        generated_content.push_str("function complexFunction() {\n");
        generated_content.push_str("  console.log('Start complexFunction');\n");
        generated_content.push_str("  if (true) {\n");
        generated_content.push_str("    for (let i = 0; i < 5; i++) {\n");
        generated_content.push_str("      console.log('Iteration', i);\n");
        generated_content.push_str("      (function nested() {\n");
        generated_content.push_str("        if (i === 3) {\n");
        generated_content.push_str("          function deepNested() {\n");
        generated_content.push_str("            console.log('Deep nested start');\n");
        generated_content.push_str("            // TODO: - process this complex scenario\n");
        generated_content.push_str("            console.log('Deep nested end');\n");
        generated_content.push_str("          }\n");
        generated_content.push_str("          deepNested();\n");
        generated_content.push_str("        } else {\n");
        generated_content.push_str("          console.log('No special case');\n");
        generated_content.push_str("        }\n");
        generated_content.push_str("      })();\n");
        generated_content.push_str("    }\n");
        generated_content.push_str("  }\n");
        generated_content.push_str("  console.log('End complexFunction');\n");
        generated_content.push_str("}\n\n");

        // Generate 50 more simple functions.
        for i in 50..100 {
            generated_content.push_str(&format!(
                "function simpleFunc_{}() {{\n  console.log('Simple function {}');\n}}\n\n",
                i, i
            ));
        }

        // Write the generated content to the main JS file.
        fs::write(&js_file_path, &generated_content).unwrap();

        // Create another JS file that should be ignored.
        let other_content = r#"console.log("This file should not be processed in singular mode.");"#;
        fs::write(&other_js_path, other_content).unwrap();

        // Set environment variables so that kitchen_sink.js is used as the instruction file.
        env::set_var("GET_INSTRUCTION_FILE", js_file_path.to_str().unwrap());
        env::set_var("GET_GIT_ROOT", temp_dir.path().to_str().unwrap());

        // Set up a dummy pbcopy executable that writes its stdin to a temporary clipboard file.
        let pbcopy_dir = TempDir::new().unwrap();
        let clipboard_file = pbcopy_dir.path().join("clipboard.txt");
        let dummy_pbcopy_path = pbcopy_dir.path().join("pbcopy");
        fs::write(
            &dummy_pbcopy_path,
            format!("#!/bin/sh\ncat > \"{}\"", clipboard_file.display())
        ).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dummy_pbcopy_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dummy_pbcopy_path, perms).unwrap();
        }
        // Prepend the dummy pbcopy directory to the PATH.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));
        env::remove_var("DISABLE_PBCOPY");

        // Run the generate_prompt binary in singular mode.
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--singular");
        cmd.assert().success();

        // Read the output from the dummy clipboard file.
        let output = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Assertions:
        // Verify that the complex function's block (with its nested structure and TODO marker)
        // is fully included in the final prompt.
        assert!(output.contains("complexFunction"), "Expected 'complexFunction' to be in output");
        assert!(output.contains("Start complexFunction"), "Expected start of complexFunction block");
        assert!(output.contains("Iteration"), "Expected loop iteration logs to be present");
        assert!(output.contains("Deep nested start"), "Expected nested function log to be present");
        assert!(output.contains("// TODO: - process this complex scenario"), "Expected TODO marker in output");
        assert!(output.contains("Deep nested end"), "Expected nested function log to be present");
        assert!(output.contains("End complexFunction"), "Expected end of complexFunction block");
        // Ensure that content from the other JS file is not included.
        assert!(!output.contains("This file should not be processed"), "Did not expect other.js content");
    }
    
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_js_scrubs_extra_todo_markers() {
        // Ensure diff mode is disabled so that no extra diff markers are appended.
        env::remove_var("DIFF_WITH_BRANCH");
        
        // Create a temporary directory to simulate a dummy JavaScript project.
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let temp_dir_path = temp_dir.path();
        env::set_var("GET_GIT_ROOT", temp_dir_path.to_str().unwrap());

        // Create an Instruction.js file with multiple "// TODO: -" markers.
        // The first marker is the primary (CTA) marker.
        // Extra markers in the file should be scrubbed.
        let instruction_file = temp_dir_path.join("Instruction.js");
        let instruction_content = r#"
function doSomething() {
    // some code here
}
// TODO: - Primary TODO marker
// TODO: - Extra marker that should be scrubbed
// TODO: - Another marker that should be scrubbed
"#;
        fs::write(&instruction_file, instruction_content)
            .expect("Failed to write Instruction.js");

        // Set the environment variable so generate_prompt uses this instruction file.
        env::set_var("GET_INSTRUCTION_FILE", instruction_file.to_str().unwrap());
        env::remove_var("DISABLE_PBCOPY");

        // Set up a dummy pbcopy executable to capture clipboard output.
        let pbcopy_dir = TempDir::new().expect("Failed to create dummy pbcopy directory");
        let clipboard_file = pbcopy_dir.path().join("clipboard.txt");
        let dummy_pbcopy_path = pbcopy_dir.path().join("pbcopy");
        fs::write(
            &dummy_pbcopy_path,
            format!("#!/bin/sh\ncat > \"{}\"", clipboard_file.display())
        ).expect("Failed to write dummy pbcopy script");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dummy_pbcopy_path)
                .expect("Failed to get metadata")
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dummy_pbcopy_path, perms)
                .expect("Failed to set permissions on dummy pbcopy");
        }
        // Prepend the dummy pbcopy directory to the PATH.
        let original_path = env::var("PATH").expect("PATH not found");
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        // Run generate_prompt in singular mode so that only the instruction file is processed.
        let mut cmd = Command::cargo_bin("generate_prompt")
            .expect("Failed to find generate_prompt binary");
        cmd.arg("--singular");
        cmd.assert().success();

        // Read the final prompt from the clipboard output.
        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read clipboard file");

        // Count the occurrences of marker lines.
        let marker_count = clipboard_content.matches("// TODO: -").count();
        // We expect exactly 2 markers: the primary marker from the instruction file
        // and the fixed instruction (acting as the final marker) appended by the prompt assembler.
        assert_eq!(
            marker_count, 2,
            "Expected exactly 2 TODO markers in final prompt, found {}",
            marker_count
        );

        // Verify that the primary marker appears.
        assert!(
            clipboard_content.contains("// TODO: - Primary TODO marker"),
            "Primary marker not found in final prompt"
        );
        // Verify that the fixed instruction (serving as the final marker) appears.
        assert!(
            clipboard_content.contains("Can you do the TODO:- in the above code? But ignoring all FIXMEs and other TODOs...i.e. only do the one and only one TODO that is marked by \"// TODO: - \", i.e. ignore things like \"// TODO: example\" because it doesn't have the hyphen"),
            "Fixed instruction final marker not found in final prompt"
        );
        // Ensure that extra markers have been scrubbed.
        assert!(
            !clipboard_content.contains("Extra marker that should be scrubbed"),
            "Extra marker was not scrubbed from final prompt"
        );
        assert!(
            !clipboard_content.contains("Another marker that should be scrubbed"),
            "Another extra marker was not scrubbed from final prompt"
        );
    }
}

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
}

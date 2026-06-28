// crates/generate_prompt/tests/integration_objc.rs

mod strict_end_to_end_tests {
    use assert_cmd::assert::OutputAssertExt;
    use assert_cmd::Command; // for running your binary via cargo_bin
    use assert_fs::fixture::PathChild; // explicitly bring the PathChild trait into scope
    use assert_fs::prelude::*; // for methods like child(), which requires the PathChild trait
    use predicates::boolean::PredicateBooleanExt;
    use std::process::Command as StdCommand;

    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_with_objc_input() {
        // Create a temporary directory to simulate the Git repository.
        let temp = assert_fs::TempDir::new().unwrap();

        // Create an Objective-C source file ("Example.m") with the given input.
        let example_objc = temp.child("Example.m");
        example_objc
            .write_str(
                r#"// v
// ^

- (void)exampleMethod
{
    Foo *foo = [[FooBar alloc] init];
    // TODO: - example
}
"#,
            )
            .unwrap();

        // Canonicalize the temporary directory for use as the explicit Git root.
        let canonical_git_root = temp.path().canonicalize().unwrap();

        // Optionally, initialize a Git repository.
        StdCommand::new("git")
            .current_dir(temp.path())
            .args(["init"])
            .assert()
            .success();

        // Run the generate_prompt binary from the temporary directory.
        Command::cargo_bin("generate_prompt")
            .unwrap()
            .current_dir(temp.path())
            .env("GET_GIT_ROOT", &canonical_git_root)
            .env("GET_INSTRUCTION_FILE", example_objc.path())
            .env("DISABLE_PBCOPY", "1")
            .env_remove("DIFF_WITH_BRANCH")
            .assert()
            .success()
            .stdout(
                predicates::str::contains("Success:").and(predicates::str::contains("example")),
            );

        // Cleanup
        temp.close().unwrap();
    }
}

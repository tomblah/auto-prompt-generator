// NB: these tests are currently for when we find a bug and want to reproduce it
mod strict_end_to_end_tests {
    use assert_cmd::Command;           // for running your binary via cargo_bin
    use std::env;                      // for env::set_var etc.
    use assert_fs::prelude::*;         // for methods like child(), which requires the PathChild trait
    use assert_fs::fixture::PathChild; // explicitly bring the PathChild trait into scope
    use std::process::Command as StdCommand;
    use assert_cmd::assert::OutputAssertExt;
    use predicates::boolean::PredicateBooleanExt;

    #[test]
    #[cfg(unix)]
    #[ignore]
    fn test_generate_prompt_with_objc_input() {
        // Remove any diff branch setting so we don't run diff mode.
        env::remove_var("DIFF_WITH_BRANCH");

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

        // Force generate_prompt to use this file as the instruction file.
        env::set_var("GET_INSTRUCTION_FILE", example_objc.path().to_str().unwrap());

        // Set GET_GIT_ROOT to the canonicalized temporary directory.
        let canonical_git_root = temp.path().canonicalize().unwrap();
        env::set_var("GET_GIT_ROOT", canonical_git_root.to_str().unwrap());

        // Disable clipboard copying during testing.
        env::set_var("DISABLE_PBCOPY", "1");

        // Optionally, initialize a Git repository.
        StdCommand::new("git")
            .current_dir(temp.path())
            .args(&["init"])
            .assert()
            .success();

        // Run the generate_prompt binary from the temporary directory.
        Command::cargo_bin("generate_prompt")
            .unwrap()
            .current_dir(temp.path())
            .assert()
            .success()
            .stdout(
                predicates::str::contains("Success:")
                    .and(predicates::str::contains("example")),
            );

        // Cleanup
        temp.close().unwrap();
    }
}

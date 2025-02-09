#!/usr/bin/env bats

setup() {
  TMP_DIR=$(mktemp -d)
}

teardown() {
  rm -rf "$TMP_DIR"
}

# Load the get-git-root module.
load "${BATS_TEST_DIRNAME}/get-git-root.sh"

@test "returns Git root when inside a Git repository" {
  # Create a temporary Git repository.
  pushd "$TMP_DIR" > /dev/null
  git init -q

  run get-git-root
  [ "$status" -eq 0 ]
  
  # Compare with the physical current directory using pwd -P to account for symlink resolution.
  [ "$output" = "$(pwd -P)" ]
  popd > /dev/null
}

@test "returns error when not in a Git repository" {
  # Run the function from a directory that is not a Git repo.
  pushd "$TMP_DIR" > /dev/null
  run get-git-root
  [ "$status" -ne 0 ]
  [[ "$output" == *"Error: Not a git repository."* ]]
  popd > /dev/null
}

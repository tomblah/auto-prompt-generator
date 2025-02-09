#!/usr/bin/env bats
# test_filter-files-singular.bats
#
# These tests verify that the filter_files_singular function
# correctly returns a temporary file containing only the provided
# TODO file path, and that when the script is executed directly,
# it enforces the correct usage.

setup() {
  # Create a temporary directory for any auxiliary files.
  TMP_DIR=$(mktemp -d)
}

teardown() {
  rm -rf "$TMP_DIR"
}

#------------------------------------------------------------------------------
# Test when the script is sourced and the function is called.
#------------------------------------------------------------------------------
load "${BATS_TEST_DIRNAME}/filter-files-singular.sh"

@test "filter_files_singular returns a temp file containing the provided todo file path" {
  todo_file="/path/to/TODO.swift"
  # Call the function and capture its output (which is the temp file path).
  temp_file="$(filter_files_singular "$todo_file")"

  # Check that the returned file exists.
  [ -f "$temp_file" ]

  # Verify that the content of the temporary file is exactly the todo file path.
  content="$(cat "$temp_file")"
  [ "$content" = "$todo_file" ]
}

#------------------------------------------------------------------------------
# Test the direct execution behavior of the script.
# (In direct execution mode, the script checks the argument count.)
#------------------------------------------------------------------------------
@test "direct execution with no arguments prints usage and exits with non-zero" {
  run bash "${BATS_TEST_DIRNAME}/filter-files-singular.sh"
  [ "$status" -ne 0 ]
  [[ "$output" == *"Usage:"* ]]
}

@test "direct execution with multiple arguments prints usage and exits with non-zero" {
  run bash "${BATS_TEST_DIRNAME}/filter-files-singular.sh" "arg1" "arg2"
  [ "$status" -ne 0 ]
  [[ "$output" == *"Usage:"* ]]
}

@test "direct execution with one argument outputs temp file containing the provided path" {
  todo_file="/path/to/TODO.swift"
  run bash "${BATS_TEST_DIRNAME}/filter-files-singular.sh" "$todo_file"
  [ "$status" -eq 0 ]

  # The output should be the path to the temporary file.
  temp_file="$output"
  [ -f "$temp_file" ]

  # Verify that the file content matches the provided todo file path.
  content="$(cat "$temp_file")"
  [ "$content" = "$todo_file" ]
}

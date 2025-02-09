#!/usr/bin/env bats
# test_generate-prompt.bats
#
# These tests run the main generate-prompt.sh script in a simulated Git repository.
# They verify that (a) when a valid TODO instruction exists, the prompt is assembled
# (and “copied” to our dummy clipboard file), (b) that the script fails when no valid
# TODO instruction is present, and (c) that the --slim and --exclude options work as expected.

setup() {
  # Create a temporary directory that will serve as our fake repository.
  TMP_DIR=$(mktemp -d)
  
  # Create a dummy "pbcopy" executable so that our script does not touch the real clipboard.
  mkdir -p "$TMP_DIR/dummybin"
  cat << 'EOF' > "$TMP_DIR/dummybin/pbcopy"
#!/bin/bash
# Write the clipboard content to a file named "clipboard.txt" in the current directory.
cat > clipboard.txt
EOF
  chmod +x "$TMP_DIR/dummybin/pbcopy"
  # Prepend dummybin to PATH so that pbcopy is overridden.
  export PATH="$TMP_DIR/dummybin:$PATH"
  
  # Copy the main script and all its dependency components to TMP_DIR.
  # (This assumes your test files and these scripts are in the same directory;
  # adjust the source paths if necessary.)
  cp "${BATS_TEST_DIRNAME}/generate-prompt.sh" "$TMP_DIR/"
  cp "${BATS_TEST_DIRNAME}/find-prompt-instruction.sh" "$TMP_DIR/"
  cp "${BATS_TEST_DIRNAME}/extract-instruction-content.sh" "$TMP_DIR/"
  cp "${BATS_TEST_DIRNAME}/extract-types.sh" "$TMP_DIR/"
  cp "${BATS_TEST_DIRNAME}/find-definition-files.sh" "$TMP_DIR/"
  cp "${BATS_TEST_DIRNAME}/filter-files.sh" "$TMP_DIR/"
  cp "${BATS_TEST_DIRNAME}/exclude-files.sh" "$TMP_DIR/"
  cp "${BATS_TEST_DIRNAME}/assemble-prompt.sh" "$TMP_DIR/"
  cp "${BATS_TEST_DIRNAME}/get-git-root.sh" "$TMP_DIR/"
  cp "${BATS_TEST_DIRNAME}/get-package-root.sh" "$TMP_DIR/"
  cp "${BATS_TEST_DIRNAME}/get-search-roots.sh" "$TMP_DIR/"

  # Change to TMP_DIR (this will become our repository root).
  cd "$TMP_DIR"

  # Initialize a Git repository.
  git init -q .

  # Create a Swift file with a valid TODO instruction.
  cat << 'EOF' > Test.swift
import Foundation
// TODO: - Test instruction for prompt
class TestClass {}
EOF

  # (Optionally, create an extra Swift file that defines the same type or another type.)
  cat << 'EOF' > Another.swift
struct AnotherStruct {}
EOF
}

teardown() {
  rm -rf "$TMP_DIR"
}

@test "generate-prompt.sh outputs success message and assembles prompt with fixed instruction" {
  # Run the main script.
  run bash generate-prompt.sh
  [ "$status" -eq 0 ]

  # Check that the output includes a success message and the fixed instruction.
  [[ "$output" == *"Success:"* ]]
  [[ "$output" == *"Can you do the TODO:- in the above code?"* ]]
  
  # Check that our dummy pbcopy created a clipboard file and that it contains prompt details.
  [ -f "clipboard.txt" ]
  clipboard_content=$(cat clipboard.txt)
  [[ "$clipboard_content" == *"The contents of Test.swift is as follows:"* ]]
  [[ "$clipboard_content" == *"TestClass"* ]]
}

@test "generate-prompt.sh fails when no valid TODO instruction is present" {
  # Remove the valid TODO instruction from Test.swift.
  cat << 'EOF' > Test.swift
import Foundation
class TestClass {}
EOF

  run bash generate-prompt.sh
  [ "$status" -ne 0 ]
  [[ "$output" == *"Error:"* ]]
}

@test "generate-prompt.sh slim mode excludes disallowed files" {
  # Create an extra file that should be filtered out in slim mode.
  cat << 'EOF' > ViewController.swift
import UIKit
class ViewController {}
EOF

  # Run the script with the --slim flag.
  run bash generate-prompt.sh --slim
  [ "$status" -eq 0 ]
  
  # The section showing the final list of files should not list ViewController.swift.
  [[ "$output" != *"ViewController.swift"* ]]
  [[ "$output" == *"Success:"* ]]
}

@test "generate-prompt.sh excludes files specified with --exclude" {
  # Create an extra file to be excluded.
  cat << 'EOF' > ExcludeMe.swift
import Foundation
class ExcludeMe {}
EOF

  # Run the script with --exclude option.
  run bash generate-prompt.sh --exclude ExcludeMe.swift
  [ "$status" -eq 0 ]
  
  # Debugging output: print the complete output for inspection.
  echo "DEBUG OUTPUT:"
  echo "$output"
  
  # Extract the final list of files from the output.
  # This extracts the lines between "Files (final list):" and the next separator line.
  final_list=$(echo "$output" | awk '/Files \(final list\):/{flag=1; next} /--------------------------------------------------/{flag=0} flag')
  echo "DEBUG: Final list of files:" "$final_list" >&2
  
  # Verify that the final list of files does not include ExcludeMe.swift.
  [[ "$final_list" != *"ExcludeMe.swift"* ]]
}

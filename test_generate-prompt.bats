#!/usr/bin/env bats
# test_generate-prompt.bats
#
# These tests run the main generate-prompt.sh script in a simulated Git repository.
# They verify that (a) when a valid TODO instruction exists, the prompt is assembled
# (and “copied” to our dummy clipboard file), (b) that the script fails when no valid
# TODO instruction is present, (c) that the --slim and --exclude options work as expected,
# and (d) that the --singular option causes only the TODO file to be included.
 
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
  cp "${BATS_TEST_DIRNAME}/filter-files-singular.sh" "$TMP_DIR/"
 
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
 
  # Create an extra Swift file that would normally be discovered for type definitions.
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
 
# --- New tests for singular mode ---
 
@test "generate-prompt.sh singular mode includes only the TODO file" {
  # Create an additional extra file that would normally be processed.
  cat << 'EOF' > Extra.swift
import Foundation
struct ExtraStruct {}
EOF
 
  # Run the script with the --singular flag.
  run bash generate-prompt.sh --singular
  [ "$status" -eq 0 ]
 
  # Check that the output indicates singular mode.
  [[ "$output" == *"Singular mode enabled: only including the TODO file"* ]]
 
  # Extract the final list of file basenames.
  final_list=$(echo "$output" | awk '/Files \(final list\):/{flag=1; next} /--------------------------------------------------/{flag=0} flag' | tr -d '\r')
 
  # In singular mode, only the TODO file (Test.swift) should be listed.
  [ "$final_list" = "Test.swift" ]
 
  # Verify that the clipboard content (from dummy pbcopy) includes only Test.swift.
  [ -f "clipboard.txt" ]
  clipboard_content=$(cat clipboard.txt)
  [[ "$clipboard_content" == *"The contents of Test.swift is as follows:"* ]]
  [[ "$clipboard_content" != *"Another.swift"* ]]
  [[ "$clipboard_content" != *"Extra.swift"* ]]
}
 
@test "generate-prompt.sh singular mode ignores non-TODO files even when present" {
  # Create another extra Swift file that would normally be considered.
  cat << 'EOF' > IgnoreMe.swift
import Foundation
class IgnoreMe {}
EOF
 
  # Run the script with the --singular flag.
  run bash generate-prompt.sh --singular
  [ "$status" -eq 0 ]
 
  # Verify that the final file list printed includes only Test.swift.
  final_list=$(echo "$output" | awk '/Files \(final list\):/{flag=1; next} /--------------------------------------------------/{flag=0} flag' | tr -d '\r')
  [ "$final_list" = "Test.swift" ]
 
  # Also check that the assembled prompt in clipboard.txt does not mention IgnoreMe.swift.
  [ -f "clipboard.txt" ]
  clipboard_content=$(cat clipboard.txt)
  [[ "$clipboard_content" == *"Test.swift"* ]]
  [[ "$clipboard_content" != *"IgnoreMe.swift"* ]]
}

@test "generate-prompt.sh does not include Swift files from .build directories" {
  # Create a Swift file inside a .build directory that should be ignored.
  mkdir -p ".build/ThirdParty"
  cat << 'EOF' > ".build/ThirdParty/ThirdParty.swift"
import Foundation
class ThirdPartyClass {}
EOF

  # Also create a normal Swift file to be processed.
  cat << 'EOF' > Normal.swift
import Foundation
class NormalClass {}
EOF

  # Ensure Test.swift (with the valid TODO instruction) is reset.
  cat << 'EOF' > Test.swift
import Foundation
// TODO: - Test instruction for prompt
class TestClass {}
EOF

  # Run the main script.
  run bash generate-prompt.sh
  [ "$status" -eq 0 ]

  # Extract the final list of files from the output.
  final_list=$(echo "$output" | awk '/Files \(final list\):/{flag=1; next} /--------------------------------------------------/{flag=0} flag')
  
  # Assert that the list includes Normal.swift and does not include ThirdParty.swift.
  [[ "$final_list" == *"Normal.swift"* ]]
  [[ "$final_list" != *"ThirdParty.swift"* ]]

  # Also check that the assembled prompt (in clipboard.txt) does not include ThirdParty.swift.
  clipboard_content=$(cat clipboard.txt)
  [[ "$clipboard_content" == *"Normal.swift"* ]]
  [[ "$clipboard_content" != *"ThirdParty.swift"* ]]
}

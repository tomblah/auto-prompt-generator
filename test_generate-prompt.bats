#!/usr/bin/env bats
# test_generate-prompt.bats
#
# These tests run the main generate-prompt.sh script in a simulated Git repository.
# They verify that:
#   (a) when a valid TODO instruction exists, the prompt is assembled
#       (and “copied” to our dummy clipboard file),
#   (b) the script fails when no valid TODO instruction is present,
#   (c) the --slim and --exclude options work as expected,
#   (d) the --singular option causes only the TODO file to be included,
#   (e) the new --force-global option causes the script to ignore package boundaries,
#   (f) the normal (non-singular) mode includes both the TODO file and type definitions,
#   (g) and when multiple TODO files exist the most-recently modified one is chosen.
 
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
 
  # Create a Swift file that defines a type referenced by Test.swift.
  cat << 'EOF' > Another.swift
import Foundation
struct AnotherStruct {}
EOF
}
 
teardown() {
  rm -rf "$TMP_DIR"
}
 
@test "generate-prompt.sh outputs success message and assembles prompt with fixed instruction" {
  run bash generate-prompt.sh
  [ "$status" -eq 0 ]
 
  [[ "$output" == *"Success:"* ]]
  [[ "$output" == *"Can you do the TODO:- in the above code?"* ]]
 
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
 
  run bash generate-prompt.sh --slim
  [ "$status" -eq 0 ]
 
  # The final file list should not include ViewController.swift.
  [[ "$output" != *"ViewController.swift"* ]]
  [[ "$output" == *"Success:"* ]]
}
 
@test "generate-prompt.sh excludes files specified with --exclude" {
  # Create an extra file to be excluded.
  cat << 'EOF' > ExcludeMe.swift
import Foundation
class ExcludeMe {}
EOF
 
  run bash generate-prompt.sh --exclude ExcludeMe.swift
  [ "$status" -eq 0 ]
 
  # Extract the final list of files from the output.
  final_list=$(echo "$output" | awk '/Files \(final list\):/{flag=1; next} /--------------------------------------------------/{flag=0} flag')
  # Verify that the final list does not include ExcludeMe.swift.
  [[ "$final_list" != *"ExcludeMe.swift"* ]]
}
 
@test "generate-prompt.sh singular mode includes only the TODO file" {
  # Create an additional file that would normally be processed.
  cat << 'EOF' > Extra.swift
import Foundation
struct ExtraStruct {}
EOF
 
  run bash generate-prompt.sh --singular
  [ "$status" -eq 0 ]
 
  [[ "$output" == *"Singular mode enabled: only including the TODO file"* ]]
 
  final_list=$(echo "$output" | awk '/Files \(final list\):/{flag=1; next} /--------------------------------------------------/{flag=0} flag' | tr -d '\r')
  [ "$final_list" = "Test.swift" ]
 
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
 
  run bash generate-prompt.sh --singular
  [ "$status" -eq 0 ]
 
  final_list=$(echo "$output" | awk '/Files \(final list\):/{flag=1; next} /--------------------------------------------------/{flag=0} flag' | tr -d '\r')
  [ "$final_list" = "Test.swift" ]
 
  [ -f "clipboard.txt" ]
  clipboard_content=$(cat clipboard.txt)
  [[ "$clipboard_content" == *"Test.swift"* ]]
  [[ "$clipboard_content" != *"IgnoreMe.swift"* ]]
}
 
@test "generate-prompt.sh does not include Swift files from .build directories" {
  mkdir -p ".build/ThirdParty"
  cat << 'EOF' > ".build/ThirdParty/ThirdParty.swift"
import Foundation
class ThirdPartyClass {}
EOF

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

  run bash generate-prompt.sh
  [ "$status" -eq 0 ]

  final_list=$(echo "$output" | awk '/Files \(final list\):/{flag=1; next} /--------------------------------------------------/{flag=0} flag')
  
  [[ "$final_list" == *"Normal.swift"* ]]
  [[ "$final_list" != *"ThirdParty.swift"* ]]

  clipboard_content=$(cat clipboard.txt)
  [[ "$clipboard_content" == *"Normal.swift"* ]]
  [[ "$clipboard_content" != *"ThirdParty.swift"* ]]
}
 
@test "generate-prompt.sh does not include Swift files from Pods directories" {
  mkdir -p "Pods/SubDir"
  cat << 'EOF' > "Pods/SubDir/PodsFile.swift"
import Foundation
class PodsClass {}
EOF

  cat << 'EOF' > Normal.swift
import Foundation
class NormalClass {}
EOF

  cat << 'EOF' > Test.swift
import Foundation
// TODO: - Test instruction for prompt
class TestClass {}
EOF

  run bash generate-prompt.sh
  [ "$status" -eq 0 ]

  final_list=$(echo "$output" | awk '/Files \(final list\):/{flag=1; next} /--------------------------------------------------/{flag=0} flag')
  
  [[ "$final_list" == *"Normal.swift"* ]]
  [[ "$final_list" != *"PodsFile.swift"* ]]

  clipboard_content=$(cat clipboard.txt)
  [[ "$clipboard_content" == *"Normal.swift"* ]]
  [[ "$clipboard_content" != *"PodsFile.swift"* ]]
}
 
@test "generate-prompt.sh uses package root when available without --force-global" {
  mkdir -p "PackageDir"
  cat << 'EOF' > PackageDir/Package.swift
// Package.swift content
EOF
  mv Test.swift PackageDir/Test.swift

  run bash generate-prompt.sh
  [ "$status" -eq 0 ]
  [[ "$output" == *"Found package root:"* ]]
  [[ "$output" == *"PackageDir"* ]]
}
 
@test "generate-prompt.sh with --force-global ignores package boundaries" {
  mkdir -p "PackageDir"
  cat << 'EOF' > PackageDir/Package.swift
// Package.swift content
EOF
  mv Test.swift PackageDir/Test.swift

  run bash generate-prompt.sh --force-global
  [ "$status" -eq 0 ]
  [[ "$output" == *"Force global enabled: ignoring package boundaries and using Git root for context."* ]]
  [[ "$output" != *"Found package root:"* ]]
}
 
# --- Additional tests to increase coverage ---
 
@test "generate-prompt.sh normal mode includes both TODO file and type definition files" {
  # In normal mode (without --singular), both the TODO file and files
  # containing type definitions should be processed.
  run bash generate-prompt.sh
  [ "$status" -eq 0 ]
 
  final_list=$(echo "$output" | awk '/Files \(final list\):/{flag=1; next} /--------------------------------------------------/{flag=0} flag')
  # Expect both Test.swift (the file with the TODO) and Another.swift (for the type definition)
  [[ "$final_list" == *"Test.swift"* ]]
  [[ "$final_list" == *"Another.swift"* ]]
}
 
@test "generate-prompt.sh chooses the most recently modified TODO file when multiple exist" {
  # Create a second file with a valid TODO instruction.
  cat << 'EOF' > SecondTest.swift
import Foundation
// TODO: - Second test instruction for prompt
class SecondTestClass {}
EOF
  # Ensure SecondTest.swift is more recent than Test.swift.
  sleep 1
  touch SecondTest.swift
 
  run bash generate-prompt.sh
  [ "$status" -eq 0 ]
 
  # The script should report that it found exactly one instruction in the most recent file.
  [[ "$output" == *"Found exactly one instruction in SecondTest.swift"* ]]
  # And it should log that the other TODO file (Test.swift) was ignored.
  [[ "$output" == *"Ignored file:"* ]]
  [[ "$output" == *"Test.swift"* ]]
}

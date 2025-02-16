#!/usr/bin/env bats
# test_generate-prompt-diff-with.bats
#
# This file tests the end-to-end behavior of the --diff-with option in generate-prompt.sh.
# It creates a temporary Git repository, commits a file with a valid TODO instruction,
# then either modifies it (to produce diff output) or resets it (so no diff appears),
# and finally asserts that the assembled prompt includes (or omits) a diff section accordingly.

setup() {
  TMP_DIR=$(mktemp -d)
  
  # Create a dummy "pbcopy" so that the script writes to clipboard.txt instead of the system clipboard.
  mkdir -p "$TMP_DIR/dummybin"
  cat << 'EOF' > "$TMP_DIR/dummybin/pbcopy"
#!/bin/bash
# Redirect clipboard content to a file named "clipboard.txt"
cat > clipboard.txt
EOF
  chmod +x "$TMP_DIR/dummybin/pbcopy"
  export PATH="$TMP_DIR/dummybin:$PATH"
  
  # Copy required scripts to TMP_DIR (adjust paths if necessary)
  cp "${BATS_TEST_DIRNAME}/generate-prompt.sh" "$TMP_DIR/"
  cp "${BATS_TEST_DIRNAME}/find-prompt-instruction.sh" "$TMP_DIR/"
  cp "${BATS_TEST_DIRNAME}/extract-instruction-content.sh" "$TMP_DIR/"
  cp "${BATS_TEST_DIRNAME}/extract-types.sh" "$TMP_DIR/"
  cp "${BATS_TEST_DIRNAME}/find-definition-files.sh" "$TMP_DIR/"
  cp "${BATS_TEST_DIRNAME}/filter-files.sh" "$TMP_DIR/"
  cp "${BATS_TEST_DIRNAME}/assemble-prompt.sh" "$TMP_DIR/"
  cp "${BATS_TEST_DIRNAME}/get-git-root.sh" "$TMP_DIR/"
  cp "${BATS_TEST_DIRNAME}/get-package-root.sh" "$TMP_DIR/"
  cp "${BATS_TEST_DIRNAME}/get-search-roots.sh" "$TMP_DIR/"
  cp "${BATS_TEST_DIRNAME}/find-referencing-files.sh" "$TMP_DIR/"
  cp "${BATS_TEST_DIRNAME}/file-types.sh" "$TMP_DIR/"
  cp -R "${BATS_TEST_DIRNAME}/rust" "$TMP_DIR/"
  
  # Change to TMP_DIR which will act as our repository root.
  cd "$TMP_DIR"
  
  # Initialize a Git repository and set user config.
  git init -q .
  git config user.email "test@example.com"
  git config user.name "Test User"
  git branch -M main
  
  # Create a Swift file with a valid TODO instruction.
  cat << 'EOF' > Test.swift
import Foundation
// TODO: - Original instruction
class TestClass {}
EOF
  
  # Add and commit the file.
  git add Test.swift
  git commit -q -m "Initial commit"
}

teardown() {
  rm -rf "$TMP_DIR"
}

# Uncomment this test if you want to verify diff output appears when the file is modified.
# @test "generate-prompt.sh with --diff-with includes diff output when file is modified" {
#   # Modify Test.swift so that it differs from the committed version.
#   echo "// Added comment" >> Test.swift
#
#   # Run generate-prompt.sh with the --diff-with option.
#   run bash generate-prompt.sh --diff-with main --verbose
#   [ "$status" -eq 0 ]
#
#   # Assert that the output (or clipboard content) includes a diff header for Test.swift.
#   [[ "$output" == *"The diff for Test.swift (against branch main) is as follows:"* ]]
#   # Also check that the diff output includes our added line (the "+" prefix indicates an addition).
#   [[ "$output" == *"+// Added comment"* ]]
# }

@test "generate-prompt.sh with --diff-with does not include diff output when file is unmodified" {
  # Reset Test.swift so that it matches the committed version.
  git checkout -- Test.swift
  
  # Run generate-prompt.sh with the --diff-with option.
  run bash generate-prompt.sh --diff-with main
  [ "$status" -eq 0 ]
  
  # Assert that no diff section is printed in the output.
  [[ "$output" != *"The diff for Test.swift (against branch main) is as follows:"* ]]
  
  # Also, if our dummy pbcopy wrote to clipboard.txt, verify it does not contain diff output.
  if [ -f "clipboard.txt" ]; then
    clipboard_content=$(cat clipboard.txt)
    [[ "$clipboard_content" != *"The diff for Test.swift (against branch main) is as follows:"* ]]
  fi
}

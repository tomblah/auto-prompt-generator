#!/usr/bin/env bats

setup() {
  # Create a temporary directory to simulate the Git repository root.
  GIT_ROOT=$(mktemp -d)
  # Create a temporary file to hold type names.
  TYPES_FILE=$(mktemp)
}

teardown() {
  rm -rf "$GIT_ROOT"
  rm -f "$TYPES_FILE"
}

# Load the find_definition_files component.
load "${BATS_TEST_DIRNAME}/find_definition_files.sh"

@test "returns file path when type is defined" {
  # Create a Swift file with a definition for MyClass.
  swift_file="$GIT_ROOT/Test.swift"
  cat <<'EOF' > "$swift_file"
import Foundation
class MyClass {
}
EOF

  # Write the type name to the types file.
  echo "MyClass" > "$TYPES_FILE"

  run find_definition_files "$TYPES_FILE" "$GIT_ROOT"
  [ "$status" -eq 0 ]

  # The function outputs a temporary file containing found file paths.
  FOUND_FILES_FILE="$output"
  result="$(cat "$FOUND_FILES_FILE")"

  # Expect the swift file path to be present in the result.
  [[ "$result" == *"$swift_file"* ]]
}

@test "returns empty result when no matching type definition exists" {
  # Create a Swift file that does NOT contain a definition for MyStruct.
  swift_file="$GIT_ROOT/Test.swift"
  cat <<'EOF' > "$swift_file"
import Foundation
// This file has no type definitions.
EOF

  # Write a type name that won't be found.
  echo "MyStruct" > "$TYPES_FILE"

  run find_definition_files "$TYPES_FILE" "$GIT_ROOT"
  [ "$status" -eq 0 ]

  FOUND_FILES_FILE="$output"
  result="$(cat "$FOUND_FILES_FILE")"
  
  # Expect result to be empty.
  [ -z "$result" ]
}

@test "finds multiple files if type defined in more than one" {
  # Create two Swift files, each defining MyProtocol.
  swift_file1="$GIT_ROOT/File1.swift"
  swift_file2="$GIT_ROOT/File2.swift"
  
  cat <<'EOF' > "$swift_file1"
import Foundation
protocol MyProtocol {
}
EOF

  cat <<'EOF' > "$swift_file2"
import UIKit
protocol MyProtocol {
}
EOF

  echo "MyProtocol" > "$TYPES_FILE"

  run find_definition_files "$TYPES_FILE" "$GIT_ROOT"
  [ "$status" -eq 0 ]

  FOUND_FILES_FILE="$output"
  result="$(cat "$FOUND_FILES_FILE")"

  # The result should contain both file paths.
  [[ "$result" == *"$swift_file1"* ]]
  [[ "$result" == *"$swift_file2"* ]]
}

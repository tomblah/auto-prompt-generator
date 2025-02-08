#!/usr/bin/env bats

setup() {
  # Create a temporary directory for each test.
  TEST_DIR=$(mktemp -d)
}

teardown() {
  # Remove the temporary directory and all its contents.
  rm -rf "$TEST_DIR"
}

# Load the extract_types component. Adjust the path if necessary.
load "${BATS_TEST_DIRNAME}/extract_types.sh"

@test "extract_types returns empty for file with no capitalized words" {
  swift_file="$TEST_DIR/empty.swift"
  cat <<'EOF' > "$swift_file"
import foundation
let x = 5
EOF

  run extract_types "$swift_file"
  [ "$status" -eq 0 ]
  
  # The function outputs the name of a temporary file containing the types.
  types_file="$output"
  result="$(cat "$types_file")"
  
  # Expect no types to be found.
  [ -z "$result" ]
}

@test "extract_types extracts capitalized words from a Swift file" {
  swift_file="$TEST_DIR/test.swift"
  cat <<'EOF' > "$swift_file"
import Foundation
class MyClass {
}
struct MyStruct {
}
enum MyEnum {
}
EOF

  run extract_types "$swift_file"
  [ "$status" -eq 0 ]
  
  types_file="$output"
  result="$(cat "$types_file")"
  
  # The expected result is that MyClass, MyStruct, and MyEnum are extracted.
  # Note: The AWK pipeline sorts them alphabetically.
  expected="MyClass
MyEnum
MyStruct"
  
  [ "$result" = "$expected" ]
}

@test "extract_types extracts type names from bracket notation" {
  swift_file="$TEST_DIR/test_bracket.swift"
  cat <<'EOF' > "$swift_file"
import UIKit
let array: [CustomType] = []
EOF

  run extract_types "$swift_file"
  [ "$status" -eq 0 ]
  
  types_file="$output"
  result="$(cat "$types_file")"
  
  # Expect the type name extracted from the bracket notation.
  [ "$result" = "CustomType" ]
}

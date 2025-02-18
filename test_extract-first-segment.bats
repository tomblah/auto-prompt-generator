#!/usr/bin/env bats

# Test that sourcing the helper does not trigger its direct‐execution block.
setup() {
  # Source the helper file from the same directory as this test file.
  source "$BATS_TEST_DIRNAME/extract-first-segment.sh"
}

@test "Extracts first segment from HandleView.swift" {
  run extract_first_segment "HandleView.swift"
  [ "$status" -eq 0 ]
  [ "$output" = "Handle" ]
}

@test "Extracts first segment from AppDelegate.swift (returns 'App')" {
  run extract_first_segment "AppDelegate.swift"
  [ "$status" -eq 0 ]
  [ "$output" = "App" ]
}

@test "Returns full basename when pattern does not match (MYFILE.swift)" {
  run extract_first_segment "MYFILE.swift"
  [ "$status" -eq 0 ]
  [ "$output" = "MYFILE" ]
}

@test "Extracts first segment from SomeFile.txt" {
  run extract_first_segment "SomeFile.txt"
  [ "$status" -eq 0 ]
  [ "$output" = "Some" ]
}

@test "Works with a filename with no extension (File)" {
  run extract_first_segment "File"
  [ "$status" -eq 0 ]
  [ "$output" = "File" ]
}

@test "Returns full basename when first letter is not capitalized (xFile.swift)" {
  run extract_first_segment "xFile.swift"
  [ "$status" -eq 0 ]
  [ "$output" = "xFile" ]
}

# Also test the direct execution usage message when no argument is provided.
@test "Direct execution without argument returns usage error" {
  run bash "$BATS_TEST_DIRNAME/extract-first-segment.sh"
  [ "$status" -eq 1 ]
  [[ "$output" == *"Usage:"* ]]
}

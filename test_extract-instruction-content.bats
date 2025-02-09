#!/usr/bin/env bats

setup() {
  TMP_DIR=$(mktemp -d)
}

teardown() {
  rm -rf "$TMP_DIR"
}

# Load the extract-instruction-content module.
load "${BATS_TEST_DIRNAME}/extract-instruction-content.sh"

@test "returns error when no valid TODO instruction is found" {
  swift_file="$TMP_DIR/no_todo.swift"
  cat <<'EOF' > "$swift_file"
// This file contains no proper TODO instruction.
func doSomething() {
    print("Hello")
}
EOF

  run extract-instruction-content "$swift_file"
  [ "$status" -ne 0 ]
  [[ "$output" == *"Error: No valid TODO instruction found"* ]]
}

@test "extracts instruction with '// TODO: - ' correctly" {
  swift_file="$TMP_DIR/todo_dash.swift"
  cat <<'EOF' > "$swift_file"
import Foundation
// TODO: - Implement the new feature
class FeatureClass {}
EOF

  run extract-instruction-content "$swift_file"
  [ "$status" -eq 0 ]
  expected="// TODO: - Implement the new feature"
  [ "$output" = "$expected" ]
}

@test "extracts instruction with '// TODO: ChatGPT: ' correctly" {
  swift_file="$TMP_DIR/todo_chatgpt.swift"
  cat <<'EOF' > "$swift_file"
import Foundation
// TODO: ChatGPT: Resolve the error handling
func doWork() {}
EOF

  run extract-instruction-content "$swift_file"
  [ "$status" -eq 0 ]
  expected="// TODO: ChatGPT: Resolve the error handling"
  [ "$output" = "$expected" ]
}

@test "returns the first matching instruction when multiple are present" {
  swift_file="$TMP_DIR/multiple_todos.swift"
  cat <<'EOF' > "$swift_file"
import Foundation
// TODO: - First instruction should be picked
// TODO: ChatGPT: Second instruction that should be ignored
func anotherFunc() {}
EOF

  run extract-instruction-content "$swift_file"
  [ "$status" -eq 0 ]
  expected="// TODO: - First instruction should be picked"
  [ "$output" = "$expected" ]
}

#!/usr/bin/env bats

setup() {
  TMP_DIR=$(mktemp -d)
}

teardown() {
  rm -rf "$TMP_DIR"
}

@test "extract-enclosing-type returns the last type before the TODO line" {
  file="$TMP_DIR/test.swift"
  cat <<EOF > "$file"
class FirstClass {}
struct SecondStruct {}
// TODO: - Implement feature
EOF

  run bash -c "source ./extract-enclosing-type.sh; extract_enclosing_type \"$file\""
  [ "$status" -eq 0 ]
  # Since the function scans until it reaches the TODO line, it should output the last type seen before that line.
  [ "$output" = "SecondStruct" ]
}

@test "extract-enclosing-type returns the type when only one is defined" {
  file="$TMP_DIR/test.swift"
  cat <<EOF > "$file"
enum OnlyEnum {}
// TODO: - Fix bug
EOF

  run bash -c "source ./extract-enclosing-type.sh; extract_enclosing_type \"$file\""
  [ "$status" -eq 0 ]
  [ "$output" = "OnlyEnum" ]
}

@test "extract-enclosing-type outputs nothing if no type is defined before the TODO" {
  file="$TMP_DIR/test.swift"
  cat <<EOF > "$file"
// A comment line
// TODO: - Nothing to implement
EOF

  run bash -c "source ./extract-enclosing-type.sh; extract_enclosing_type \"$file\""
  [ "$status" -eq 0 ]
  # Expect no output because no type was defined before the TODO.
  [ -z "$output" ]
}

@test "extract-enclosing-type ignores any types defined after the TODO" {
  file="$TMP_DIR/test.swift"
  cat <<EOF > "$file"
class Before {}
// TODO: - Do something
struct After {}
EOF

  run bash -c "source ./extract-enclosing-type.sh; extract_enclosing_type \"$file\""
  [ "$status" -eq 0 ]
  # Only the type defined before the TODO should be returned.
  [ "$output" = "Before" ]
}

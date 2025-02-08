#!/usr/bin/env bats

setup() {
  TMP_DIR=$(mktemp -d)
}

teardown() {
  rm -rf "$TMP_DIR"
}

# Load the get_package_root component.
# Adjust the path if your test file is in a different directory.
load "${BATS_TEST_DIRNAME}/get_package_root.sh"

@test "returns package root when Package.swift is in the same directory as the file" {
  # Create a package directory with a Package.swift and a Swift file.
  mkdir -p "$TMP_DIR/package"
  echo "swift package" > "$TMP_DIR/package/Package.swift"
  echo "public struct Dummy {}" > "$TMP_DIR/package/SomeFile.swift"

  file_path="$TMP_DIR/package/SomeFile.swift"

  run get_package_root "$file_path"
  [ "$status" -eq 0 ]
  [ "$output" = "$TMP_DIR/package" ]
}

@test "returns package root when Package.swift is in an ancestor directory" {
  # Create a nested directory structure.
  mkdir -p "$TMP_DIR/package/subdir"
  echo "swift package" > "$TMP_DIR/package/Package.swift"
  echo "public class Dummy {}" > "$TMP_DIR/package/subdir/File.swift"

  file_path="$TMP_DIR/package/subdir/File.swift"

  run get_package_root "$file_path"
  [ "$status" -eq 0 ]
  [ "$output" = "$TMP_DIR/package" ]
}

@test "returns non-zero exit and no output when no Package.swift is found" {
  # Create a directory structure without a Package.swift.
  mkdir -p "$TMP_DIR/nopackage"
  echo "public enum Dummy {}" > "$TMP_DIR/nopackage/File.swift"

  file_path="$TMP_DIR/nopackage/File.swift"

  run get_package_root "$file_path"
  [ "$status" -ne 0 ]
  [ -z "$output" ]
}

@test "handles a file in a directory with no Package.swift (root-level file)" {
  # Create a standalone Swift file without any Package.swift in TMP_DIR.
  echo "public protocol Dummy {}" > "$TMP_DIR/Standalone.swift"
  file_path="$TMP_DIR/Standalone.swift"

  run get_package_root "$file_path"
  [ "$status" -ne 0 ]
  [ -z "$output" ]
}

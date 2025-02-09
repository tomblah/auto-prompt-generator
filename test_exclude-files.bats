#!/usr/bin/env bats

setup() {
  TMP_DIR=$(mktemp -d)
}

teardown() {
  rm -rf "$TMP_DIR"
}

# Load the exclude-files component.
load "${BATS_TEST_DIRNAME}/exclude-files.sh"

@test "filter_excluded_files returns original list when no exclusions provided" {
  # Create a temporary file simulating a list of found files.
  found_files=$(mktemp)
  echo "/path/to/FileA.swift" >> "$found_files"
  echo "/path/to/FileB.swift" >> "$found_files"

  # Call function with no extra exclusion arguments.
  filtered_file=$(filter_excluded_files "$found_files")
  result="$(cat "$filtered_file")"

  expected="/path/to/FileA.swift
/path/to/FileB.swift"

  [ "$result" = "$expected" ]

  rm -f "$found_files" "$filtered_file"
}

@test "filter_excluded_files removes matching file" {
  found_files=$(mktemp)
  echo "/path/to/FileA.swift" >> "$found_files"
  echo "/path/to/Unwanted.swift" >> "$found_files"
  echo "/path/to/FileB.swift" >> "$found_files"

  filtered_file=$(filter_excluded_files "$found_files" "Unwanted.swift")
  result="$(cat "$filtered_file")"

  expected="/path/to/FileA.swift
/path/to/FileB.swift"

  [ "$result" = "$expected" ]

  rm -f "$found_files" "$filtered_file"
}

@test "filter_excluded_files removes multiple matching files" {
  found_files=$(mktemp)
  echo "/path/to/FileA.swift" >> "$found_files"
  echo "/path/to/ExcludeThis.swift" >> "$found_files"
  echo "/path/to/Another.swift" >> "$found_files"
  echo "/path/to/RemoveMe.swift" >> "$found_files"
  echo "/path/to/FileB.swift" >> "$found_files"

  filtered_file=$(filter_excluded_files "$found_files" "ExcludeThis.swift" "RemoveMe.swift")
  result="$(cat "$filtered_file")"

  expected="/path/to/FileA.swift
/path/to/Another.swift
/path/to/FileB.swift"

  [ "$result" = "$expected" ]

  rm -f "$found_files" "$filtered_file"
}

@test "filter_excluded_files only excludes exact matches" {
  found_files=$(mktemp)
  echo "/path/to/FileA.swift" >> "$found_files"
  echo "/path/to/ExtraFile.swift" >> "$found_files"
  echo "/path/to/FileB.swift" >> "$found_files"

  # Using an exclusion that is a substring (and not an exact match)
  # should not remove ExtraFile.swift.
  filtered_file=$(filter_excluded_files "$found_files" "File.swift")
  result="$(cat "$filtered_file")"

  expected="/path/to/FileA.swift
/path/to/ExtraFile.swift
/path/to/FileB.swift"

  [ "$result" = "$expected" ]

  rm -f "$found_files" "$filtered_file"
}

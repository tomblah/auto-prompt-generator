#!/usr/bin/env bats

setup() {
  TMP_DIR=$(mktemp -d)
}

teardown() {
  rm -rf "$TMP_DIR"
}

# Load the filter_files component.
load "${BATS_TEST_DIRNAME}/filter_files.sh"

@test "filter_files_for_slim_mode includes only the TODO file and allowed files" {
    # Create a temporary file simulating the list of found files.
    found_files=$(mktemp)
    
    # Simulated file paths.
    echo "/path/to/TODO.swift" >> "$found_files"
    echo "/path/to/Model.swift" >> "$found_files"
    echo "/path/to/ViewController.swift" >> "$found_files"
    echo "/path/to/Manager.swift" >> "$found_files"
    echo "/path/to/ExtraModel.swift" >> "$found_files"
    
    todo_file="/path/to/TODO.swift"
    
    # Call the function.
    filtered_file=$(filter_files_for_slim_mode "$todo_file" "$found_files")
    
    # Read the filtered result.
    result="$(cat "$filtered_file")"
    
    # Expected output: The TODO file, Model.swift, and ExtraModel.swift (in that order).
    expected="/path/to/TODO.swift
/path/to/Model.swift
/path/to/ExtraModel.swift"
    
    [ "$result" = "$expected" ]
    
    rm -f "$found_files" "$filtered_file"
}

@test "filter_files_for_slim_mode ignores duplicate TODO file entries" {
    found_files=$(mktemp)
    
    # Add duplicate TODO entries.
    echo "/path/to/TODO.swift" >> "$found_files"
    echo "/path/to/TODO.swift" >> "$found_files"
    echo "/path/to/ValidModel.swift" >> "$found_files"
    
    todo_file="/path/to/TODO.swift"
    
    filtered_file=$(filter_files_for_slim_mode "$todo_file" "$found_files")
    result="$(cat "$filtered_file")"
    
    # Expected: Only one TODO file and the valid model file.
    expected="/path/to/TODO.swift
/path/to/ValidModel.swift"
    
    [ "$result" = "$expected" ]
    
    rm -f "$found_files" "$filtered_file"
}

@test "filter_files_for_slim_mode with empty found files list returns only the TODO file" {
    found_files=$(mktemp)
    
    # Do not add any candidate file.
    todo_file="/path/to/TODO.swift"
    
    filtered_file=$(filter_files_for_slim_mode "$todo_file" "$found_files")
    result="$(cat "$filtered_file")"
    
    expected="/path/to/TODO.swift"
    
    [ "$result" = "$expected" ]
    
    rm -f "$found_files" "$filtered_file"
}

@test "filter_files_for_slim_mode excludes files with new keywords" {
    found_files=$(mktemp)
    
    # Include files with the new exclusion keywords.
    echo "/path/to/TODO.swift" >> "$found_files"  # TODO file always included.
    echo "/path/to/Configurator.swift" >> "$found_files"
    echo "/path/to/DataSource.swift" >> "$found_files"
    echo "/path/to/Delegate.swift" >> "$found_files"
    echo "/path/to/MyView.swift" >> "$found_files"
    echo "/path/to/ValidModel.swift" >> "$found_files"
    
    todo_file="/path/to/TODO.swift"
    
    filtered_file=$(filter_files_for_slim_mode "$todo_file" "$found_files")
    result="$(cat "$filtered_file")"
    
    # Expected: Only TODO.swift and ValidModel.swift are allowed.
    expected="/path/to/TODO.swift
/path/to/ValidModel.swift"
    
    [ "$result" = "$expected" ]
    
    rm -f "$found_files" "$filtered_file"
}

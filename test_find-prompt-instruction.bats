#!/usr/bin/env bats

setup() {
    TEST_DIR=$(mktemp -d)
}

teardown() {
    rm -rf "$TEST_DIR"
}

load ./find-prompt-instruction.sh

@test "No Swift file with valid TODO pattern returns error" {
    # Create a Swift file with a non-matching TODO.
    echo "// TODO: Something else entirely" > "$TEST_DIR/File.swift"
    
    run find-prompt-instruction "$TEST_DIR"
    [ "$status" -ne 0 ]
    # Adjusted expectation: our script now outputs "Error: No files found containing '// TODO: - '"
    [[ "$output" == *"Error: No files found containing '// TODO: - '"* ]]
}

@test "Multiple Swift files with valid TODO instructions returns the most recently modified file" {
    # Create two Swift files with TODO instructions.
    echo "// TODO: - First instruction" > "$TEST_DIR/File1.swift"
    # Set an older modification time for File1.swift.
    touch -t 200001010000 "$TEST_DIR/File1.swift"
    
    echo "// TODO: - Second instruction" > "$TEST_DIR/File2.swift"
    # Set a later modification time for File2.swift.
    touch -t 202501010000 "$TEST_DIR/File2.swift"
    
    run find-prompt-instruction "$TEST_DIR"
    [ "$status" -eq 0 ]
    # Expect that File2.swift is returned because it was modified later.
    [[ "$output" == *"File2.swift"* ]]
}

@test "Single Swift file with '// TODO: - ' returns its path" {
    echo "// TODO: - Only instruction" > "$TEST_DIR/File.swift"
    
    run find-prompt-instruction "$TEST_DIR"
    [ "$status" -eq 0 ]
    [[ "$output" == *"File.swift"* ]]
}

@test "Single Swift file with '// TODO: ChatGPT: ' returns error" {
    echo "// TODO: ChatGPT: Only instruction" > "$TEST_DIR/File.swift"
    
    run find-prompt-instruction "$TEST_DIR"
    [ "$status" -ne 0 ]
    [[ "$output" == *"Error:"* ]]
}

@test "Swift file in Pods directory is ignored when a valid non-Pods file exists" {
    # Create a TODO file inside Pods.
    mkdir -p "$TEST_DIR/Pods"
    echo "// TODO: - Pods instruction" > "$TEST_DIR/Pods/PodsFile.swift"
    # Create a valid TODO file outside of Pods.
    echo "// TODO: - Valid instruction" > "$TEST_DIR/Valid.swift"
    touch -t 202501010000 "$TEST_DIR/Valid.swift"
    
    run find-prompt-instruction "$TEST_DIR"
    [ "$status" -eq 0 ]
    # Expect that the returned file is Valid.swift and not the Pods file.
    [[ "$output" == *"Valid.swift"* ]]
    [[ "$output" != *"PodsFile.swift"* ]]
}

@test "Swift file only in Pods directory is ignored and returns error" {
    mkdir -p "$TEST_DIR/Pods"
    echo "// TODO: - Only Pods instruction" > "$TEST_DIR/Pods/OnlyPods.swift"
    
    run find-prompt-instruction "$TEST_DIR"
    [ "$status" -ne 0 ]
    [[ "$output" == *"Error:"* ]]
}

# --- New tests for Objective-C files ---

@test "Single Objective-C header file with valid TODO returns its path" {
    echo "// TODO: - Header instruction" > "$TEST_DIR/Header.h"
    run find-prompt-instruction "$TEST_DIR"
    [ "$status" -eq 0 ]
    [[ "$output" == *"Header.h"* ]]
}

@test "Single Objective-C implementation file with valid TODO returns its path" {
    echo "// TODO: - Implementation instruction" > "$TEST_DIR/Impl.m"
    run find-prompt-instruction "$TEST_DIR"
    [ "$status" -eq 0 ]
    [[ "$output" == *"Impl.m"* ]]
}

@test "When both Swift and Objective-C files with valid TODO exist, the most recently modified is returned" {
    echo "// TODO: - Swift instruction" > "$TEST_DIR/SwiftFile.swift"
    touch -t 200001010000 "$TEST_DIR/SwiftFile.swift"
    echo "// TODO: - ObjC instruction" > "$TEST_DIR/ObjCFile.h"
    touch -t 202501010000 "$TEST_DIR/ObjCFile.h"
    run find-prompt-instruction "$TEST_DIR"
    [ "$status" -eq 0 ]
    [[ "$output" == *"ObjCFile.h"* ]]
}

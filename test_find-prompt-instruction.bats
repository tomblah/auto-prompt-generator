#!/usr/bin/env bats

setup() {
    TEST_DIR=$(mktemp -d)
}

teardown() {
    rm -rf "$TEST_DIR"
}

load ./find-prompt-instruction.sh

@test "No Swift file with either TODO pattern returns error" {
    # Create a Swift file with a non-matching TODO.
    echo "// TODO: Something else entirely" > "$TEST_DIR/File.swift"
    
    run find-prompt-instruction "$TEST_DIR"
    [ "$status" -ne 0 ]
    [[ "$output" == *"Error: No Swift files found"* ]]
}

@test "Multiple Swift files with TODO patterns returns the most recently modified file" {
    # Create two Swift files with TODO instructions.
    echo "// TODO: - First instruction" > "$TEST_DIR/File1.swift"
    # Set an older modification time for File1.swift.
    touch -t 200001010000 "$TEST_DIR/File1.swift"
    
    echo "// TODO: ChatGPT: Second instruction" > "$TEST_DIR/File2.swift"
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

@test "Single Swift file with '// TODO: ChatGPT: ' returns its path" {
    echo "// TODO: ChatGPT: Only instruction" > "$TEST_DIR/File.swift"
    
    run find-prompt-instruction "$TEST_DIR"
    [ "$status" -eq 0 ]
    [[ "$output" == *"File.swift"* ]]
}

#!/usr/bin/env bats

setup() {
    TEST_DIR=$(mktemp -d)
}

teardown() {
    rm -rf "$TEST_DIR"
}

load ./find_prompt_instruction.sh

@test "No Swift file with either TODO pattern returns error" {
    # Create a Swift file with a non-matching TODO.
    echo "// TODO: Something else entirely" > "$TEST_DIR/File.swift"
    
    run find_prompt_instruction "$TEST_DIR"
    [ "$status" -ne 0 ]
    [[ "$output" == *"Error: No Swift files found"* ]]
}

@test "Multiple Swift files with TODO patterns return error" {
    echo "// TODO: - First instruction" > "$TEST_DIR/File1.swift"
    echo "// TODO: ChatGPT: Second instruction" > "$TEST_DIR/File2.swift"
    
    run find_prompt_instruction "$TEST_DIR"
    [ "$status" -ne 0 ]
    [[ "$output" == *"Error: More than one instruction found:"* ]]
}

@test "Single Swift file with '// TODO: - ' returns its path" {
    echo "// TODO: - Only instruction" > "$TEST_DIR/File.swift"
    
    run find_prompt_instruction "$TEST_DIR"
    [ "$status" -eq 0 ]
    [[ "$output" == *"File.swift"* ]]
}

@test "Single Swift file with '// TODO: ChatGPT: ' returns its path" {
    echo "// TODO: ChatGPT: Only instruction" > "$TEST_DIR/File.swift"
    
    run find_prompt_instruction "$TEST_DIR"
    [ "$status" -eq 0 ]
    [[ "$output" == *"File.swift"* ]]
}

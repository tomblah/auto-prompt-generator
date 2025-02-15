#!/usr/bin/env bats
# test_check-prompt-size.bats
#
# This file tests the check_prompt_size function defined in check-prompt-size.sh.
# With the threshold set to 100,000 characters, the function should:
#   - Not output a warning for prompts shorter than or exactly 100,000 characters.
#   - Output a warning for prompts that exceed 100,000 characters.

setup() {
  TMP_DIR=$(mktemp -d)
}

teardown() {
  rm -rf "$TMP_DIR"
}

@test "check_prompt_size does not output warning for a short prompt" {
  run bash -c "source \"./check-prompt-size.sh\"; check_prompt_size 'Short prompt'"
  [ "$status" -eq 0 ]
  [ -z "$output" ]
}

@test "check_prompt_size does not output warning for a prompt exactly at threshold" {
  run bash -c "source \"./check-prompt-size.sh\"; prompt=\$(head -c 100000 /dev/zero | tr '\\0' 'a'); check_prompt_size \"\$prompt\""
  [ "$status" -eq 0 ]
  [ -z "$output" ]
}

@test "check_prompt_size outputs warning for a prompt exceeding threshold" {
  run bash -c "source \"./check-prompt-size.sh\"; prompt=\$(head -c 100100 /dev/zero | tr '\\0' 'a'); check_prompt_size \"\$prompt\""
  [ "$status" -eq 0 ]
  [[ "$output" == *"Warning: The prompt is"* ]]
}

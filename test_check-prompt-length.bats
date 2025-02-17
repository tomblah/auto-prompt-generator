#!/usr/bin/env bats

setup() {
  # Assume check-prompt-length.sh is in the same directory as this test file.
  source "$(dirname "$BATS_TEST_FILENAME")/check-prompt-length.sh"
}

@test "does not output warning when prompt length is below threshold" {
  run bash -c './check-prompt-length.sh "Short prompt" 2>&1'
  [ "$status" -eq 0 ]
  [ -z "$output" ]
}

@test "outputs warning when prompt length exceeds threshold" {
  # Create a string with 600001 characters (default threshold is 600000)
  prompt="$(head -c 600001 < /dev/zero | tr '\0' 'a')"
  run bash -c './check-prompt-length.sh "$0" 2>&1' "$prompt"
  [ "$status" -eq 0 ]
  # Check that the output contains "Warning: The prompt is" and a number of characters.
  [[ "$output" =~ Warning:\ The\ prompt\ is\ [0-9]+\ characters\ long\. ]]
}

@test "warning message shows trimmed character count" {
  # Create a string of exactly 627286 characters.
  prompt="$(head -c 627286 < /dev/zero | tr '\0' 'a')"
  run bash -c './check-prompt-length.sh "$0" 2>&1' "$prompt"
  expected="Warning: The prompt is 627286 characters long. This may exceed what the AI can handle effectively."
  [ "$output" = "$expected" ]
}

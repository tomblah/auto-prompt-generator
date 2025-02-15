use std::io::{self, Read};

/// Checks the length of the prompt and returns a warning message if it exceeds MAX_LENGTH.
pub fn check_prompt_length(input: &str) -> Option<String> {
    const MAX_LENGTH: usize = 100_000;
    let prompt_length = input.chars().count();
    if prompt_length > MAX_LENGTH {
        Some(format!(
            "Warning: The prompt is {} characters long. This may exceed what the AI can handle effectively.",
            prompt_length
        ))
    } else {
        None
    }
}

/// Processes the prompt and returns:
/// - Ok(message) if the prompt length is acceptable (i.e. no suggestions for exclusion),
/// - Err(warning) if the prompt length exceeds the acceptable maximum.
pub fn process_prompt(input: &str) -> Result<String, String> {
    if let Some(warning) = check_prompt_length(input) {
        Err(warning)
    } else {
        Ok("The prompt length is within acceptable limits. No suggestions for exclusion.".to_string())
    }
}

fn main() {
    let mut input = String::new();
    // Read all input from STDIN.
    if let Err(e) = io::stdin().read_to_string(&mut input) {
        eprintln!("Error reading input: {}", e);
        std::process::exit(1);
    }
    
    match process_prompt(&input) {
        Ok(message) => println!("{}", message),
        Err(warning) => eprintln!("{}", warning),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for the low-level check_prompt_length function.

    #[test]
    fn test_prompt_below_limit() {
        let input = "This is a short prompt.";
        // Should not trigger a warning.
        assert!(check_prompt_length(input).is_none());
    }

    #[test]
    fn test_prompt_at_limit() {
        // Exactly at 100,000 characters.
        let input = "a".repeat(100_000);
        // Should not trigger a warning since the condition is ">" not ">=".
        assert!(check_prompt_length(&input).is_none());
    }

    #[test]
    fn test_prompt_above_limit() {
        // 100,001 characters, which exceeds the threshold.
        let input = "a".repeat(100_001);
        let expected_warning = format!(
            "Warning: The prompt is {} characters long. This may exceed what the AI can handle effectively.",
            100_001
        );
        assert_eq!(check_prompt_length(&input).unwrap(), expected_warning);
    }

    // Tests for the higher-level process_prompt function.

    #[test]
    fn test_process_prompt_below_limit() {
        let input = "Short prompt";
        let result = process_prompt(input);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "The prompt length is within acceptable limits. No suggestions for exclusion."
        );
    }

    #[test]
    fn test_process_prompt_at_limit() {
        let input = "a".repeat(100_000);
        let result = process_prompt(&input);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "The prompt length is within acceptable limits. No suggestions for exclusion."
        );
    }

    #[test]
    fn test_process_prompt_above_limit() {
        let input = "a".repeat(100_001);
        let result = process_prompt(&input);
        assert!(result.is_err());
        let expected_warning = format!(
            "Warning: The prompt is {} characters long. This may exceed what the AI can handle effectively.",
            100_001
        );
        assert_eq!(result.unwrap_err(), expected_warning);
    }
}

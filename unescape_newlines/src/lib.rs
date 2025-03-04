use std::io::{self, Read, Write};

/// Converts literal "\n" sequences in the input string to actual newline characters.
pub fn unescape_newlines(input: &str) -> String {
    input.replace("\\n", "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_escape_sequences() {
        let input = "This is a test.";
        let expected = "This is a test.";
        assert_eq!(unescape_newlines(input), expected);
    }

    #[test]
    fn test_single_escape_sequence() {
        let input = "Line1\\nLine2";
        let expected = "Line1\nLine2";
        assert_eq!(unescape_newlines(input), expected);
    }

    #[test]
    fn test_multiple_escape_sequences() {
        let input = "Line1\\nLine2\\nLine3";
        let expected = "Line1\nLine2\nLine3";
        assert_eq!(unescape_newlines(input), expected);
    }

    #[test]
    fn test_escape_at_beginning_and_end() {
        let input = "\\nLine1\\n";
        let expected = "\nLine1\n";
        assert_eq!(unescape_newlines(input), expected);
    }

    #[test]
    fn test_consecutive_escape_sequences() {
        let input = "Line1\\n\\nLine2";
        let expected = "Line1\n\nLine2";
        assert_eq!(unescape_newlines(input), expected);
    }
}

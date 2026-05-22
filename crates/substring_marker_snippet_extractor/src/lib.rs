// crates/substring_marker_snippet_extractor/src/lib.rs

/*!
# substring_marker_snippet_extractor

Marker-filtering and enclosing-block extraction for the prompt-generation
pipeline. Substring markers (`// v` … `// ^`) delimit the region of a source
file that should be included in a prompt, and the enclosing-block helpers find
the surrounding function/class/enum for additional context.

The `FileProcessor` trait and its default implementation have moved to the
`assemble_prompt` crate (their sole consumer). TODO-position analysis helpers
(`todo_index`, `is_todo_inside_markers`) now live in the `todo_marker` crate.
*/

pub mod utils;

pub use utils::marker_utils::{
    extract_enclosing_block, extract_enclosing_block_from_content, file_uses_markers,
    filter_substring_markers,
};

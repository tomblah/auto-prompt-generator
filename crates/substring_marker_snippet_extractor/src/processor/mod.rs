// crates/substring_marker_snippet_extractor/src/processor/mod.rs

pub mod file_processor;
pub use file_processor::{DefaultFileProcessor, process_file_with_processor};

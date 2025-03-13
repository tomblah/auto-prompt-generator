// crates/substring_marker_snippet_extractor/src/lib.rs
/*!
# substring_marker_snippet_extractor

This crate provides functionality for extracting and processing code snippets
from source files based on substring markers. It includes:

- **Marker Utilities:** Functions to filter content between markers, determine if
  markers are used, locate TODO markers, and extract enclosing code blocks.
- **File Processing:** A trait-based API for processing files with a default
  implementation that uses the marker utilities.

The crate is organized into two primary modules:
- `utils::marker_utils`: Contains helper functions for handling marker-related logic.
- `processor`: Defines the `FileProcessor` trait and provides a default implementation.

This modular design facilitates reuse of common marker processing logic and
encourages a clear separation of concerns.
*/

pub mod utils;
pub mod processor;

pub use utils::marker_utils::filter_substring_markers;

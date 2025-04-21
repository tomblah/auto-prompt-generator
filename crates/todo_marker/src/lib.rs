// crates/todo_marker/src/lib.rs

//! throughout the prompt‑generation tool‑chain.

/// Exact form **without** the trailing space.
pub const TODO_MARKER: &str = "// TODO: -";

/// Exact form **with** a trailing space (the version most
/// parsers look for when scanning file content).
pub const TODO_MARKER_WS: &str = "// TODO: - ";

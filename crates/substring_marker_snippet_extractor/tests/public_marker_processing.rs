// crates/substring_marker_snippet_extractor/tests/public_marker_processing.rs

use std::io::Write;

use substring_marker_snippet_extractor::filter_substring_markers;
use substring_marker_snippet_extractor::processor::{
    process_file_with_processor,
    DefaultFileProcessor,
};
use tempfile::NamedTempFile;

#[test]
fn public_marker_processing_filters_markers_and_appends_enclosing_context() {
    let content = concat!(
        "function handleRequest() {\n",
        "    return true;\n",
        "}\n",
        "\n",
        "// v\n",
        "selected line\n",
        "// ^\n",
        "\n",
        "// TODO: - update behavior\n",
    );

    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(content.as_bytes())
        .expect("Failed to write temp file");
    let file_name = file
        .path()
        .file_name()
        .and_then(|name| name.to_str())
        .expect("Temp file name should be valid UTF-8");

    let output = process_file_with_processor(&DefaultFileProcessor, file.path(), Some(file_name))
        .expect("DefaultFileProcessor should process the temp file");

    assert!(output.starts_with(&filter_substring_markers(content, "// ...")));
    assert!(output.contains("// Enclosing function context:"));
    assert!(output.contains("function handleRequest() {"));
}

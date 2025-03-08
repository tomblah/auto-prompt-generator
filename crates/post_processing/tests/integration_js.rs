use post_processing::scrub_extra_todo_markers;

#[test]
fn integration_js_content_primary_marker() {
    let primary_marker = "// TODO: - JS Primary Marker";
    let input = r#"// Some JavaScript code
function greet() {
    console.log("Hello, world!");
}
// TODO: - JS Primary Marker
var x = 42;
// Some more JS code
// TODO: - Extra Marker
function add(a, b) {
    return a + b;
}
// End of code
// TODO: - CTA Marker"#;
    let expected = r#"// Some JavaScript code
function greet() {
    console.log("Hello, world!");
}
// TODO: - JS Primary Marker
var x = 42;
// Some more JS code
function add(a, b) {
    return a + b;
}
// End of code
// TODO: - CTA Marker"#;
    
    let output = scrub_extra_todo_markers(input, false, primary_marker)
        .expect("Primary marker should be found");
    assert_eq!(output, expected);
}

#[test]
fn integration_js_content_missing_primary_marker() {
    let primary_marker = "// TODO: - JS Primary Marker";
    let input = r#"// Some JavaScript code
function greet() {
    console.log("Hello, world!");
}
// TODO: - Extra Marker
var x = 42;
// Some more JS code"#;
    let result = scrub_extra_todo_markers(input, false, primary_marker);
    assert!(result.is_err());
}

use post_processing::scrub_extra_todo_markers;

#[test]
fn integration_swift_content_primary_marker() {
    let primary_marker = "// TODO: - Swift Primary Marker";
    let input = r#"import Foundation
// TODO: - Swift Primary Marker
func someFunction() {
    print("Hello, world!")
}
// Some random code
// TODO: - Extra Marker
func anotherFunction() {
    print("Extra marker code")
}
// More code
// TODO: - Another Extra Marker
// TODO: - CTA Marker"#;
    let expected = r#"import Foundation
// TODO: - Swift Primary Marker
func someFunction() {
    print("Hello, world!")
}
// Some random code
func anotherFunction() {
    print("Extra marker code")
}
// More code
// TODO: - CTA Marker"#;
    
    let output = scrub_extra_todo_markers(input, false, primary_marker)
        .expect("Primary marker should be found");
    assert_eq!(output, expected);
}

#[test]
fn integration_swift_content_missing_primary_marker() {
    let primary_marker = "// TODO: - Swift Primary Marker";
    let input = r#"import Foundation
// TODO: - Extra Marker
func someFunction() {
    print("Hello, world!")
}
// Some random code
// TODO: - CTA Marker"#;
    let result = scrub_extra_todo_markers(input, false, primary_marker);
    assert!(result.is_err());
}

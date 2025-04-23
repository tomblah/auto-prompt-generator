use lang_support::for_extension;
use std::fs;
use tempfile::tempdir;

#[test]
fn js_walk_finds_relative_import() {
    let dir = tempdir().unwrap();
    let main = dir.path().join("main.js");
    let util = dir.path().join("utils.js");
    fs::write(&main,  r#"import { helper } from "./utils.js";"#).unwrap();
    fs::write(&util,  r#"export function helper() {}"#).unwrap();

    let lang = for_extension("js").unwrap();
    let deps = lang.walk_dependencies(&main, dir.path());
    assert_eq!(deps, vec![util]);
}

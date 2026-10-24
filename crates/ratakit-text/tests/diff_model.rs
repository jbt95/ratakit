use ratakit_text::{ChangeTag, DiffModel};

#[test]
fn from_texts_builds_hunks_with_inline_spans() {
    let old = "fn greet() {\n    println!(\"hi\");\n}\n";
    let new = "fn greet() {\n    println!(\"hello\");\n}\n";
    let model = DiffModel::from_texts("main.rs", old, new);
    assert_eq!(model.files.len(), 1);
    let file = &model.files[0];
    assert_eq!(file.new_path.as_deref(), Some("main.rs"));
    assert!(!file.hunks.is_empty());
    let changed = file
        .hunks
        .iter()
        .flat_map(|h| h.lines.iter())
        .find(|l| l.tag == ChangeTag::Delete || l.tag == ChangeTag::Insert);
    assert!(changed.is_some());
}

#[test]
fn from_unified_parses_paths_and_markers() {
    let patch = "\
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,3 +1,3 @@
 fn main() {
-    let x = 1;
+    let x = 2;
 }
";
    let model = DiffModel::from_unified(patch).expect("parse");
    assert_eq!(model.files.len(), 1);
    let file = &model.files[0];
    assert_eq!(file.old_path.as_deref(), Some("src/lib.rs"));
    let hunk = &file.hunks[0];
    assert!(hunk.lines.len() >= 3);
    assert_eq!(hunk.lines[1].tag, ChangeTag::Delete);
    assert_eq!(hunk.lines[2].tag, ChangeTag::Insert);
}

#[test]
fn toggle_hunk_collapsed() {
    let model = DiffModel::from_texts("a.rs", "a\n", "b\n");
    assert!(!model.files[0].hunks[0].collapsed);
    let mut model = model;
    model.toggle_hunk_collapsed(0, 0);
    assert!(model.files[0].hunks[0].collapsed);
}

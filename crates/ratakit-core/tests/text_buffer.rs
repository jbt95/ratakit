use ratakit_core::{Atom, TextBuffer};

#[test]
fn insert_and_undo() {
    let mut buf = TextBuffer::new();
    buf.insert_str("hello");
    assert_eq!(buf.text(), "hello");
    assert!(buf.undo());
    assert_eq!(buf.text(), "");
}

#[test]
fn large_paste_becomes_atom() {
    let mut buf = TextBuffer::new().with_paste_line_threshold(3);
    buf.insert_paste("a\nb\nc\nd");
    assert_eq!(buf.text(), "[Pasted 4 lines]");
    assert_eq!(buf.atoms().len(), 1);
}

#[test]
fn mention_atom_round_trip() {
    let mut buf = TextBuffer::new();
    buf.insert_str("see ");
    buf.insert_atom(Atom::mention("@src/main.rs"));
    assert_eq!(buf.text(), "see @src/main.rs");
}

#[test]
fn delete_backward_removes_char() {
    let mut buf = TextBuffer::new();
    buf.insert_str("ab");
    buf.move_right();
    buf.delete_backward();
    assert_eq!(buf.text(), "a");
}

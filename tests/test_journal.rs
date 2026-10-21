use mupdf::pdf::{PdfDocument, PdfPage};
use mupdf::{Quad, Rect};

fn open() -> PdfDocument {
    PdfDocument::from_copied_bytes(include_bytes!("../tests/files/dummy.pdf")).unwrap()
}

fn page(doc: &PdfDocument) -> PdfPage {
    PdfPage::try_from(doc.load_page(0).unwrap()).unwrap()
}

fn annotations(doc: &PdfDocument) -> usize {
    page(doc).annotations().count()
}

/// Adds one highlight as one operation.
fn highlight(doc: &mut PdfDocument, y: f32) {
    doc.begin_operation("Highlight").unwrap();
    let mut p = page(doc);
    let mut annot = p
        .add_highlight_annotation([Quad::from(Rect::new(50.0, y, 200.0, y + 12.0))])
        .unwrap();
    annot.update().unwrap();
    drop(annot);
    drop(p);
    doc.end_operation().unwrap();
}

#[test]
fn test_undo_and_redo_walk_the_history() {
    let mut doc = open();
    doc.enable_journal().unwrap();
    assert_eq!(doc.undo_redo_state().unwrap(), (0, 0));
    assert!(!doc.can_undo().unwrap());

    highlight(&mut doc, 50.0);
    highlight(&mut doc, 80.0);
    assert_eq!(annotations(&doc), 2);
    assert_eq!(doc.undo_redo_state().unwrap(), (2, 2));

    doc.undo().unwrap();
    assert_eq!(annotations(&doc), 1);
    assert_eq!(doc.undo_redo_state().unwrap(), (1, 2));
    assert!(doc.can_undo().unwrap() && doc.can_redo().unwrap());

    doc.undo().unwrap();
    assert_eq!(annotations(&doc), 0);
    assert!(!doc.can_undo().unwrap());

    doc.redo().unwrap();
    doc.redo().unwrap();
    assert_eq!(annotations(&doc), 2);
    assert!(!doc.can_redo().unwrap());
}

#[test]
fn test_a_new_operation_after_undo_discards_redo() {
    let mut doc = open();
    doc.enable_journal().unwrap();
    highlight(&mut doc, 50.0);
    highlight(&mut doc, 80.0);
    doc.undo().unwrap();
    highlight(&mut doc, 110.0);
    assert_eq!(doc.undo_redo_state().unwrap(), (2, 2));
    assert!(!doc.can_redo().unwrap());
}

#[test]
fn test_journal_errors_are_returned_not_thrown() {
    // Without a journal.
    let mut doc = open();
    assert_eq!(doc.undo_redo_state().unwrap(), (0, 0));
    assert!(doc.undo().is_err());
    assert!(doc.redo().is_err());

    // At the start and the end of the history.
    doc.enable_journal().unwrap();
    assert!(doc.undo().is_err());
    highlight(&mut doc, 50.0);
    assert!(doc.redo().is_err());

    // While an operation is open.
    doc.begin_operation("Open").unwrap();
    assert!(doc.undo().is_err());
    assert!(doc.undo_redo_state().is_err());
    doc.abandon_operation().unwrap();
    doc.undo().unwrap();
    assert_eq!(annotations(&doc), 0);
}

use mupdf::pdf::PdfDocument;
use mupdf::Error;

fn open() -> PdfDocument {
    PdfDocument::from_copied_bytes(include_bytes!("files/dummy.pdf")).unwrap()
}

fn rotation(doc: &PdfDocument) -> Result<i32, Error> {
    doc.load_pdf_page(0)?.rotation()
}

fn rotate(doc: &mut PdfDocument, degrees: i32) -> Result<(), Error> {
    doc.begin_operation("Rotate page")?;
    doc.load_pdf_page(0)?.set_rotation(degrees)?;
    doc.end_operation()
}

#[test]
fn undo_and_redo_walk_journal_steps() -> Result<(), Error> {
    let mut doc = open();
    doc.enable_journal()?;
    assert_eq!(doc.undo_redo_state()?, (0, 0));

    rotate(&mut doc, 90)?;
    rotate(&mut doc, 180)?;
    assert_eq!(doc.undo_redo_state()?, (2, 2));
    assert!(doc.can_undo()?);
    assert!(!doc.can_redo()?);

    doc.undo()?;
    assert_eq!(rotation(&doc)?, 90);
    doc.undo()?;
    assert_eq!(rotation(&doc)?, 0);
    assert_eq!(doc.undo_redo_state()?, (0, 2));
    assert!(!doc.can_undo()?);
    assert!(doc.can_redo()?);

    doc.redo()?;
    doc.redo()?;
    assert_eq!(rotation(&doc)?, 180);
    Ok(())
}

#[test]
fn an_edit_after_undo_discards_redo() -> Result<(), Error> {
    let mut doc = open();
    doc.enable_journal()?;
    rotate(&mut doc, 90)?;
    doc.undo()?;

    rotate(&mut doc, 270)?;

    assert_eq!(doc.undo_redo_state()?, (1, 1));
    assert!(!doc.can_redo()?);
    Ok(())
}

#[test]
fn undo_without_a_journal_is_an_error() {
    let mut doc = open();

    assert!(doc.undo().is_err());
    assert!(doc.redo().is_err());
    assert_eq!(doc.undo_redo_state().unwrap(), (0, 0));
    assert!(!doc.can_undo().unwrap());
}

#[test]
fn undo_past_the_start_and_redo_past_the_end_are_errors() -> Result<(), Error> {
    let mut doc = open();
    doc.enable_journal()?;

    assert!(doc.undo().is_err());
    assert!(doc.redo().is_err());
    Ok(())
}

#[test]
fn undo_during_an_operation_is_an_error() -> Result<(), Error> {
    let mut doc = open();
    doc.enable_journal()?;
    rotate(&mut doc, 90)?;

    doc.begin_operation("Unfinished")?;
    assert!(doc.undo().is_err());
    assert!(doc.can_undo().is_err());
    doc.end_operation()?;

    doc.undo()?;
    assert_eq!(rotation(&doc)?, 0);
    Ok(())
}

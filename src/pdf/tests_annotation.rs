use crate::pdf::{
    annotation::AnnotationFlags, Intent, PdfAnnotationType, PdfDocument, PdfFilterOptions,
};
use crate::{color::AnnotationColor, Point, Rect, Size};

const PAGE_SIZE: Size = Size::A4;

struct AnnotTester {
    doc: PdfDocument,
}

impl AnnotTester {
    fn new() -> Self {
        let mut doc = PdfDocument::new();
        doc.new_page(PAGE_SIZE).unwrap();
        Self { doc }
    }

    fn page(&self) -> crate::pdf::PdfPage {
        self.doc.load_pdf_page(0).unwrap()
    }

    fn create(&self, ty: PdfAnnotationType) -> crate::pdf::PdfAnnotation {
        self.page().create_annotation(ty).unwrap()
    }
}

#[test]
fn annotation_properties_and_soundness() {
    let tester = AnnotTester::new();

    let mut text_annot = tester.create(PdfAnnotationType::Text);
    let mut square_annot = tester.create(PdfAnnotationType::Square);
    let mut line_annot = tester.create(PdfAnnotationType::Line);

    assert_eq!(text_annot.r#type().unwrap(), PdfAnnotationType::Text);
    assert_eq!(square_annot.r#type().unwrap(), PdfAnnotationType::Square);
    assert_eq!(line_annot.r#type().unwrap(), PdfAnnotationType::Line);

    assert!(!text_annot.is_hot());
    text_annot.set_hot(true);
    assert!(text_annot.is_hot());

    assert!(!text_annot.is_active());
    text_annot.set_active(true).unwrap();
    assert!(text_annot.is_active());

    let author = text_annot.author().unwrap();
    assert!(author.is_none() || author == Some(""));
    text_annot.set_author("Tester").unwrap();
    assert_eq!(text_annot.author().unwrap(), Some("Tester"));

    text_annot
        .set_popup(Rect::new(100.0, 100.0, 300.0, 200.0))
        .unwrap();
    text_annot
        .set_flags(AnnotationFlags::IS_PRINT | AnnotationFlags::IS_LOCKED)
        .unwrap();

    square_annot
        .set_rect(Rect::new(10.0, 20.0, 100.0, 80.0))
        .unwrap();
    square_annot
        .set_color(AnnotationColor::Rgb {
            red: 1.0,
            green: 0.0,
            blue: 0.0,
        })
        .unwrap();
    square_annot.set_border_width(2.5).unwrap();

    let mut freetext_annot = tester.create(PdfAnnotationType::FreeText);
    freetext_annot.filter(PdfFilterOptions::default()).unwrap();

    line_annot
        .set_line(Point { x: 10.0, y: 20.0 }, Point { x: 200.0, y: 300.0 })
        .unwrap();
    line_annot.set_intent(Intent::LineArrow).unwrap();
}

#[test]
fn annotation_iteration_and_deletion() {
    let tester = AnnotTester::new();

    assert_eq!(tester.page().annotations().count(), 0);

    tester.create(PdfAnnotationType::Text);
    tester.create(PdfAnnotationType::Highlight);
    tester.create(PdfAnnotationType::Square);

    let first_annot = {
        let page = tester.page();
        let mut iter = page.annotations();

        drop(page);

        let first = iter.next().unwrap();
        assert_eq!(first.r#type().unwrap(), PdfAnnotationType::Text);

        let remaining: Vec<_> = iter.collect();
        assert_eq!(remaining.len(), 2);
        assert_eq!(remaining[0].r#type().unwrap(), PdfAnnotationType::Highlight);

        first
    };

    assert_eq!(first_annot.r#type().unwrap(), PdfAnnotationType::Text);

    let mut page = tester.page();
    page.delete_annotation(&first_annot).unwrap();

    let final_annots: Vec<_> = page.annotations().collect();
    assert_eq!(final_annots.len(), 2);
    assert_eq!(
        final_annots[0].r#type().unwrap(),
        PdfAnnotationType::Highlight
    );
}

#[test]
fn annotations_from_different_pages() {
    let mut doc = PdfDocument::new();
    doc.new_page(PAGE_SIZE).unwrap();
    doc.new_page(PAGE_SIZE).unwrap();

    let (annot0, annot1) = {
        let mut page0 = doc.load_pdf_page(0).unwrap();
        let mut page1 = doc.load_pdf_page(1).unwrap();
        let a0 = page0.create_annotation(PdfAnnotationType::Text).unwrap();
        let a1 = page1
            .create_annotation(PdfAnnotationType::Highlight)
            .unwrap();
        (a0, a1)
    };

    assert_eq!(annot0.r#type().unwrap(), PdfAnnotationType::Text);
    assert_eq!(annot1.r#type().unwrap(), PdfAnnotationType::Highlight);
}

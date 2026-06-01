use crate::pdf::{
    annotation::{AnnotationBorderStyle, AnnotationFlags},
    Intent, LineEndingStyle, PdfAnnotationType, PdfDocument, PdfFilterOptions,
};
use crate::shape::{Shape, TextOptions};
use crate::{color::AnnotationColor, Error, Point, Quad, Rect, Size};

const PAGE_SIZE: Size = Size::A4;

fn assert_rect_close(actual: Rect, expected: Rect) {
    const EPSILON: f32 = 0.001;
    assert!(
        (actual.x0 - expected.x0).abs() < EPSILON,
        "x0: {actual:?} != {expected:?}"
    );
    assert!(
        (actual.y0 - expected.y0).abs() < EPSILON,
        "y0: {actual:?} != {expected:?}"
    );
    assert!(
        (actual.x1 - expected.x1).abs() < EPSILON,
        "x1: {actual:?} != {expected:?}"
    );
    assert!(
        (actual.y1 - expected.y1).abs() < EPSILON,
        "y1: {actual:?} != {expected:?}"
    );
}

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

const TYPES: [PdfAnnotationType; 3] = [
    PdfAnnotationType::Text,
    PdfAnnotationType::Highlight,
    PdfAnnotationType::Square,
];

#[test]
fn annotation_iteration_and_deletion() {
    let tester = AnnotTester::new();

    assert_eq!(tester.page().annotations().count(), 0);

    for ty in TYPES {
        tester.create(ty);
    }

    let page = tester.page();
    let annots: Vec<_> = page.annotations().collect();
    assert_eq!(annots.len(), TYPES.len());
    for (annot, expected) in annots.iter().zip(TYPES) {
        assert_eq!(annot.r#type().unwrap(), expected);
    }

    // Annotations remain valid after the page is dropped
    drop(page);
    for (annot, expected) in annots.iter().zip(TYPES) {
        assert_eq!(annot.r#type().unwrap(), expected);
    }

    let tester = AnnotTester::new();

    for ty in TYPES {
        tester.create(ty);
    }

    // Collect annotations, then delete the first one
    let mut page = tester.page();
    let mut annots: Vec<_> = page.annotations().collect();
    let first = annots.remove(0);
    page.delete_annotation(first).unwrap();

    let remaining: Vec<_> = page.annotations().collect();
    assert_eq!(remaining.len(), TYPES.len() - 1);
    for (annot, expected) in remaining.iter().zip(&TYPES[1..]) {
        assert_eq!(annot.r#type().unwrap(), *expected);
    }

    let tester = AnnotTester::new();

    for ty in TYPES {
        tester.create(ty);
    }

    // Collect then delete all annotations
    let mut page = tester.page();
    let annots: Vec<_> = page.annotations().collect();
    for annot in annots {
        page.delete_annotation(annot).unwrap();
    }

    assert_eq!(page.annotations().count(), 0);
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

#[test]
fn annotation_property_roundtrips() {
    let tester = AnnotTester::new();
    let mut square = tester
        .page()
        .add_rect_annotation(Rect::new(10.0, 20.0, 100.0, 80.0))
        .unwrap();

    square.set_border_width(2.5).unwrap();
    square
        .set_border_style(AnnotationBorderStyle::Dashed)
        .unwrap();
    square.set_border_dash_pattern(&[2.0, 3.0]).unwrap();
    square
        .set_color(AnnotationColor::Rgb {
            red: 1.0,
            green: 0.0,
            blue: 0.0,
        })
        .unwrap();
    square
        .set_interior_color(AnnotationColor::Rgb {
            red: 0.0,
            green: 1.0,
            blue: 0.0,
        })
        .unwrap();
    square.set_opacity(0.5).unwrap();
    square
        .set_flags(AnnotationFlags::IS_PRINT | AnnotationFlags::IS_LOCKED)
        .unwrap();

    assert_eq!(square.border_width().unwrap(), 2.5);
    assert_eq!(
        square.border_style().unwrap(),
        AnnotationBorderStyle::Dashed
    );
    assert_eq!(square.border_dash_pattern().unwrap(), vec![2.0, 3.0]);
    assert_eq!(
        square.color().unwrap(),
        Some(AnnotationColor::Rgb {
            red: 1.0,
            green: 0.0,
            blue: 0.0,
        })
    );
    assert_eq!(
        square.interior_color().unwrap(),
        Some(AnnotationColor::Rgb {
            red: 0.0,
            green: 1.0,
            blue: 0.0,
        })
    );
    assert!(square.flags().unwrap().contains(AnnotationFlags::IS_LOCKED));
    assert!(square.opacity().unwrap() <= 0.5);
}

#[test]
fn annotation_convenience_geometry() {
    let tester = AnnotTester::new();
    let mut page = tester.page();

    let start = Point::new(10.0, 20.0);
    let end = Point::new(50.0, 60.0);
    let mut line = page.add_line_annotation(start, end).unwrap();
    assert_eq!(line.r#type().unwrap(), PdfAnnotationType::Line);
    assert_eq!(line.line().unwrap(), (start, end));
    assert!(!line.rect().unwrap().is_empty());
    line.set_line_ending_styles(LineEndingStyle::OpenArrow, LineEndingStyle::ClosedArrow)
        .unwrap();
    assert_eq!(
        line.line_ending_styles().unwrap(),
        (LineEndingStyle::OpenArrow, LineEndingStyle::ClosedArrow)
    );

    let polygon_points = [
        Point::new(10.0, 10.0),
        Point::new(50.0, 10.0),
        Point::new(30.0, 40.0),
    ];
    let polygon = page.add_polygon_annotation(polygon_points).unwrap();
    assert_eq!(polygon.r#type().unwrap(), PdfAnnotationType::Polygon);
    assert_eq!(polygon.vertices().unwrap(), polygon_points);

    let strokes = vec![
        vec![Point::new(100.0, 100.0), Point::new(120.0, 120.0)],
        vec![Point::new(130.0, 100.0), Point::new(150.0, 120.0)],
    ];
    let ink = page.add_ink_annotation(strokes.clone()).unwrap();
    assert_eq!(ink.r#type().unwrap(), PdfAnnotationType::Ink);
    assert_eq!(ink.ink_list().unwrap(), strokes);
}

#[test]
fn annotation_geometry_rects_roundtrip_on_rotated_pages() {
    let mut doc = PdfDocument::new();
    let mut page = doc.new_page(PAGE_SIZE).unwrap();
    page.set_rotation(90).unwrap();

    let start = Point::new(10.0, 20.0);
    let end = Point::new(50.0, 60.0);
    let line = page.add_line_annotation(start, end).unwrap();
    assert_eq!(line.line().unwrap(), (start, end));
    assert_rect_close(line.rect().unwrap(), Rect::new(7.0, 17.0, 53.0, 63.0));

    let quad = Quad::from(Rect::new(72.0, 72.0, 160.0, 90.0));
    let highlight = page.add_highlight_annotation(quad.clone()).unwrap();
    assert_eq!(highlight.quad_points().unwrap(), vec![quad.clone()]);
    assert_rect_close(highlight.rect().unwrap(), Rect::from(quad));
}

#[test]
fn text_markup_and_redaction_convenience() {
    let mut doc = PdfDocument::new();
    let mut page = doc.new_page(PAGE_SIZE).unwrap();

    let quad = Quad::from(Rect::new(72.0, 72.0, 160.0, 90.0));
    let highlight = page.add_highlight_annotation(quad.clone()).unwrap();
    assert_eq!(highlight.r#type().unwrap(), PdfAnnotationType::Highlight);
    assert_eq!(highlight.quad_points().unwrap(), vec![quad.clone()]);
    assert_eq!(highlight.rect().unwrap(), Rect::from(quad));

    {
        let mut shape = Shape::new(&mut page).unwrap();
        shape
            .insert_text(
                Point::new(72.0, 120.0),
                "SECRET visible text",
                &TextOptions::default(),
            )
            .unwrap()
            .commit(&mut doc, true)
            .unwrap();
    }
    assert_eq!(page.search("SECRET", 10).unwrap().len(), 1);

    page.add_redact_annotation(Rect::new(0.0, 0.0, 595.0, 842.0))
        .unwrap();
    assert!(page.apply_redactions().unwrap());
    assert_eq!(page.search("SECRET", 10).unwrap().len(), 0);
}

#[test]
fn detached_annotations_error_after_page_redaction() {
    let mut doc = PdfDocument::new();
    let mut page = doc.new_page(PAGE_SIZE).unwrap();
    let annot = page
        .add_redact_annotation(Rect::new(10.0, 10.0, 30.0, 30.0))
        .unwrap();

    page.apply_redactions().unwrap();

    assert!(matches!(
        annot.r#type(),
        Err(Error::InvalidArgument(message)) if message.contains("no longer attached")
    ));
}

#[test]
fn delete_annotation_rejects_wrong_page() {
    let mut doc = PdfDocument::new();
    doc.new_page(PAGE_SIZE).unwrap();
    doc.new_page(PAGE_SIZE).unwrap();
    let mut page0 = doc.load_pdf_page(0).unwrap();
    let mut page1 = doc.load_pdf_page(1).unwrap();
    let annot = page0.create_annotation(PdfAnnotationType::Text).unwrap();

    assert!(matches!(
        page1.delete_annotation(annot),
        Err(Error::InvalidArgument(message)) if message.contains("does not belong")
    ));
}

#[test]
fn invalid_redaction_areas_are_rejected() {
    let mut doc = PdfDocument::new();
    let mut page = doc.new_page(PAGE_SIZE).unwrap();

    assert!(matches!(
        page.add_redact_annotation(Rect::new(10.0, 10.0, 10.0, 20.0)),
        Err(Error::InvalidArgument(_))
    ));
    assert!(matches!(
        page.add_redact_annotation(Vec::<Quad>::new()),
        Err(Error::InvalidArgument(_))
    ));
    assert!(matches!(
        page.add_redact_annotation(Quad::from(Rect::new(10.0, 10.0, 10.0, 20.0))),
        Err(Error::InvalidArgument(_))
    ));
    assert!(matches!(
        page.add_redact_annotation(Quad::new(
            Point::new(10.0, 10.0),
            Point::new(20.0, 20.0),
            Point::new(30.0, 30.0),
            Point::new(40.0, 40.0),
        )),
        Err(Error::InvalidArgument(_))
    ));
}

#[test]
fn constructor_failure_removes_partial_annotation() {
    let mut doc = PdfDocument::new();
    let mut page = doc.new_page(PAGE_SIZE).unwrap();

    let result = page.add_stamp_annotation(Rect::new(10.0, 10.0, 20.0, 20.0), "bad\0icon");

    assert!(result.is_err());
    assert_eq!(page.annotations().count(), 0);
}

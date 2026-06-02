use mupdf::drawing::{DrawingItem, DrawingType};
use mupdf::pdf::{PdfDocument, PdfObject};
use mupdf::{LineCap, LineJoin, Rect, Size};

fn assert_near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 1e-4,
        "actual={actual}, expected={expected}"
    );
}

fn assert_rect_near(actual: Rect, expected: Rect) {
    assert_near(actual.x0, expected.x0);
    assert_near(actual.y0, expected.y0);
    assert_near(actual.x1, expected.x1);
    assert_near(actual.y1, expected.y1);
}

#[test]
fn extracts_fill_stroke_rect_and_stroked_line() {
    let mut doc = PdfDocument::new();
    let mut page = doc.new_page(Size::new(300.0, 300.0)).unwrap();
    page.insert_contents(
        &mut doc,
        b"q\n1 0 0 RG\n1 1 0 rg\n2 w\n10 20 100 50 re\nB\nQ\n\
          q\n0 0 1 RG\n3 w\n[3 2] 1 d\n20 100 m\n80 100 l\nS\nQ\n",
        true,
    )
    .unwrap();

    let drawings = page.drawings().unwrap();
    assert_eq!(drawings.len(), 2, "drawings: {drawings:#?}");

    let rect_drawing = drawings
        .iter()
        .find(|drawing| {
            matches!(
                drawing.items.as_slice(),
                [DrawingItem::Rect {
                    rect: _,
                    orientation: _
                }]
            )
        })
        .expect("missing rectangle drawing");
    assert_eq!(rect_drawing.drawing_type, DrawingType::FillStroke);
    assert_eq!(rect_drawing.color, Some([1.0, 0.0, 0.0]));
    assert_eq!(rect_drawing.fill, Some([1.0, 1.0, 0.0]));
    assert_eq!(rect_drawing.seqno, 0);
    assert_eq!(rect_drawing.width, Some(2.0));
    assert_eq!(
        rect_drawing.line_cap,
        Some((LineCap::Butt, LineCap::Butt, LineCap::Butt))
    );
    assert_eq!(rect_drawing.line_join, Some(LineJoin::Miter));
    assert_eq!(rect_drawing.close_path, Some(false));
    assert_eq!(rect_drawing.fill_opacity, Some(1.0));
    assert_eq!(rect_drawing.stroke_opacity, Some(1.0));
    let [DrawingItem::Rect { rect, orientation }] = rect_drawing.items.as_slice() else {
        unreachable!();
    };
    assert_rect_near(*rect, Rect::new(10.0, 230.0, 110.0, 280.0));
    assert_eq!(*orientation, 1);

    let line_drawing = drawings
        .iter()
        .find(|drawing| matches!(drawing.items.as_slice(), [DrawingItem::Line(_, _)]))
        .expect("missing line drawing");
    assert_eq!(line_drawing.drawing_type, DrawingType::Stroke);
    assert_eq!(line_drawing.color, Some([0.0, 0.0, 1.0]));
    assert_eq!(line_drawing.seqno, 2);
    assert_eq!(line_drawing.close_path, Some(false));
    assert_eq!(line_drawing.width, Some(3.0));
    let dashes = line_drawing.dashes.as_ref().expect("missing dashes");
    assert_eq!(dashes.dashes.len(), 2);
    assert_near(dashes.dashes[0], 3.0);
    assert_near(dashes.dashes[1], 2.0);
    assert_near(dashes.phase, 1.0);
    let [DrawingItem::Line(start, end)] = line_drawing.items.as_slice() else {
        unreachable!();
    };
    assert_near(start.x, 20.0);
    assert_near(start.y, 200.0);
    assert_near(end.x, 80.0);
    assert_near(end.y, 200.0);
}

#[test]
fn preserves_close_path_and_detects_quads() {
    let mut doc = PdfDocument::new();
    let mut page = doc.new_page(Size::new(100.0, 100.0)).unwrap();
    page.insert_contents(
        &mut doc,
        b"q\n0 1 0 rg\n10 10 m\n50 10 l\n50 50 l\nf\nQ\n\
          q\n0 1 0 rg\n10 60 m\n50 60 l\n50 90 l\nh\nf\nQ\n\
          q\n0 0 1 RG\n10 10 m\n40 20 l\n50 60 l\n5 45 l\n10.00001 10.00001 l\nh\nS\nQ\n",
        true,
    )
    .unwrap();

    let drawings = page.drawings().unwrap();
    assert_eq!(drawings.len(), 3, "drawings: {drawings:#?}");

    assert_eq!(drawings[0].drawing_type, DrawingType::Fill);
    assert_eq!(drawings[0].close_path, None);

    assert_eq!(drawings[1].drawing_type, DrawingType::Fill);
    assert_eq!(drawings[1].close_path, Some(false));

    assert_eq!(drawings[2].drawing_type, DrawingType::Stroke);
    assert!(
        matches!(drawings[2].items.as_slice(), [DrawingItem::Quad(_)]),
        "expected quad item, got {:#?}",
        drawings[2].items
    );
    assert_eq!(drawings[2].close_path, Some(false));
}

#[test]
fn detects_rectangles_with_tiny_coordinate_drift() {
    let mut doc = PdfDocument::new();
    let mut page = doc.new_page(Size::new(100.0, 100.0)).unwrap();
    page.insert_contents(
        &mut doc,
        b"q\n1 0 0 RG\n10 10 m\n50 10.00001 l\n50 50 l\n10.00001 50 l\nh\nS\nQ\n",
        true,
    )
    .unwrap();

    let drawings = page.drawings().unwrap();
    assert_eq!(drawings.len(), 1, "drawings: {drawings:#?}");

    let [DrawingItem::Rect { rect, orientation }] = drawings[0].items.as_slice() else {
        panic!("expected rectangle drawing, got {:#?}", drawings[0].items);
    };
    assert_rect_near(*rect, Rect::new(10.0, 50.0, 50.0, 90.0));
    assert_eq!(*orientation, 1);
}

#[test]
fn closed_rectangle_normalizes_current_point() {
    let mut doc = PdfDocument::new();
    let mut page = doc.new_page(Size::new(100.0, 100.0)).unwrap();
    page.insert_contents(
        &mut doc,
        b"q\n1 0 0 RG\n10 10 m\n50 10 l\n50 50 l\n10 50 l\nh\n10 70 l\nS\nQ\n",
        true,
    )
    .unwrap();

    let drawings = page.drawings().unwrap();
    assert_eq!(drawings.len(), 1, "drawings: {drawings:#?}");

    let [DrawingItem::Rect { rect, orientation }, DrawingItem::Line(start, end)] =
        drawings[0].items.as_slice()
    else {
        panic!(
            "expected rectangle followed by line, got {:#?}",
            drawings[0].items
        );
    };

    assert_rect_near(*rect, Rect::new(10.0, 50.0, 50.0, 90.0));
    assert_eq!(*orientation, 1);
    assert_near(start.x, 10.0);
    assert_near(start.y, 90.0);
    assert_near(end.x, 10.0);
    assert_near(end.y, 30.0);
}

#[test]
fn pdf_page_drawings_ignores_rotation() {
    let mut doc = PdfDocument::new();
    let mut page = doc.new_page(Size::new(100.0, 200.0)).unwrap();
    page.insert_contents(&mut doc, b"q\n1 0 0 RG\n2 w\n10 20 30 40 re\nS\nQ\n", true)
        .unwrap();
    page.set_rotation(90).unwrap();

    let drawings = page.drawings().unwrap();
    assert_eq!(page.rotation().unwrap(), 90);
    assert_eq!(drawings.len(), 1, "drawings: {drawings:#?}");

    let [DrawingItem::Rect { rect, orientation }] = drawings[0].items.as_slice() else {
        panic!("expected rectangle drawing, got {:#?}", drawings[0].items);
    };
    assert_rect_near(*rect, Rect::new(10.0, 140.0, 40.0, 180.0));
    assert_eq!(*orientation, 1);
}

#[test]
fn pdf_page_drawings_preserves_inherited_rotation() {
    let mut doc = PdfDocument::new();
    let mut page = doc.new_page(Size::new(100.0, 200.0)).unwrap();
    page.insert_contents(&mut doc, b"q\n1 0 0 RG\n2 w\n10 20 30 40 re\nS\nQ\n", true)
        .unwrap();

    let mut page_obj = page.object();
    let mut parent = page_obj.get_dict("Parent").unwrap().unwrap();
    page_obj.dict_delete("Rotate").unwrap();
    parent
        .dict_put("Rotate", PdfObject::new_int(90).unwrap())
        .unwrap();

    assert!(page_obj.get_dict("Rotate").unwrap().is_none());
    assert_eq!(page.rotation().unwrap(), 90);

    let drawings = page.drawings().unwrap();

    assert_eq!(page.rotation().unwrap(), 90);
    assert!(page.object().get_dict("Rotate").unwrap().is_none());
    assert_eq!(drawings.len(), 1, "drawings: {drawings:#?}");

    let [DrawingItem::Rect { rect, orientation }] = drawings[0].items.as_slice() else {
        panic!("expected rectangle drawing, got {:#?}", drawings[0].items);
    };
    assert_rect_near(*rect, Rect::new(10.0, 140.0, 40.0, 180.0));
    assert_eq!(*orientation, 1);
}

#[test]
fn pdf_page_drawings_rejects_malformed_rotation_before_mutating() {
    let mut doc = PdfDocument::new();
    let page = doc.new_page(Size::new(100.0, 200.0)).unwrap();
    page.object()
        .dict_put("Rotate", PdfObject::new_int(45).unwrap())
        .unwrap();

    let error = page.drawings().unwrap_err();

    assert!(
        error.to_string().contains("multiple of 90"),
        "unexpected error: {error}"
    );
    let rotate = page.object().get_dict("Rotate").unwrap().unwrap();
    assert_eq!(rotate.as_int().unwrap(), 45);
}

#[test]
fn pdf_page_drawings_rejects_malformed_inherited_rotation_before_mutating() {
    let mut doc = PdfDocument::new();
    let page = doc.new_page(Size::new(100.0, 200.0)).unwrap();

    let mut page_obj = page.object();
    let mut parent = page_obj.get_dict("Parent").unwrap().unwrap();
    page_obj.dict_delete("Rotate").unwrap();
    parent
        .dict_put("Rotate", PdfObject::new_int(45).unwrap())
        .unwrap();

    let error = page.drawings().unwrap_err();

    assert!(
        error.to_string().contains("multiple of 90"),
        "unexpected error: {error}"
    );
    assert!(page.object().get_dict("Rotate").unwrap().is_none());
    let rotate = parent.get_dict("Rotate").unwrap().unwrap();
    assert_eq!(rotate.as_int().unwrap(), 45);
}

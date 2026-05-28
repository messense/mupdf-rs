use crate::support::{assert_snapshot, render_page};
use mupdf::pdf::{PdfDocument, PdfPage};
use mupdf::shape::Shape;
use mupdf::{Point, Rect, Size};

pub mod draw {
    use super::*;
    use mupdf::shape::{FinishOptions, PdfColor};

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn single_line_snapshot() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let rendered = {
            let mut shape = Shape::new(&mut page).unwrap();
            shape
                .draw_line(Point::new(100.0, 100.0), Point::new(400.0, 100.0))
                .unwrap()
                .finish(&Default::default())
                .unwrap()
                .commit(&mut doc, true)
                .unwrap();
            render_page(shape.page())
        };

        assert_snapshot("tests/shape/snapshots/draw_line.png", &rendered);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn rect_fill_stroke_snapshot() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let rendered = {
            let mut shape = Shape::new(&mut page).unwrap();
            shape
                .draw_rect(&Rect::new(50.0, 50.0, 200.0, 150.0))
                .unwrap()
                .finish(&FinishOptions {
                    color: Some(PdfColor::rgb(0.0, 0.0, 0.0)),
                    fill: Some(PdfColor::rgb(1.0, 1.0, 0.0)),
                    width: 2.0,
                    ..Default::default()
                })
                .unwrap()
                .commit(&mut doc, true)
                .unwrap();
            render_page(shape.page())
        };

        assert_snapshot("tests/shape/snapshots/rect_fill_stroke.png", &rendered);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn triangle_closed_snapshot() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let rendered = {
            let mut shape = Shape::new(&mut page).unwrap();
            shape
                .draw_polyline(&[
                    Point::new(100.0, 100.0),
                    Point::new(200.0, 300.0),
                    Point::new(300.0, 100.0),
                ])
                .unwrap()
                .finish(&FinishOptions {
                    close_path: true,
                    width: 1.5,
                    ..Default::default()
                })
                .unwrap()
                .commit(&mut doc, true)
                .unwrap();
            render_page(shape.page())
        };

        assert_snapshot("tests/shape/snapshots/triangle_closed.png", &rendered);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn overlay_preserves_existing_snapshot() {
        let mut doc = PdfDocument::from_bytes(include_bytes!("../files/dummy.pdf")).unwrap();
        let mut page = PdfPage::try_from(doc.load_page(0).unwrap()).unwrap();
        let rendered = {
            let mut shape = Shape::new(&mut page).unwrap();
            shape
                .draw_line(Point::new(40.0, 420.0), Point::new(555.0, 420.0))
                .unwrap()
                .finish(&FinishOptions {
                    color: Some(PdfColor::rgb(1.0, 0.0, 0.0)),
                    width: 4.0,
                    ..Default::default()
                })
                .unwrap()
                .commit(&mut doc, true)
                .unwrap();
            render_page(shape.page())
        };

        assert_snapshot(
            "tests/shape/snapshots/overlay_preserves_existing.png",
            &rendered,
        );
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn bezier_rotated_snapshot() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        page.set_rotation(90).unwrap();

        let rendered = {
            let mut shape = Shape::new(&mut page).unwrap();
            shape
                .draw_bezier(
                    Point::new(0.0, 0.0),
                    Point::new(50.0, 150.0),
                    Point::new(150.0, 150.0),
                    Point::new(200.0, 0.0),
                )
                .unwrap()
                .finish(&FinishOptions {
                    width: 2.0,
                    ..Default::default()
                })
                .unwrap()
                .commit(&mut doc, true)
                .unwrap();
            render_page(shape.page())
        };

        assert_snapshot("tests/shape/snapshots/bezier_rotated.png", &rendered);
    }
}

use std::path::Path;

use mupdf::pdf::{PdfDocument, PdfPage};
use mupdf::{
    Colorspace, FinishOptions, Image, ImageFormat, Matrix, PdfColor, Point, Quad, Rect, RectRadius,
    Shape, Size,
};

fn render_page(page: &PdfPage) -> mupdf::Pixmap {
    page.to_pixmap(
        &Matrix::new_scale(1.0, 1.0),
        &Colorspace::device_rgb(),
        false,
        true,
    )
    .unwrap()
}

fn assert_snapshot(snapshot: &str, rendered: &mupdf::Pixmap) {
    if std::env::var_os("UPDATE_SHAPE_SNAPSHOTS").is_some() {
        rendered.save_as(snapshot, ImageFormat::PNG).unwrap();
    }

    assert!(
        Path::new(snapshot).exists(),
        "missing snapshot {snapshot}; rerun with UPDATE_SHAPE_SNAPSHOTS=1"
    );
    let expected = Image::from_file(snapshot).unwrap().to_pixmap().unwrap();
    assert_eq!(rendered.width(), expected.width());
    assert_eq!(rendered.height(), expected.height());
    assert_eq!(rendered.n(), expected.n());
    assert_eq!(rendered.samples(), expected.samples());
}

pub mod drawing {
    use super::*;

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn oval_from_square_rect_matches_circle_render() {
        let render_oval = {
            let mut doc = PdfDocument::new();
            let mut page = doc.new_page(Size::A4).unwrap();
            let mut shape = Shape::new(&mut page).unwrap();
            shape
                .draw_oval(Rect::new(50.0, 50.0, 150.0, 150.0))
                .unwrap()
                .finish(&FinishOptions {
                    color: None,
                    fill: Some(PdfColor::rgb(0.0, 0.0, 1.0)),
                    ..Default::default()
                })
                .unwrap()
                .commit(&mut doc, true)
                .unwrap();
            render_page(shape.page())
        };

        let render_circle = {
            let mut doc = PdfDocument::new();
            let mut page = doc.new_page(Size::A4).unwrap();
            let mut shape = Shape::new(&mut page).unwrap();
            shape
                .draw_circle(Point::new(100.0, 100.0), 50.0)
                .unwrap()
                .finish(&FinishOptions {
                    color: None,
                    fill: Some(PdfColor::rgb(0.0, 0.0, 1.0)),
                    ..Default::default()
                })
                .unwrap()
                .commit(&mut doc, true)
                .unwrap();
            render_page(shape.page())
        };

        assert_eq!(render_oval.width(), render_circle.width());
        assert_eq!(render_oval.height(), render_circle.height());
        assert_eq!(render_oval.n(), render_circle.n());
        assert_eq!(render_oval.samples(), render_circle.samples());
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn sector_circle_oval_snapshot() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let rendered = {
            let mut shape = Shape::new(&mut page).unwrap();
            shape
                .draw_sector(
                    Point::new(100.0, 120.0),
                    Point::new(145.0, 120.0),
                    120.0,
                    true,
                )
                .unwrap()
                .finish(&FinishOptions {
                    color: Some(PdfColor::rgb(1.0, 0.0, 0.0)),
                    width: 2.0,
                    ..Default::default()
                })
                .unwrap()
                .draw_circle(Point::new(250.0, 120.0), 30.0)
                .unwrap()
                .finish(&FinishOptions {
                    color: Some(PdfColor::rgb(0.0, 0.0, 0.0)),
                    fill: Some(PdfColor::rgb(0.0, 0.0, 1.0)),
                    width: 1.0,
                    ..Default::default()
                })
                .unwrap()
                .draw_oval(Rect::new(340.0, 80.0, 470.0, 160.0))
                .unwrap()
                .finish(&FinishOptions {
                    color: Some(PdfColor::rgb(0.0, 0.5, 0.0)),
                    width: 2.0,
                    ..Default::default()
                })
                .unwrap()
                .draw_oval(Quad::new(
                    Point::new(80.0, 260.0),
                    Point::new(190.0, 240.0),
                    Point::new(100.0, 340.0),
                    Point::new(210.0, 320.0),
                ))
                .unwrap()
                .finish(&FinishOptions {
                    color: Some(PdfColor::rgb(0.6, 0.0, 0.8)),
                    width: 2.0,
                    ..Default::default()
                })
                .unwrap()
                .commit(&mut doc, true)
                .unwrap();
            render_page(shape.page())
        };

        assert_snapshot("tests/shape/snapshots/sector_circle_oval.png", &rendered);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn quad_snapshot() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let rendered = {
            let mut shape = Shape::new(&mut page).unwrap();
            shape
                .draw_quad(Quad::new(
                    Point::new(300.0, 250.0),
                    Point::new(450.0, 220.0),
                    Point::new(330.0, 360.0),
                    Point::new(480.0, 330.0),
                ))
                .unwrap()
                .finish(&FinishOptions {
                    color: Some(PdfColor::rgb(0.0, 0.0, 0.0)),
                    width: 3.0,
                    ..Default::default()
                })
                .unwrap()
                .commit(&mut doc, true)
                .unwrap();
            render_page(shape.page())
        };

        assert_snapshot("tests/shape/snapshots/quad.png", &rendered);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn rect_quad_snapshot() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let rendered = {
            let mut shape = Shape::new(&mut page).unwrap();
            shape
                .draw_rect(&Rect::new(60.0, 70.0, 170.0, 140.0))
                .unwrap()
                .draw_rect_with_radius(&Rect::new(220.0, 70.0, 340.0, 150.0), 8.0)
                .unwrap()
                .draw_rect_with_radius(&Rect::new(390.0, 70.0, 510.0, 130.0), 80.0)
                .unwrap()
                .draw_quad(Quad::new(
                    Point::new(90.0, 250.0),
                    Point::new(230.0, 220.0),
                    Point::new(120.0, 350.0),
                    Point::new(260.0, 320.0),
                ))
                .unwrap()
                .finish(&FinishOptions {
                    color: Some(PdfColor::rgb(0.0, 0.0, 0.0)),
                    width: 1.0,
                    ..Default::default()
                })
                .unwrap()
                .commit(&mut doc, true)
                .unwrap();
            render_page(shape.page())
        };

        assert_snapshot("tests/shape/snapshots/rect_quad.png", &rendered);

        let mut absolute_doc = PdfDocument::new();
        let mut absolute_page = absolute_doc.new_page(Size::A4).unwrap();
        let absolute = {
            let mut shape = Shape::new(&mut absolute_page).unwrap();
            shape
                .draw_rect_with_radius(&Rect::new(0.0, 0.0, 40.0, 20.0), 10.0)
                .unwrap()
                .finish(&FinishOptions::default())
                .unwrap()
                .commit(&mut absolute_doc, true)
                .unwrap();
            render_page(shape.page())
        };

        let mut fractional_doc = PdfDocument::new();
        let mut fractional_page = fractional_doc.new_page(Size::A4).unwrap();
        let fractional = {
            let mut shape = Shape::new(&mut fractional_page).unwrap();
            shape
                .draw_rect_with_radius(
                    &Rect::new(0.0, 0.0, 40.0, 20.0),
                    RectRadius::fractional(0.5, 1.0),
                )
                .unwrap()
                .finish(&FinishOptions::default())
                .unwrap()
                .commit(&mut fractional_doc, true)
                .unwrap();
            render_page(shape.page())
        };

        assert_eq!(absolute.samples(), fractional.samples());
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn wave_primitives_snapshot() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let rendered = {
            let mut shape = Shape::new(&mut page).unwrap();
            shape
                .draw_zigzag(Point::new(60.0, 80.0), Point::new(270.0, 80.0), 6.0)
                .unwrap()
                .draw_zigzag(Point::new(330.0, 70.0), Point::new(520.0, 160.0), 6.0)
                .unwrap()
                .draw_squiggle(Point::new(60.0, 250.0), Point::new(270.0, 250.0), 6.0)
                .unwrap()
                .draw_squiggle(Point::new(330.0, 230.0), Point::new(520.0, 340.0), 6.0)
                .unwrap()
                .finish(&FinishOptions {
                    color: Some(PdfColor::rgb(0.0, 0.0, 0.0)),
                    width: 0.75,
                    ..Default::default()
                })
                .unwrap()
                .commit(&mut doc, true)
                .unwrap();
            render_page(shape.page())
        };

        assert_snapshot("tests/shape/snapshots/waves.png", &rendered);
    }
}

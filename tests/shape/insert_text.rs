#![cfg(not(target_arch = "wasm32"))]

use mupdf::pdf::{PdfDocument, PdfPage};
use mupdf::{
    Colorspace, Image, ImageFormat, Matrix, PdfColor, Point, Rect, Shape, Size, TextAlign,
    TextOptions, TextboxOptions,
};
use std::path::Path;

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

fn render_text(point: Point, text: &str, opts: &TextOptions) -> mupdf::Pixmap {
    let mut doc = PdfDocument::new();
    let mut page = doc.new_page(Size::A4).unwrap();
    let rendered = {
        let mut shape = Shape::new(&mut page).unwrap();
        shape
            .insert_text(point, text, opts)
            .unwrap()
            .commit(&mut doc, true)
            .unwrap();
        render_page(shape.page())
    };
    rendered
}

fn render_textbox(rect: Rect, text: &str, opts: &TextboxOptions) -> (mupdf::Pixmap, f32) {
    let mut doc = PdfDocument::new();
    let mut page = doc.new_page(Size::A4).unwrap();
    let (rendered, deficit) = {
        let mut shape = Shape::new(&mut page).unwrap();
        let deficit = shape.insert_textbox(rect, text, opts).unwrap();
        shape.commit(&mut doc, true).unwrap();
        (render_page(shape.page()), deficit)
    };
    (rendered, deficit)
}

#[test]
fn insert_text_at_point_snapshot() {
    let rendered = render_text(
        Point::new(100.0, 150.0),
        "Hello Shape",
        &TextOptions::default(),
    );

    assert_snapshot("tests/shape/snapshots/text_at_point.png", &rendered);
}

#[test]
fn insert_text_multiline_snapshot() {
    let rendered = render_text(
        Point::new(80.0, 120.0),
        "line1\nline2\nline3",
        &TextOptions {
            fontsize: 18.0,
            lineheight: 1.25,
            color: Some(PdfColor::rgb(0.0, 0.0, 1.0)),
            ..Default::default()
        },
    );

    assert_snapshot("tests/shape/snapshots/text_multiline.png", &rendered);
}

#[test]
fn insert_text_rotate_90_snapshot() {
    let rendered = render_text(
        Point::new(300.0, 500.0),
        "Rot90",
        &TextOptions {
            fontsize: 24.0,
            rotate: 90,
            color: Some(PdfColor::rgb(1.0, 0.0, 0.0)),
            ..Default::default()
        },
    );

    assert_snapshot("tests/shape/snapshots/text_rot_90.png", &rendered);
}

#[test]
fn insert_text_rotate_180_snapshot() {
    let rendered = render_text(
        Point::new(350.0, 300.0),
        "Rot180",
        &TextOptions {
            fontsize: 24.0,
            rotate: 180,
            color: Some(PdfColor::rgb(0.0, 0.5, 0.0)),
            ..Default::default()
        },
    );

    assert_snapshot("tests/shape/snapshots/text_rot_180.png", &rendered);
}

#[test]
fn insert_text_rotate_270_snapshot() {
    let rendered = render_text(
        Point::new(300.0, 250.0),
        "Rot270",
        &TextOptions {
            fontsize: 24.0,
            rotate: 270,
            color: Some(PdfColor::rgb(0.6, 0.0, 0.8)),
            ..Default::default()
        },
    );

    assert_snapshot("tests/shape/snapshots/text_rot_270.png", &rendered);
}

#[test]
fn insert_textbox_left_snapshot() {
    let (rendered, deficit) = render_textbox(
        Rect::new(80.0, 100.0, 300.0, 210.0),
        "The quick brown fox jumps over the lazy dog.",
        &TextboxOptions {
            fontsize: 16.0,
            lineheight: 1.1,
            color: Some(PdfColor::rgb(0.0, 0.0, 0.0)),
            ..Default::default()
        },
    );

    assert!(deficit >= 0.0);
    assert_snapshot("tests/shape/snapshots/textbox_left.png", &rendered);
}

#[test]
fn insert_textbox_center_snapshot() {
    let (rendered, _deficit) = render_textbox(
        Rect::new(80.0, 230.0, 320.0, 340.0),
        "Centered text wraps inside this rectangle.",
        &TextboxOptions {
            fontsize: 16.0,
            lineheight: 1.1,
            align: TextAlign::Center,
            color: Some(PdfColor::rgb(0.0, 0.2, 0.8)),
            ..Default::default()
        },
    );

    assert_snapshot("tests/shape/snapshots/textbox_center.png", &rendered);
}

#[test]
fn insert_textbox_right_snapshot() {
    let (rendered, _deficit) = render_textbox(
        Rect::new(80.0, 360.0, 320.0, 470.0),
        "Right aligned words wrap cleanly.",
        &TextboxOptions {
            fontsize: 16.0,
            lineheight: 1.1,
            align: TextAlign::Right,
            color: Some(PdfColor::rgb(0.7, 0.0, 0.0)),
            ..Default::default()
        },
    );

    assert_snapshot("tests/shape/snapshots/textbox_right.png", &rendered);
}

#[test]
fn insert_textbox_rotate_90_snapshot() {
    let (rendered, _deficit) = render_textbox(
        Rect::new(360.0, 100.0, 460.0, 320.0),
        "Rotated ninety degrees and wrapped.",
        &TextboxOptions {
            fontsize: 14.0,
            lineheight: 1.1,
            rotate: 90,
            color: Some(PdfColor::rgb(0.0, 0.5, 0.0)),
            ..Default::default()
        },
    );

    assert_snapshot("tests/shape/snapshots/textbox_rot_90.png", &rendered);
}

#[test]
fn insert_textbox_rotate_180_snapshot() {
    let (rendered, _deficit) = render_textbox(
        Rect::new(360.0, 340.0, 540.0, 450.0),
        "Rotated one hundred eighty degrees.",
        &TextboxOptions {
            fontsize: 14.0,
            lineheight: 1.1,
            rotate: 180,
            color: Some(PdfColor::rgb(0.5, 0.0, 0.5)),
            ..Default::default()
        },
    );

    assert_snapshot("tests/shape/snapshots/textbox_rot_180.png", &rendered);
}

#[test]
fn insert_textbox_rotate_270_snapshot() {
    let (rendered, _deficit) = render_textbox(
        Rect::new(120.0, 500.0, 220.0, 730.0),
        "Rotated two hundred seventy degrees.",
        &TextboxOptions {
            fontsize: 14.0,
            lineheight: 1.1,
            rotate: 270,
            color: Some(PdfColor::rgb(0.8, 0.4, 0.0)),
            ..Default::default()
        },
    );

    assert_snapshot("tests/shape/snapshots/textbox_rot_270.png", &rendered);
}

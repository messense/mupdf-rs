#![cfg(not(target_arch = "wasm32"))]

use crate::support::{assert_snapshot, render_page};
use mupdf::pdf::PdfDocument;
use mupdf::shape::{PdfColor, Shape, TextAlign, TextboxOptions};
use mupdf::{Rect, Size};

const JUSTIFY_TEXT: &str = concat!(
    "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ",
    "Donec varius ligula sit amet libero pulvinar, vel finibus arcu pretium.\n",
    "Integer posuere neque sed erat facilisis, vitae placerat massa posuere."
);

#[test]
fn insert_textbox_justify_snapshot() {
    let mut doc = PdfDocument::new();
    let mut page = doc.new_page(Size::A4).unwrap();
    let rendered = {
        let mut shape = Shape::new(&mut page).unwrap();
        let narrow_deficit = shape
            .insert_textbox(
                Rect::new(60.0, 80.0, 255.0, 260.0),
                JUSTIFY_TEXT,
                &TextboxOptions {
                    fontsize: 11.0,
                    lineheight: 1.15,
                    align: TextAlign::Justify,
                    color: Some(PdfColor::rgb(0.0, 0.0, 0.0)),
                    ..Default::default()
                },
            )
            .unwrap();
        let wide_deficit = shape
            .insert_textbox(
                Rect::new(300.0, 80.0, 540.0, 260.0),
                JUSTIFY_TEXT,
                &TextboxOptions {
                    fontsize: 11.0,
                    lineheight: 1.15,
                    align: TextAlign::Justify,
                    color: Some(PdfColor::rgb(0.0, 0.0, 0.5)),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(narrow_deficit >= 0.0);
        assert!(wide_deficit >= 0.0);
        shape.commit(&mut doc, true).unwrap();
        render_page(shape.page())
    };

    assert_snapshot("tests/shape/snapshots/textbox_justify.png", &rendered);
}

#[test]
fn insert_textbox_justify_rotate_90_snapshot() {
    let mut doc = PdfDocument::new();
    let mut page = doc.new_page(Size::A4).unwrap();
    let rendered = {
        let mut shape = Shape::new(&mut page).unwrap();
        let deficit = shape
            .insert_textbox(
                Rect::new(120.0, 320.0, 290.0, 760.0),
                JUSTIFY_TEXT,
                &TextboxOptions {
                    fontsize: 11.0,
                    lineheight: 1.15,
                    align: TextAlign::Justify,
                    rotate: 90,
                    color: Some(PdfColor::rgb(0.0, 0.35, 0.0)),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(deficit >= 0.0);
        shape.commit(&mut doc, true).unwrap();
        render_page(shape.page())
    };

    assert_snapshot(
        "tests/shape/snapshots/textbox_justify_rot_90.png",
        &rendered,
    );
}

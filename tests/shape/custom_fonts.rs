#![cfg(not(target_arch = "wasm32"))]

use crate::support::{assert_snapshot, render_page};
use mupdf::pdf::PdfDocument;
use mupdf::shape::{PdfColor, Shape, TextOptions};
use mupdf::{Point, Size};

const CUSTOM_FONT_BYTES: &[u8] = include_bytes!("../files/custom.ttf");
const CUSTOM_FONT_SNAPSHOT: &str = "tests/shape/snapshots/text_custom_font.png";

pub mod m5 {
    pub mod fonts {
        use super::super::*;

        #[test]
        fn custom_ttf_snapshot() {
            let mut doc = PdfDocument::new();
            let mut page = doc.new_page(Size::A4).unwrap();
            let rendered = {
                let mut shape = Shape::new(&mut page).unwrap();
                shape
                    .insert_text(
                        Point::new(72.0, 144.0),
                        "Custom font",
                        &TextOptions {
                            fontsize: 32.0,
                            fontname: "PdfCustomSnapshot".to_owned(),
                            fontfile: Some(CUSTOM_FONT_BYTES),
                            color: Some(PdfColor::rgb(0.0, 0.0, 0.0)),
                            ..Default::default()
                        },
                    )
                    .unwrap()
                    .commit(&mut doc, true)
                    .unwrap();
                render_page(shape.page())
            };

            assert_snapshot(CUSTOM_FONT_SNAPSHOT, &rendered);
        }
    }
}

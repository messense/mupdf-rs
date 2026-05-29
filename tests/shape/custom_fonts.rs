#![cfg(not(target_arch = "wasm32"))]

use crate::support::{assert_snapshot, render_page};
use mupdf::pdf::PdfDocument;
use mupdf::shape::{PdfColor, Shape, TextOptions};
use mupdf::{Font, Point, Size};

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

        #[test]
        fn custom_ttf_unicode_text_uses_font_glyph_ids() {
            let font = Font::from_bytes("PdfCustomUnicode", CUSTOM_FONT_BYTES).unwrap();
            let (ch, glyph) = ['Ω', 'Ж', '✓', '中', 'あ']
                .into_iter()
                .filter_map(|ch| {
                    font.encode_character(ch as i32)
                        .ok()
                        .filter(|glyph| *glyph > 0 && *glyph != ch as i32)
                        .map(|glyph| (ch, glyph))
                })
                .next()
                .expect("custom font should contain at least one non-ASCII test glyph");

            let mut doc = PdfDocument::new();
            let mut page = doc.new_page(Size::A4).unwrap();
            let mut shape = Shape::new(&mut page).unwrap();
            shape
                .insert_text(
                    Point::new(72.0, 144.0),
                    &ch.to_string(),
                    &TextOptions {
                        fontname: "PdfCustomUnicode".to_owned(),
                        fontfile: Some(CUSTOM_FONT_BYTES),
                        ..Default::default()
                    },
                )
                .unwrap();

            let expected = format!("[<{:04x}>] TJ", glyph as u32);
            let codepoint_encoding = format!("[<{:04x}>] TJ", ch as u32);
            assert!(
                shape.text_cont().contains(&expected),
                "{}",
                shape.text_cont()
            );
            assert!(
                !shape.text_cont().contains(&codepoint_encoding),
                "{}",
                shape.text_cont()
            );
        }
    }
}

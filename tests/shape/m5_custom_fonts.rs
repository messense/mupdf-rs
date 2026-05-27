#![cfg(not(target_arch = "wasm32"))]

use std::path::Path;

use mupdf::pdf::{PdfDocument, PdfPage};
use mupdf::{Colorspace, Image, ImageFormat, Matrix, PdfColor, Point, Shape, Size, TextOptions};

const CUSTOM_FONT_BYTES: &[u8] = include_bytes!("../files/custom.ttf");
const CUSTOM_FONT_SNAPSHOT: &str = "tests/shape/snapshots/text_custom_font.png";

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

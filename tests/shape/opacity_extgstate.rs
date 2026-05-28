#![cfg(not(target_arch = "wasm32"))]

use std::path::Path;

use mupdf::pdf::{PdfDocument, PdfPage};
use mupdf::{Colorspace, FinishOptions, Image, ImageFormat, Matrix, PdfColor, Rect, Shape, Size};

const OPACITY_OVERLAY_SNAPSHOT: &str = "tests/shape/snapshots/opacity_overlay.png";

fn render_page(page: &PdfPage) -> mupdf::Pixmap {
    page.to_pixmap(
        &Matrix::new_scale(1.0, 1.0),
        &Colorspace::device_rgb(),
        false,
        true,
    )
    .unwrap()
}

fn pixel_rgb(pixmap: &mupdf::Pixmap, x: u32, y: u32) -> [u8; 3] {
    let n = pixmap.n() as usize;
    assert_eq!(n, 3);
    let index = ((y * pixmap.width() + x) as usize) * n;
    let samples = pixmap.samples();
    [samples[index], samples[index + 1], samples[index + 2]]
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
    pub mod opacity {
        use super::super::*;

        #[test]
        fn opacity_overlay_snapshot() {
            let mut doc = PdfDocument::new();
            let mut page = doc.new_page(Size::A4).unwrap();
            let rendered = {
                let mut shape = Shape::new(&mut page).unwrap();
                shape
                    .draw_rect(&Rect::new(0.0, 0.0, 300.0, 300.0))
                    .unwrap()
                    .finish(&FinishOptions {
                        color: None,
                        fill: Some(PdfColor::rgb(0.0, 0.0, 1.0)),
                        ..Default::default()
                    })
                    .unwrap()
                    .draw_rect(&Rect::new(50.0, 50.0, 250.0, 250.0))
                    .unwrap()
                    .finish(&FinishOptions {
                        color: None,
                        fill: Some(PdfColor::rgb(1.0, 0.0, 0.0)),
                        fill_opacity: Some(0.4),
                        ..Default::default()
                    })
                    .unwrap()
                    .commit(&mut doc, true)
                    .unwrap();
                render_page(shape.page())
            };

            let center = pixel_rgb(&rendered, 150, 150);
            assert!(
                center[0].abs_diff(102) <= 4
                    && center[1] <= 4
                    && center[2].abs_diff(153) <= 4,
                "expected red at 40% opacity over blue to blend to approximately (102, 0, 153), got {center:?}"
            );

            assert_snapshot(OPACITY_OVERLAY_SNAPSHOT, &rendered);
        }
    }
}

#![cfg(not(target_arch = "wasm32"))]

use crate::support::{assert_snapshot, render_page};
use mupdf::pdf::PdfDocument;
use mupdf::shape::{FinishOptions, PdfColor, Shape};
use mupdf::{Rect, Size};

const OPACITY_OVERLAY_SNAPSHOT: &str = "tests/shape/snapshots/opacity_overlay.png";

fn pixel_rgb(pixmap: &mupdf::Pixmap, x: u32, y: u32) -> [u8; 3] {
    let n = pixmap.n() as usize;
    assert_eq!(n, 3);
    let index = ((y * pixmap.width() + x) as usize) * n;
    let samples = pixmap.samples();
    [samples[index], samples[index + 1], samples[index + 2]]
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

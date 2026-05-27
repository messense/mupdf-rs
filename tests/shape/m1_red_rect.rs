use std::path::Path;

use mupdf::pdf::{PdfDocument, PdfPage};
use mupdf::{Colorspace, Image, ImageFormat, Matrix};

const SNAPSHOT: &str = "tests/shape/snapshots/m1_red_rect.png";
const RED_RECT_STREAM: &[u8] = b"q 1 0 0 rg 250 350 100 100 re f Q\n";

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

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn m1_red_rect_snapshot() {
    let mut doc = PdfDocument::from_bytes(include_bytes!("../files/dummy.pdf")).unwrap();
    let mut page = PdfPage::try_from(doc.load_page(0).unwrap()).unwrap();
    let baseline = render_page(&page);

    page.insert_contents(&mut doc, RED_RECT_STREAM, true)
        .unwrap();
    let rendered = render_page(&page);

    let center = pixel_rgb(&rendered, 300, 442);
    assert!(
        center[0].abs_diff(255) <= 4 && center[1] <= 4 && center[2] <= 4,
        "expected center pixel to be red, got {center:?}"
    );
    assert_eq!(
        pixel_rgb(&rendered, 20, 20),
        pixel_rgb(&baseline, 20, 20),
        "pixel outside the rectangle changed"
    );

    if std::env::var_os("UPDATE_SHAPE_SNAPSHOTS").is_some() {
        rendered.save_as(SNAPSHOT, ImageFormat::PNG).unwrap();
    }

    assert!(
        Path::new(SNAPSHOT).exists(),
        "missing snapshot {SNAPSHOT}; rerun with UPDATE_SHAPE_SNAPSHOTS=1"
    );
    let expected = Image::from_file(SNAPSHOT).unwrap().to_pixmap().unwrap();
    assert_eq!(rendered.width(), expected.width());
    assert_eq!(rendered.height(), expected.height());
    assert_eq!(rendered.n(), expected.n());
    assert_eq!(rendered.samples(), expected.samples());
}

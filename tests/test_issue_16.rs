use mupdf::{Colorspace, ImageFormat, Matrix, PdfDocument};

#[test]
fn test_pixmap_to_png() {
    let document = PdfDocument::open("tests/files/dummy.pdf").unwrap();
    let page = document.load_page(0).unwrap();
    let matrix = Matrix::new_scale(72f32 / 72f32, 72f32 / 72f32);
    let pixmap = page
        .to_pixmap(&matrix, &Colorspace::device_rgb(), 0.0, true)
        .unwrap();
    pixmap
        .save_as("tests/output/test.png", ImageFormat::PNG)
        .unwrap();
}

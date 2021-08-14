use mupdf::pdf::PdfDocument;
use mupdf::{Colorspace, Error, ImageFormat, Matrix, TextPageOptions};

#[test]
fn test_issue_16_pixmap_to_png() {
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

#[test]
fn test_issue_27_flatten() {
    let doc = PdfDocument::open("tests/files/dummy.pdf").unwrap();
    let blocks = doc
        .pages()
        .unwrap()
        .map(|page| Ok(page?.to_text_page(TextPageOptions::PRESERVE_LIGATURES)?))
        .collect::<Result<Vec<_>, Error>>()
        .unwrap()
        .into_iter()
        .map(|text_page| text_page.blocks())
        .flatten()
        .collect::<Vec<_>>();
    assert!(!blocks.is_empty());
}

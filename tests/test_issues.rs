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
    let pages = doc
        .pages()
        .unwrap()
        .map(|page| page?.to_text_page(TextPageOptions::PRESERVE_LIGATURES))
        .collect::<Result<Vec<_>, Error>>()
        .unwrap();
    // The original code from the issue doesn't compile anymore since `pages` is required to hold
    // ownership.
    let blocks = pages
        .iter()
        .flat_map(|text_page| text_page.blocks())
        .collect::<Vec<_>>();
    assert!(!blocks.is_empty());
}

#[test]
fn test_issue_43_malloc() {
    const IDENTITY: mupdf::Matrix = mupdf::Matrix {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    let density = 300;
    let height = 1500;
    let options = format!("resolution={},height={}", density, height);

    let mut writer = mupdf::document_writer::DocumentWriter::new(
        "tests/output/issue_43.png",
        "png",
        options.as_str(),
    )
    .unwrap();
    let doc = mupdf::document::Document::open("tests/files/dummy.pdf").unwrap();

    for _ in 0..2 {
        let page0 = doc.load_page(0).unwrap();
        let mediabox = page0.bounds().unwrap();
        let device = writer.begin_page(mediabox).unwrap();
        page0.run(&device, &IDENTITY).unwrap();
        writer.end_page(device).unwrap();
    }
}

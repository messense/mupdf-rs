use mupdf::pdf::PdfDocument;
use mupdf::{Error, TextPageFlags};

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_issue_16_pixmap_to_png() {
    let document = PdfDocument::from_bytes(include_bytes!("../tests/files/dummy.pdf")).unwrap();
    let page = document.load_page(0).unwrap();
    let matrix = mupdf::Matrix::new_scale(72f32 / 72f32, 72f32 / 72f32);
    let pixmap = page
        .to_pixmap(&matrix, &mupdf::Colorspace::device_rgb(), false, true)
        .unwrap();
    pixmap
        .save_as("tests/output/test.png", mupdf::ImageFormat::PNG)
        .unwrap();
}

#[test]
fn test_issue_27_flatten() {
    let doc = PdfDocument::from_bytes(include_bytes!("../tests/files/dummy.pdf")).unwrap();
    let pages = doc
        .pages()
        .unwrap()
        .map(|page| page?.to_text_page(TextPageFlags::PRESERVE_LIGATURES))
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

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_issue_43_malloc() {
    let density = 300;
    let height = 1500;
    let options = format!("resolution={},height={}", density, height);

    let mut writer =
        mupdf::DocumentWriter::new("tests/output/issue_43.png", "png", options.as_str()).unwrap();
    let doc = mupdf::Document::from_bytes(include_bytes!("../tests/files/dummy.pdf"), "").unwrap();

    for _ in 0..2 {
        let page0 = doc.load_page(0).unwrap();
        let mediabox = page0.bounds().unwrap();
        let device = writer.begin_page(mediabox).unwrap();
        page0.run(&device, &mupdf::Matrix::IDENTITY).unwrap();
        writer.end_page(device).unwrap();
    }
}

#[test]
fn test_issue_60_display_list() {
    let doc = PdfDocument::from_bytes(include_bytes!("../tests/files/p11.pdf")).unwrap();
    let num_pages = doc.page_count().unwrap();
    println!("Document has {} page(s)", num_pages);

    let _display_list: Vec<(usize, mupdf::DisplayList)> = doc
        .pages()
        .unwrap()
        .enumerate()
        .map(|(index, p)| {
            let display = p.unwrap().to_display_list(true).unwrap();
            (index, display)
        })
        .collect();
}

#[test]
fn test_issue_86_invalid_utf8() {
    let doc = PdfDocument::from_bytes(include_bytes!("../tests/files/utf8-error-on-this-file.pdf"))
        .unwrap();
    for (idx, page) in doc.pages().unwrap().enumerate() {
        let page = page.unwrap();
        let text_page = page.to_text_page(TextPageFlags::empty()).unwrap();

        let text = text_page.to_text();
        assert!(text.is_ok());
        println!("page: {idx}, text: {}", text.unwrap());

        let json = text_page.to_json(1.0);
        assert!(json.is_ok());

        // Validate JSON parsing
        let parsed_json: Result<serde_json::Value, _> = serde_json::from_str(&json.unwrap());
        assert!(parsed_json.is_ok());
    }
}

#[test]
#[cfg(feature = "serde")]
fn test_issue_i32_box() {
    let doc = PdfDocument::from_bytes(include_bytes!("../tests/files/i32-box.pdf")).unwrap();
    for (idx, page) in doc.pages().unwrap().enumerate() {
        let page = page.unwrap();
        let text_page = page.to_text_page(TextPageFlags::empty()).unwrap();

        let text = text_page.to_text();
        assert!(text.is_ok());
        println!("page: {idx}, text: {}", text.unwrap());

        let json = text_page.to_json(1.0);
        assert!(json.is_ok());

        let stext_page: Result<mupdf::page::StextPage, _> =
            serde_json::from_str(json.unwrap().as_str());
        assert!(stext_page.is_ok());
    }
}

#[test]
fn test_issue_no_json() {
    let doc = PdfDocument::from_bytes(include_bytes!("../tests/files/no-json.pdf")).unwrap();
    let page = doc.load_page(0).unwrap();
    let text_page = page.to_text_page(TextPageFlags::empty()).unwrap();
    let json = text_page.to_json(1.0);
    assert!(json.is_err());
}

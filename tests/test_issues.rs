use mupdf::pdf::{PdfDocument, PdfObject};
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
    let options = format!("resolution={density},height={height}");

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
    println!("Document has {num_pages} page(s)");

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

// Regression tests for issue #207: `as_string`/`as_name`/`as_bytes` returned
// references into MuPDF's resolved (xref-backed) object, which `delete_object`
// can free. The returned values must therefore be *owned* copies that survive
// deletion of the backing object.
//
// Written with `.as_bytes()`/`.as_slice()` so they compile against both the old
// borrowed API and the new owned API.

#[test]
fn test_as_string_indirect_delete() {
    let mut pdf = PdfDocument::new();
    let original = "A".repeat(1000);

    let string_obj = PdfObject::new_string(&original).unwrap();
    let indirect = pdf.add_object(&string_obj).unwrap();
    let obj_num = indirect.as_indirect().unwrap();
    let s = indirect.as_string().unwrap();
    drop(string_obj);
    pdf.delete_object(obj_num).unwrap();
    // Reclaim the freed buffer so a dangling read observes garbage, not the
    // original bytes (makes the unsoundness surface deterministically).
    for _ in 0..32 {
        let _ = PdfObject::new_string(&"B".repeat(1000));
    }
    assert_eq!(s.as_bytes(), original.as_bytes());
}

#[test]
fn test_as_name_indirect_delete() {
    let mut pdf = PdfDocument::new();
    let original = "A".repeat(100);

    let name_obj = PdfObject::new_name(&original).unwrap();
    let indirect = pdf.add_object(&name_obj).unwrap();
    let obj_num = indirect.as_indirect().unwrap();
    let name = indirect.as_name().unwrap();
    drop(name_obj);
    pdf.delete_object(obj_num).unwrap();
    for _ in 0..32 {
        let _ = PdfObject::new_name(&"B".repeat(100));
    }
    assert_eq!(&name[..], original.as_bytes());
}

#[test]
fn test_as_bytes_owned_indirect_delete() {
    let mut pdf = PdfDocument::new();
    let original = "A".repeat(1000);

    let string_obj = PdfObject::new_string(&original).unwrap();
    let indirect = pdf.add_object(&string_obj).unwrap();
    let obj_num = indirect.as_indirect().unwrap();
    let bytes = indirect.as_bytes().unwrap();
    drop(string_obj);
    pdf.delete_object(obj_num).unwrap();
    for _ in 0..32 {
        let _ = PdfObject::new_string(&"B".repeat(1000));
    }
    assert_eq!(&bytes[..], original.as_bytes());
}

use mupdf::pdf::{InsertImageOptions, PageImageSource, PdfDocument, PdfPage, PdfWriteOptions};
#[cfg(not(target_arch = "wasm32"))]
use mupdf::DocumentWriter;
use mupdf::{Colorspace, Image, Pixmap, Rect, Size};

/// A colour image of `w` x `h` pixels with smooth gradients, as a scan has.
fn gradient(w: i32, h: i32) -> Pixmap {
    let mut pixmap = Pixmap::new_with_w_h(&Colorspace::device_rgb(), w, h, false).unwrap();
    let samples = pixmap.samples_mut();
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 3) as usize;
            samples[i] = (x * 255 / w) as u8;
            samples[i + 1] = (y * 255 / h) as u8;
            samples[i + 2] = ((x + y) % 256) as u8;
        }
    }
    pixmap
}

fn page_image_size(doc: &PdfDocument) -> (u32, u32) {
    let page = PdfPage::try_from(doc.load_page(0).unwrap()).unwrap();
    let images = page.images().unwrap();
    assert_eq!(images.len(), 1);
    (images[0].width, images[0].height)
}

/// One Letter page filled by a 300 dpi image (2550 x 3300 pixels).
fn scan() -> PdfDocument {
    let image = Image::from_pixmap(&gradient(2550, 3300)).unwrap();
    let mut doc = PdfDocument::new();
    let mut page = doc.new_page(Size::new(612.0, 792.0)).unwrap();
    page.insert_image(
        &mut doc,
        Rect::new(0.0, 0.0, 612.0, 792.0),
        PageImageSource::Image(&image),
        InsertImageOptions::default(),
    )
    .unwrap();
    drop(page);
    doc
}

fn saved_len(doc: &PdfDocument) -> usize {
    let mut options = PdfWriteOptions::default();
    options
        .set_garbage_level(4)
        .set_compress(true)
        .set_compress_images(true);
    let mut bytes = Vec::new();
    doc.write_to_with_options(&mut bytes, options).unwrap();
    bytes.len()
}

#[test]
fn test_rewrite_images_scales_to_the_target_resolution() {
    let mut doc = scan();
    let before = saved_len(&doc);
    assert_eq!(page_image_size(&doc), (2550, 3300));
    // Exactly half: the averaging subsampler would leave this image as it is.
    doc.rewrite_images(225, 150, 75).unwrap();
    assert_eq!(page_image_size(&doc), (1275, 1650));
    let after = saved_len(&doc);
    assert!(after * 2 <= before, "{before} -> {after} bytes");
}

#[test]
fn test_rewrite_images_leaves_images_below_the_threshold() {
    let mut doc = scan();
    doc.rewrite_images(400, 150, 75).unwrap();
    assert_eq!(page_image_size(&doc), (2550, 3300));
}

/// One Letter-sized page with a 1200 x 800 colour image shown at 300 dpi (288 x 192 pt).
fn small_scan() -> PdfDocument {
    let image = Image::from_pixmap(&gradient(1200, 800)).unwrap();
    let mut doc = PdfDocument::new();
    let mut page = doc.new_page(Size::new(612.0, 792.0)).unwrap();
    page.insert_image(
        &mut doc,
        Rect::new(0.0, 0.0, 288.0, 192.0),
        PageImageSource::Image(&image),
        InsertImageOptions::default(),
    )
    .unwrap();
    drop(page);
    doc
}

/// Saves and reopens the document, so the size is the one a reader would see.
fn reopened(doc: &PdfDocument) -> PdfDocument {
    let mut bytes = Vec::new();
    doc.write_to_with_options(&mut bytes, PdfWriteOptions::default())
        .unwrap();
    PdfDocument::from_copied_bytes(&bytes).unwrap()
}

#[test]
fn test_rewrite_images_refuses_a_zero_target_resolution() {
    let mut doc = small_scan();
    assert!(matches!(
        doc.rewrite_images(150, 0, 75),
        Err(mupdf::Error::InvalidArgument(_))
    ));
    // Nothing changed: the image is not a single pixel.
    assert_eq!(page_image_size(&reopened(&doc)), (1200, 800));
}

#[test]
fn test_rewrite_images_zero_threshold_turns_scaling_off() {
    let mut doc = small_scan();
    doc.rewrite_images(0, 0, 75).unwrap();
    assert_eq!(page_image_size(&reopened(&doc)), (1200, 800));
    let mut doc = small_scan();
    doc.rewrite_images(0, 150, 75).unwrap();
    assert_eq!(page_image_size(&reopened(&doc)), (1200, 800));
}

#[test]
fn test_rewrite_images_scales_and_survives_a_save() {
    let mut doc = small_scan();
    doc.rewrite_images(150, 150, 75).unwrap();
    assert_eq!(page_image_size(&reopened(&doc)), (600, 400));
}

#[test]
fn test_save_as_jpeg() {
    let dir = std::env::temp_dir();
    let path = dir.join("mupdf_rs_test_save_as_jpeg.jpg");
    gradient(320, 200)
        .save_as_jpeg(path.to_str().unwrap(), 85)
        .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(&bytes[..3], b"\xff\xd8\xff");
    let image = Image::from_file(path.to_str().unwrap()).unwrap();
    assert_eq!((image.width(), image.height()), (320, 200));
    let _ = std::fs::remove_file(&path);

    // A pixmap with alpha cannot be a JPEG: an error, not an abort.
    let alpha = Pixmap::new_with_w_h(&Colorspace::device_rgb(), 4, 4, true).unwrap();
    assert!(alpha.save_as_jpeg(path.to_str().unwrap(), 85).is_err());
    let _ = std::fs::remove_file(&path);
}

// `DocumentWriter::new` reaches the OCR writer and with it tesseract's C++ code, which the
// Emscripten build does not link (as for `test_issue_43_malloc`).
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_document_writer_close() {
    let dir = std::env::temp_dir();
    let path = dir.join("mupdf_rs_test_writer_close.cbz");
    let mut writer = DocumentWriter::new(path.to_str().unwrap(), "cbz", "").unwrap();
    let page = writer
        .begin_page(Rect::new(0.0, 0.0, 100.0, 100.0))
        .unwrap();
    page.end().unwrap();
    writer.close().unwrap();
    assert!(std::fs::metadata(&path).unwrap().len() > 0);
    let _ = std::fs::remove_file(&path);
}

use mupdf::pdf::{InsertImageOptions, PageImageSource, PdfDocument, PdfPage, PdfWriteOptions};
use mupdf::{Colorspace, DocumentWriter, Image, Pixmap, Rect, Size};

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

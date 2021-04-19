use mupdf::pdf::{PdfDocument, PdfFilterOptions, PdfPage};
use mupdf::Error;

fn count_images(doc: &PdfDocument, page_num: i32) -> Result<i32, Error> {
    let page = doc.find_page(page_num).unwrap();

    let objs = page
        .get_dict("Resources")?
        .unwrap()
        .get_dict("XObject")?
        .unwrap();

    let mut count = 0;
    let len = objs.dict_len()? as i32;
    for i in 0..len {
        let key = objs.get_dict_key(i)?.unwrap();
        let key_name = key.as_name()?;
        if key_name.starts_with('I') {
            count += 1;
        }
    }

    Ok(count)
}

#[test]
fn test_filter_page() {
    let doc = PdfDocument::open("tests/files/multiple-images.pdf").unwrap();
    let page_num = 0;
    let mut page: PdfPage = doc.load_page(page_num).unwrap().into();

    assert_eq!(count_images(&doc, page_num).unwrap(), 5);

    let mut opts = PdfFilterOptions::default();
    // Otherwise filtering is disabled.
    opts.set_sanitize(true);
    // The first three images will be removed.
    let mut count = 0;
    opts.set_image_filter(|_ctm, name, image| {
        println!("name: {:?}", name);
        if count < 3 {
            count += 1;
            None
        } else {
            Some(image.clone())
        }
    });

    page.filter(opts).unwrap();

    assert_eq!(count_images(&doc, page_num).unwrap(), 2);
}

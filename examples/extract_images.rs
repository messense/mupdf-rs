use std::io::Write;

use mupdf::{Document, ImageFormat, TextPageFlags};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filename: String = std::env::args().nth(1).expect("missing filename");
    let document = Document::open(&filename)?;

    let mut image_num: u32 = 0;

    for page in document.pages()? {
        let text_page = page?.to_text_page(TextPageFlags::PRESERVE_IMAGES)?;

        for block in text_page.blocks() {
            if let Some(image) = block.image() {
                let pixmap = image.to_pixmap()?;
                let mut bytes: Vec<u8> = vec![];
                pixmap.write_to(&mut bytes, ImageFormat::PNG)?;

                let mut output_file = std::fs::File::create(format!("output_{}.png", image_num))?;
                output_file.write_all(&bytes)?;

                image_num += 1;
            }
        }
    }

    Ok(())
}

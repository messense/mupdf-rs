use std::io;

use mupdf::{page::StextPage, Document, TextPageFlags};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filename: String = std::env::args().nth(1).expect("missing filename");
    let document = Document::open(&filename)?;

    for page in document.pages()? {
        let text_page = page?.to_text_page(TextPageFlags::empty())?;

        let json = text_page.to_json(1.0)?;
        let stext_page: StextPage = serde_json::from_str(json.as_str())?;

        for block in stext_page.blocks {
            if block.r#type == "text" {
                for line in block.lines {
                    println!("{:?}", &line.text);
                }
            }
        }
    }

    Ok(())
}

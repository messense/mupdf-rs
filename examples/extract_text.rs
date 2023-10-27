fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filename: String = std::env::args()
        .collect::<Vec<_>>()
        .get(1)
        .expect("missing filename")
        .to_owned();
    let document = mupdf::document::Document::open(&filename)?;

    for page in document.pages()? {
        let text_page = page?.to_text_page(mupdf::text_page::TextPageOptions::empty())?;

        for block in text_page.blocks() {
            for line in block.lines() {
                let chars: String = line.chars().map(|c| c.char().unwrap()).collect();
                println!("line: {}", chars);
            }
        }
    }

    Ok(())
}

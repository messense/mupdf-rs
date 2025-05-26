use mupdf::{Document, TextPageFlags};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filename: String = std::env::args().nth(1).expect("missing filename");
    let document = Document::open(&filename)?;

    for page in document.pages()? {
        let text_page = page?.to_text_page(TextPageFlags::empty())?;

        for block in text_page.blocks() {
            for line in block.lines() {
                let chars: String = line.chars().map(|c| c.char().unwrap()).collect();
                println!("line: {}", chars);
            }
        }
    }

    Ok(())
}

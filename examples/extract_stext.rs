use std::io;

fn main() {
    // cargo run --example extract_stext
    let mut path_to_doc = String::new();
    println!("Enter a path to document: ");
    io::stdin().read_line(&mut path_to_doc).expect("Failed to read line");
    let doc = mupdf::document::Document::open(path_to_doc.trim()).unwrap();
    let page = doc.load_page(0).unwrap();
    let stext_page = page.to_text_page(mupdf::text_page::TextPageOptions::empty()).unwrap();
    match stext_page.stext_page_as_json(1.0) {
        Ok(stext_json) => {
            let stext_page: serde_json::Result<mupdf::text_page::StextPage> = serde_json::from_str(stext_json.as_str());
            match stext_page {
                Ok(res) => {
                    for block in res.blocks {
                        if block.r#type.eq("text") {
                            for line in block.lines {
                                println!("{:?}", &line.text);
                            }
                        }
                    }
                }
                Err(err) => {
                    println!("stext_json parsing error: {:?}", &err);
                }
            }
        }
        Err(_) => {}
    }
}

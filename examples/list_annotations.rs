use mupdf::pdf::PdfDocument;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filename = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: cargo run --example list_annotations -- <pdf-file>");
        std::process::exit(1);
    });

    let document = PdfDocument::open(&filename)?;
    let mut found = 0usize;

    for page_no in 0..document.page_count()? {
        let display_page_no = page_no + 1;
        let page = match document.load_pdf_page(page_no) {
            Ok(page) => page,
            Err(err) => {
                eprintln!("page {display_page_no} error={err}");
                continue;
            }
        };

        for annot in page.annotations() {
            let kind = match annot.r#type() {
                Ok(kind) => kind,
                Err(err) => {
                    eprintln!("page {display_page_no} type=<unknown> error={err}");
                    continue;
                }
            };

            match annot.rect() {
                Ok(rect) => {
                    found += 1;
                    println!("page {display_page_no} type={kind:?} rect={rect}");
                }
                Err(err) => {
                    eprintln!("page {display_page_no} type={kind:?} error={err}");
                }
            }
        }

        // Link annotations are not yielded by PdfPage::annotations().
        match page.link_annotations() {
            Ok(link_annots) => {
                let page_ctm = match page.ctm() {
                    Ok(ctm) => ctm,
                    Err(err) => {
                        eprintln!("page {display_page_no} type=Link error={err}");
                        continue;
                    }
                };

                for link in link_annots {
                    let link = match link {
                        Ok(link) => link,
                        Err(err) => {
                            eprintln!("page {display_page_no} type=Link error={err}");
                            continue;
                        }
                    };

                    match link.rect(Some(&page_ctm)) {
                        Ok(bounds) => {
                            found += 1;
                            println!("page {display_page_no} type=Link rect={bounds}");
                        }
                        Err(err) => {
                            eprintln!("page {display_page_no} type=Link error={err}");
                        }
                    }
                }
            }
            Err(err) => {
                eprintln!("page {display_page_no} type=Link error={err}");
            }
        }
    }

    if found == 0 {
        println!("no annotations found");
    }

    Ok(())
}

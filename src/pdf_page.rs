use mupdf_sys::*;

use crate::Page;

#[derive(Debug)]
pub struct PdfPage {
    page: Page,
}

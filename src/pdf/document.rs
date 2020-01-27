use std::ffi::CString;

use mupdf_sys::*;

use crate::{context, Document, Error};

#[derive(Debug)]
pub struct PdfDocument {
    inner: *mut pdf_document,
}

impl PdfDocument {
    pub fn new() -> Self {
        let inner = unsafe { pdf_create_document(context()) };
        Self { inner }
    }

    pub fn open(filename: &str) -> Result<Self, Error> {
        let doc = Document::open(filename)?;
        let inner = unsafe { pdf_document_from_fz_document(context(), doc.inner) };
        Ok(Self { inner })
    }
}

impl Drop for PdfDocument {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                pdf_drop_document(context(), self.inner);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::PdfDocument;

    #[test]
    fn test_open_pdf_document() {
        let _doc = PdfDocument::open("tests/files/dummy.pdf").unwrap();
    }
}

use mupdf_sys::*;

use crate::context;

#[derive(Debug)]
pub struct PdfDocument {
    inner: *mut pdf_document,
}

impl PdfDocument {
    pub fn new() -> Self {
        let inner = unsafe { pdf_create_document(context()) };
        Self { inner }
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

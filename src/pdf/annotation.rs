use mupdf_sys::*;

use crate::context;

#[derive(Debug)]
pub struct PdfAnnotation {
    pub(crate) inner: *mut pdf_annot,
}

impl PdfAnnotation {
    pub(crate) unsafe fn from_raw(ptr: *mut pdf_annot) -> Self {
        Self { inner: ptr }
    }
}

impl Drop for PdfAnnotation {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                pdf_drop_annot(context(), self.inner);
            }
        }
    }
}

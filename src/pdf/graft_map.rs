use mupdf_sys::*;

use crate::context;

#[derive(Debug)]
pub struct PdfGraftMap {
    pub(crate) inner: *mut pdf_graft_map,
}

impl PdfGraftMap {
    pub(crate) unsafe fn from_raw(ptr: *mut pdf_graft_map) -> Self {
        Self { inner: ptr }
    }
}

impl Drop for PdfGraftMap {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                pdf_drop_graft_map(context(), self.inner);
            }
        }
    }
}

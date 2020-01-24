use mupdf_sys::*;

use crate::context;

#[derive(Debug)]
pub struct Document {
    pub(crate) inner: *mut fz_document,
}

impl Drop for Document {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe { fz_drop_document(context(), self.inner) }
        }
    }
}

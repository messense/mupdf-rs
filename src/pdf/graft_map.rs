use mupdf_sys::*;

use crate::{context, pdf::PdfObject, Error};

#[derive(Debug)]
pub struct PdfGraftMap {
    pub(crate) inner: *mut pdf_graft_map,
}

impl PdfGraftMap {
    pub(crate) unsafe fn from_raw(ptr: *mut pdf_graft_map) -> Self {
        Self { inner: ptr }
    }

    pub fn graft_object(&mut self, obj: &PdfObject) -> Result<PdfObject, Error> {
        unsafe {
            ffi_try!(mupdf_pdf_graft_mapped_object(
                context(),
                self.inner,
                obj.inner
            ))
        }
        .map(|inner| unsafe { PdfObject::from_raw(inner) })
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

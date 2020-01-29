use std::ops::{Deref, DerefMut};

use mupdf_sys::*;

use crate::{context, Error, Page, PdfAnnotation};

#[derive(Debug)]
pub struct PdfPage {
    pub(crate) inner: *mut pdf_page,
    page: Page,
}

impl PdfPage {
    pub(crate) unsafe fn from_raw(ptr: *mut pdf_page) -> Self {
        Self {
            inner: ptr,
            page: Page::from_raw(ptr as *mut fz_page),
        }
    }

    pub fn create_annotation(&mut self, subtype: i32) -> Result<PdfAnnotation, Error> {
        unsafe {
            let annot = ffi_try!(mupdf_pdf_create_annot(context(), self.inner, subtype));
            Ok(PdfAnnotation::from_raw(annot))
        }
    }

    pub fn delete_annotation(&mut self, annot: &PdfAnnotation) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_delete_annot(context(), self.inner, annot.inner));
        }
        Ok(())
    }

    pub fn update(&mut self) -> Result<bool, Error> {
        let ret = unsafe { ffi_try!(mupdf_pdf_update_page(context(), self.inner)) };
        Ok(ret)
    }

    pub fn redact(&mut self) -> Result<bool, Error> {
        let ret = unsafe { ffi_try!(mupdf_pdf_redact_page(context(), self.inner)) };
        Ok(ret)
    }
}

impl Deref for PdfPage {
    type Target = Page;

    fn deref(&self) -> &Self::Target {
        &self.page
    }
}

impl DerefMut for PdfPage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.page
    }
}

use std::ops::{Deref, DerefMut};

use mupdf_sys::*;

use crate::{context, Error, Page, PdfAnnotation, PdfObject};

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

    pub fn object(&self) -> PdfObject {
        unsafe { PdfObject::from_raw((*self.inner).obj, false) }
    }

    pub fn rotation(&self) -> Result<i32, Error> {
        if let Some(rotate) = self
            .object()
            .get_dict_inheritable(&PdfObject::new_name("Rotate")?)?
        {
            return rotate.as_int();
        }
        Ok(0)
    }

    pub fn set_rotation(&mut self, rotate: i32) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_page_set_rotation(context(), self.inner, rotate));
        }
        Ok(())
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

impl From<Page> for PdfPage {
    fn from(page: Page) -> Self {
        let ptr = page.inner;
        Self {
            inner: ptr as *mut pdf_page,
            page,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{PdfDocument, PdfPage};

    #[test]
    fn test_page_rotation() {
        let doc = PdfDocument::open("tests/files/dummy.pdf").unwrap();
        let mut page0 = PdfPage::from(doc.load_page(0).unwrap());

        let rotation = page0.rotation().unwrap();
        assert_eq!(rotation, 0);

        page0.set_rotation(90).unwrap();
        let rotation = page0.rotation().unwrap();
        assert_eq!(rotation, 90);
    }
}

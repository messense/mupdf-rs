use std::ffi::CString;

use mupdf_sys::*;

use crate::{context, Document, Error, Font, Image, PdfObject};

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

    pub fn new_null(&self) -> PdfObject {
        unsafe {
            let inner = mupdf_pdf_new_null();
            PdfObject::from_raw(inner)
        }
    }

    pub fn new_bool(&self, b: bool) -> PdfObject {
        unsafe {
            let inner = mupdf_pdf_new_bool(b);
            PdfObject::from_raw(inner)
        }
    }

    pub fn new_int(&self, i: i32) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_new_int(context(), i));
            Ok(PdfObject::from_raw(inner))
        }
    }

    pub fn new_real(&self, f: f32) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_new_real(context(), f));
            Ok(PdfObject::from_raw(inner))
        }
    }

    pub fn new_string(&self, s: &str) -> Result<PdfObject, Error> {
        let c_str = CString::new(s)?;
        unsafe {
            let inner = ffi_try!(mupdf_pdf_new_string(context(), c_str.as_ptr()));
            Ok(PdfObject::from_raw(inner))
        }
    }

    pub fn new_name(&self, name: &str) -> Result<PdfObject, Error> {
        let c_name = CString::new(name)?;
        unsafe {
            let inner = ffi_try!(mupdf_pdf_new_name(context(), c_name.as_ptr()));
            Ok(PdfObject::from_raw(inner))
        }
    }

    pub fn new_indirect(&self, num: i32, gen: i32) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_new_indirect(context(), self.inner, num, gen));
            Ok(PdfObject::from_raw(inner))
        }
    }

    pub fn new_array(&self) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_new_array(context(), self.inner, 0));
            Ok(PdfObject::from_raw(inner))
        }
    }

    pub fn new_dict(&self) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_new_dict(context(), self.inner, 0));
            Ok(PdfObject::from_raw(inner))
        }
    }

    pub fn add_object(&mut self, obj: &PdfObject) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_add_object(context(), self.inner, obj.inner));
            Ok(PdfObject::from_raw(inner))
        }
    }

    pub fn create_object(&mut self) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_create_object(context(), self.inner));
            Ok(PdfObject::from_raw(inner))
        }
    }

    pub fn delete_object(&mut self, num: i32) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_delete_object(context(), self.inner, num));
        }
        Ok(())
    }

    pub fn add_image(&mut self, obj: &Image) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_add_image(context(), self.inner, obj.inner));
            Ok(PdfObject::from_raw(inner))
        }
    }

    pub fn add_font(&mut self, font: &Font) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_add_font(context(), self.inner, font.inner));
            Ok(PdfObject::from_raw(inner))
        }
    }

    pub fn add_cjk_font(
        &mut self,
        font: &Font,
        ordering: i32,
        wmode: i32,
        serif: bool,
    ) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_add_cjk_font(
                context(),
                self.inner,
                font.inner,
                ordering,
                wmode,
                serif
            ));
            Ok(PdfObject::from_raw(inner))
        }
    }

    pub fn add_simple_font(&mut self, font: &Font, encoding: i32) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_add_simple_font(
                context(),
                self.inner,
                font.inner,
                encoding
            ));
            Ok(PdfObject::from_raw(inner))
        }
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

    #[test]
    fn test_pdf_document_new_objs() {
        let pdf = PdfDocument::new();

        let obj = pdf.new_null();
        assert!(obj.is_null().unwrap());

        let obj = pdf.new_bool(true);
        assert!(obj.is_bool().unwrap());
        assert!(obj.as_bool().unwrap());

        let obj = pdf.new_int(1).unwrap();
        assert!(obj.is_int().unwrap());
        assert!(obj.is_number().unwrap());
        assert_eq!(obj.as_int().unwrap(), 1);

        let obj = pdf.new_real(1.0).unwrap();
        assert!(obj.is_real().unwrap());
        assert!(obj.is_number().unwrap());
        assert_eq!(obj.as_float().unwrap(), 1.0);

        let obj = pdf.new_string("PDF").unwrap();
        assert!(obj.is_string().unwrap());
        assert_eq!(obj.as_string().unwrap(), "PDF");

        let obj = pdf.new_name("Type").unwrap();
        assert!(obj.is_name().unwrap());
        assert_eq!(obj.as_name().unwrap(), "Type");

        let obj = pdf.new_array().unwrap();
        assert!(obj.is_array().unwrap());

        let obj = pdf.new_dict().unwrap();
        assert!(obj.is_dict().unwrap());
    }
}

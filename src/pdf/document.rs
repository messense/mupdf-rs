use std::ffi::CString;
use std::io::{self, Write};
use std::ops::{Deref, DerefMut};

use bitflags::bitflags;
use mupdf_sys::*;

use crate::{
    context, Buffer, CjkFontOrdering, Document, Error, Font, Image, PdfGraftMap, PdfObject,
    PdfPage, SimpleFontEncoding, Size, WriteMode,
};

bitflags! {
    pub struct Permission: i32 {
        const PRINT = 1 << 2;
        const MODIFY = 1 << 3;
        const COPY = 1 << 4;
        const ANNOTATE = 1 << 5;
        const FORM = 1 << 8;
        const ACCESSIBILITY = 1 << 9;
        const ASSEMBLE = 1 << 10;
        const PRINT_HQ = 1 << 11;
    }
}

#[derive(Clone, Copy)]
pub struct PdfWriteOptions {
    inner: pdf_write_options,
}

impl Default for PdfWriteOptions {
    fn default() -> Self {
        unsafe {
            Self {
                inner: pdf_default_write_options,
            }
        }
    }
}

impl PdfWriteOptions {
    pub fn incremental(&self) -> bool {
        self.inner.do_incremental != 0
    }

    pub fn set_incremental(&mut self, value: bool) -> &mut Self {
        self.inner.do_incremental = if value { 1 } else { 0 };
        self
    }

    pub fn pretty(&self) -> bool {
        self.inner.do_pretty != 0
    }

    pub fn set_pretty(&mut self, value: bool) -> &mut Self {
        self.inner.do_pretty = if value { 1 } else { 0 };
        self
    }

    pub fn ascii(&self) -> bool {
        self.inner.do_ascii != 0
    }

    pub fn set_ascii(&mut self, value: bool) -> &mut Self {
        self.inner.do_ascii = if value { 1 } else { 0 };
        self
    }

    pub fn compress(&self) -> bool {
        self.inner.do_compress != 0
    }

    pub fn set_compress(&mut self, value: bool) -> &mut Self {
        self.inner.do_compress = if value { 1 } else { 0 };
        self
    }

    pub fn compress_images(&self) -> bool {
        self.inner.do_compress_images != 0
    }

    pub fn set_compress_images(&mut self, value: bool) -> &mut Self {
        self.inner.do_compress_images = if value { 1 } else { 0 };
        self
    }

    pub fn compress_fonts(&self) -> bool {
        self.inner.do_compress_fonts != 0
    }

    pub fn set_compress_fonts(&mut self, value: bool) -> &mut Self {
        self.inner.do_compress_fonts = if value { 1 } else { 0 };
        self
    }

    pub fn decompress(&self) -> bool {
        self.inner.do_decompress != 0
    }

    pub fn set_decompress(&mut self, value: bool) -> &mut Self {
        self.inner.do_decompress = if value { 1 } else { 0 };
        self
    }

    pub fn garbage(&self) -> bool {
        self.inner.do_garbage != 0
    }

    pub fn set_garbage(&mut self, value: bool) -> &mut Self {
        self.inner.do_garbage = if value { 1 } else { 0 };
        self
    }

    pub fn linear(self) -> bool {
        self.inner.do_linear != 0
    }

    pub fn set_linear(&mut self, value: bool) -> &mut Self {
        self.inner.do_linear = if value { 1 } else { 0 };
        self
    }

    pub fn clean(&self) -> bool {
        self.inner.do_clean != 0
    }

    pub fn set_clean(&mut self, value: bool) -> &mut Self {
        self.inner.do_clean = if value { 1 } else { 0 };
        self
    }

    pub fn sanitize(&self) -> bool {
        self.inner.do_sanitize != 0
    }

    pub fn set_sanitize(&mut self, value: bool) -> &mut Self {
        self.inner.do_sanitize = if value { 1 } else { 0 };
        self
    }

    pub fn appearance(&self) -> bool {
        self.inner.do_appearance != 0
    }

    pub fn set_appearance(&mut self, value: bool) -> &mut Self {
        self.inner.do_appearance = if value { 1 } else { 0 };
        self
    }

    pub fn encrypt(&self) -> bool {
        self.inner.do_encrypt != 0
    }

    pub fn set_encrypt(&mut self, value: bool) -> &mut Self {
        self.inner.do_encrypt = if value { 1 } else { 0 };
        self
    }

    pub fn permissions(&self) -> Permission {
        Permission::from_bits(self.inner.permissions).unwrap()
    }

    pub fn set_permissions(&mut self, value: Permission) -> &mut Self {
        self.inner.permissions = value.bits;
        self
    }
    // TODO: password
}

#[derive(Debug)]
pub struct PdfDocument {
    inner: *mut pdf_document,
    doc: Document,
}

impl PdfDocument {
    pub fn new() -> Self {
        unsafe {
            let inner = pdf_create_document(context());
            let doc = Document::from_raw(&mut (*inner).super_);
            Self { inner, doc }
        }
    }

    pub fn open(filename: &str) -> Result<Self, Error> {
        let doc = Document::open(filename)?;
        let inner = unsafe { pdf_document_from_fz_document(context(), doc.inner) };
        if inner.is_null() {
            return Err(Error::InvalidPdfDocument);
        }
        Ok(Self { inner, doc })
    }

    pub fn new_null(&self) -> PdfObject {
        unsafe {
            let inner = mupdf_pdf_new_null();
            PdfObject::from_raw(inner, true)
        }
    }

    pub fn new_bool(&self, b: bool) -> PdfObject {
        unsafe {
            let inner = mupdf_pdf_new_bool(b);
            PdfObject::from_raw(inner, true)
        }
    }

    pub fn new_int(&self, i: i32) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_new_int(context(), i));
            Ok(PdfObject::from_raw(inner, true))
        }
    }

    pub fn new_real(&self, f: f32) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_new_real(context(), f));
            Ok(PdfObject::from_raw(inner, true))
        }
    }

    pub fn new_string(&self, s: &str) -> Result<PdfObject, Error> {
        let c_str = CString::new(s)?;
        unsafe {
            let inner = ffi_try!(mupdf_pdf_new_string(context(), c_str.as_ptr()));
            Ok(PdfObject::from_raw(inner, true))
        }
    }

    pub fn new_name(&self, name: &str) -> Result<PdfObject, Error> {
        let c_name = CString::new(name)?;
        unsafe {
            let inner = ffi_try!(mupdf_pdf_new_name(context(), c_name.as_ptr()));
            Ok(PdfObject::from_raw(inner, true))
        }
    }

    pub fn new_indirect(&self, num: i32, gen: i32) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_new_indirect(context(), self.inner, num, gen));
            Ok(PdfObject::from_raw(inner, true))
        }
    }

    pub fn new_array(&self) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_new_array(context(), self.inner, 0));
            Ok(PdfObject::from_raw(inner, true))
        }
    }

    pub fn new_dict(&self) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_new_dict(context(), self.inner, 0));
            Ok(PdfObject::from_raw(inner, true))
        }
    }

    pub fn new_graft_map(&self) -> Result<PdfGraftMap, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_new_graft_map(context(), self.inner));
            Ok(PdfGraftMap::from_raw(inner))
        }
    }

    pub fn graft_object(&self, obj: &PdfObject) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_graft_object(context(), self.inner, obj.inner));
            Ok(PdfObject::from_raw(inner, true))
        }
    }

    pub fn add_object(&mut self, obj: &PdfObject) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_add_object(context(), self.inner, obj.inner));
            Ok(PdfObject::from_raw(inner, true))
        }
    }

    pub fn create_object(&mut self) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_create_object(context(), self.inner));
            Ok(PdfObject::from_raw(inner, true))
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
            Ok(PdfObject::from_raw(inner, true))
        }
    }

    pub fn add_font(&mut self, font: &Font) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_add_font(context(), self.inner, font.inner));
            Ok(PdfObject::from_raw(inner, true))
        }
    }

    pub fn add_cjk_font(
        &mut self,
        font: &Font,
        ordering: CjkFontOrdering,
        wmode: WriteMode,
        serif: bool,
    ) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_add_cjk_font(
                context(),
                self.inner,
                font.inner,
                ordering as i32,
                wmode as i32,
                serif
            ));
            Ok(PdfObject::from_raw(inner, true))
        }
    }

    pub fn add_simple_font(
        &mut self,
        font: &Font,
        encoding: SimpleFontEncoding,
    ) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_add_simple_font(
                context(),
                self.inner,
                font.inner,
                encoding as i32
            ));
            Ok(PdfObject::from_raw(inner, true))
        }
    }

    pub fn has_unsaved_changes(&self) -> bool {
        unsafe { pdf_has_unsaved_changes(context(), self.inner) != 0 }
    }

    pub fn can_be_saved_incrementally(&self) -> bool {
        unsafe { pdf_can_be_saved_incrementally(context(), self.inner) != 0 }
    }

    pub fn save_with_options(&self, filename: &str, options: PdfWriteOptions) -> Result<(), Error> {
        let c_name = CString::new(filename)?;
        unsafe {
            ffi_try!(mupdf_pdf_save_document(
                context(),
                self.inner,
                c_name.as_ptr(),
                options.inner
            ));
        }
        Ok(())
    }

    pub fn enable_js(&mut self) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_enable_js(context(), self.inner));
        }
        Ok(())
    }

    pub fn disable_js(&mut self) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_disable_js(context(), self.inner));
        }
        Ok(())
    }

    pub fn is_js_supported(&mut self) -> Result<bool, Error> {
        let supported = unsafe { ffi_try!(mupdf_pdf_js_supported(context(), self.inner)) };
        Ok(supported)
    }

    pub fn calculate_form(&mut self) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_calculate_form(context(), self.inner));
        }
        Ok(())
    }

    pub fn trailer(&self) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_trailer(context(), self.inner));
            Ok(PdfObject::from_raw(inner, true))
        }
    }

    pub fn count_objects(&self) -> Result<u32, Error> {
        let count = unsafe { ffi_try!(mupdf_pdf_count_objects(context(), self.inner)) };
        Ok(count as u32)
    }

    pub fn has_acro_form(&self) -> Result<bool, Error> {
        let trailer = self.trailer()?;
        if let Some(root) = trailer.get_dict("Root")? {
            if let Some(form) = root.get_dict("AcroForm")? {
                if let Some(fields) = form.get_dict("Fields")? {
                    return Ok(fields.len()? > 0);
                }
            }
        }
        Ok(false)
    }

    pub fn has_xfa_form(&self) -> Result<bool, Error> {
        let trailer = self.trailer()?;
        if let Some(root) = trailer.get_dict("Root")? {
            if let Some(form) = root.get_dict("AcroForm")? {
                if let Some(xfa) = form.get_dict("XFA")? {
                    return xfa.is_null();
                }
            }
        }
        Ok(false)
    }

    pub fn permissions(&self) -> Permission {
        let bits = unsafe { pdf_document_permissions(context(), self.inner) };
        Permission::from_bits(bits).unwrap_or_else(Permission::all)
    }

    pub fn save(&self, filename: &str) -> Result<(), Error> {
        self.save_with_options(filename, PdfWriteOptions::default())
    }

    fn write_with_options(&self, options: PdfWriteOptions) -> Result<Buffer, Error> {
        unsafe {
            let buf = ffi_try!(mupdf_pdf_write_document(
                context(),
                self.inner,
                options.inner
            ));
            Ok(Buffer::from_raw(buf))
        }
    }

    pub fn write_to_with_options<W: Write>(
        &self,
        w: &mut W,
        options: PdfWriteOptions,
    ) -> Result<u64, Error> {
        let mut buf = self.write_with_options(options)?;
        Ok(io::copy(&mut buf, w)?)
    }

    pub fn write_to<W: Write>(&self, w: &mut W) -> Result<u64, Error> {
        self.write_to_with_options(w, PdfWriteOptions::default())
    }

    pub fn find_page(&self, page_no: i32) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_lookup_page_obj(context(), self.inner, page_no));
            Ok(PdfObject::from_raw(inner, true))
        }
    }

    pub fn new_page_at<T: Into<Size>>(&mut self, page_no: i32, size: T) -> Result<PdfPage, Error> {
        let size = size.into();
        unsafe {
            let inner = ffi_try!(mupdf_pdf_new_page(
                context(),
                self.inner,
                page_no,
                size.width,
                size.height
            ));
            Ok(PdfPage::from_raw(inner))
        }
    }

    pub fn new_page<T: Into<Size>>(&mut self, size: T) -> Result<PdfPage, Error> {
        self.new_page_at(-1, size)
    }

    pub fn insert_page(&mut self, page_no: i32, page: &PdfObject) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_insert_page(
                context(),
                self.inner,
                page_no,
                page.inner
            ));
        }
        Ok(())
    }

    pub fn delete_page(&mut self, page_no: i32) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_delete_page(context(), self.inner, page_no));
        }
        Ok(())
    }
}

impl Deref for PdfDocument {
    type Target = Document;

    fn deref(&self) -> &Self::Target {
        &self.doc
    }
}

impl DerefMut for PdfDocument {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.doc
    }
}

#[cfg(test)]
mod test {
    use super::{PdfDocument, Permission};

    #[test]
    fn test_open_pdf_document() {
        let doc = PdfDocument::open("tests/files/dummy.pdf").unwrap();
        assert!(!doc.has_unsaved_changes());
        assert!(!doc.has_acro_form().unwrap());
        assert!(!doc.has_xfa_form().unwrap());
        assert!(!doc.needs_password().unwrap());

        let trailer = doc.trailer().unwrap();
        assert!(!trailer.is_null().unwrap());

        let count = doc.count_objects().unwrap();
        assert_eq!(count, 16);

        let mut output = Vec::new();
        let n = doc.write_to(&mut output).unwrap();
        assert!(n > 0);

        let perm = doc.permissions();
        assert!(perm.contains(Permission::PRINT));
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
        assert_eq!(obj.as_bytes().unwrap(), [80, 68, 70]);

        let obj = pdf.new_name("Type").unwrap();
        assert!(obj.is_name().unwrap());
        assert_eq!(obj.as_name().unwrap(), "Type");

        let obj = pdf.new_array().unwrap();
        assert!(obj.is_array().unwrap());

        let obj = pdf.new_dict().unwrap();
        assert!(obj.is_dict().unwrap());
    }

    #[test]
    fn test_pdf_document_new_page() {
        use crate::Size;

        let mut pdf = PdfDocument::new();
        let page = pdf.new_page(Size::A4).unwrap();
        assert!(pdf.has_unsaved_changes());

        assert_eq!(page.rotation().unwrap(), 0);
        let bounds = page.bounds().unwrap();
        assert_eq!(bounds.x0, 0.0);
        assert_eq!(bounds.y0, 0.0);
        assert_eq!(bounds.x1, 595.0);
        assert_eq!(bounds.y1, 842.0);
    }
}

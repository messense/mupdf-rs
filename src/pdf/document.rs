use std::convert::TryFrom;
use std::ffi::{CStr, CString};
use std::io::{self, Write};
use std::ops::{Deref, DerefMut};
use std::ptr;

use bitflags::bitflags;
use mupdf_sys::*;
use num_enum::TryFromPrimitive;

use crate::pdf::{PdfGraftMap, PdfObject, PdfPage};
use crate::{
    context, Buffer, CjkFontOrdering, Destination, DestinationKind, Document, Error, Font, Image,
    Outline, Point, SimpleFontEncoding, Size, WriteMode,
};

bitflags! {
    pub struct Permission: u32 {
        const PRINT = PDF_PERM_PRINT as _;
        const MODIFY = PDF_PERM_MODIFY as _;
        const COPY = PDF_PERM_COPY as _;
        const ANNOTATE = PDF_PERM_ANNOTATE as _;
        const FORM = PDF_PERM_FORM as _;
        const ACCESSIBILITY = PDF_PERM_ACCESSIBILITY as _;
        const ASSEMBLE = PDF_PERM_ASSEMBLE as _;
        const PRINT_HQ = PDF_PERM_PRINT_HQ as _;
    }
}

#[derive(Debug, Copy, Clone, PartialEq, TryFromPrimitive)]
#[repr(u32)]
pub enum Encryption {
    Aes128 = PDF_ENCRYPT_AES_128 as u32,
    Aes256 = PDF_ENCRYPT_AES_256 as u32,
    Rc4_40 = PDF_ENCRYPT_RC4_40 as u32,
    Rc4_128 = PDF_ENCRYPT_RC4_128 as u32,
    Keep = PDF_ENCRYPT_KEEP as u32,
    None = PDF_ENCRYPT_NONE as u32,
    Unknown = PDF_ENCRYPT_UNKNOWN as u32,
}

impl Default for Encryption {
    fn default() -> Encryption {
        Self::None
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

    pub fn garbage_level(&self) -> i32 {
        self.inner.do_garbage
    }

    pub fn set_garbage_level(&mut self, value: i32) -> &mut Self {
        self.inner.do_garbage = value.clamp(0, 4);
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

    pub fn encryption(&self) -> Encryption {
        Encryption::try_from(self.inner.do_encrypt as u32).unwrap()
    }

    pub fn set_encryption(&mut self, value: Encryption) -> &mut Self {
        self.inner.do_encrypt = value as _;
        self
    }

    pub fn permissions(&self) -> Permission {
        Permission::from_bits(self.inner.permissions as u32).unwrap()
    }

    pub fn set_permissions(&mut self, value: Permission) -> &mut Self {
        self.inner.permissions = value.bits() as _;
        self
    }

    pub fn owner_password(&self) -> &str {
        let c_pwd = unsafe { CStr::from_ptr(self.inner.opwd_utf8.as_ptr()) };
        c_pwd.to_str().unwrap()
    }

    pub fn set_owner_password(&mut self, pwd: &str) -> &mut Self {
        let len = pwd.len() + 1;
        let c_pwd = CString::new(pwd).unwrap();
        unsafe {
            ptr::copy_nonoverlapping(c_pwd.as_ptr(), self.inner.opwd_utf8.as_mut_ptr(), len);
        }
        self
    }

    pub fn user_password(&self) -> &str {
        let c_pwd = unsafe { CStr::from_ptr(self.inner.upwd_utf8.as_ptr()) };
        c_pwd.to_str().unwrap()
    }

    pub fn set_user_password(&mut self, pwd: &str) -> &mut Self {
        let len = pwd.len() + 1;
        let c_pwd = CString::new(pwd).unwrap();
        unsafe {
            ptr::copy_nonoverlapping(c_pwd.as_ptr(), self.inner.upwd_utf8.as_mut_ptr(), len);
        }
        self
    }
}

#[derive(Debug)]
pub struct PdfDocument {
    inner: *mut pdf_document,
    doc: Document,
}

impl PdfDocument {
    pub(crate) unsafe fn from_raw(ptr: *mut pdf_document) -> Self {
        let doc = Document::from_raw(&mut (*ptr).super_);
        Self { inner: ptr, doc }
    }

    pub fn new() -> Self {
        unsafe {
            let inner = pdf_create_document(context());
            let doc = Document::from_raw(&mut (*inner).super_);
            Self { inner, doc }
        }
    }

    pub fn open(filename: &str) -> Result<Self, Error> {
        let doc = Document::open(filename)?;
        Self::try_from(doc)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let len = bytes.len();
        let mut buf = Buffer::with_capacity(len);
        buf.write_all(bytes)?;
        unsafe {
            let inner = ffi_try!(mupdf_pdf_open_document_from_bytes(context(), buf.inner));
            Ok(Self::from_raw(inner))
        }
    }

    pub fn new_null(&self) -> PdfObject {
        PdfObject::new_null()
    }

    pub fn new_bool(&self, b: bool) -> PdfObject {
        PdfObject::new_bool(b)
    }

    pub fn new_int(&self, i: i32) -> Result<PdfObject, Error> {
        PdfObject::new_int(i)
    }

    pub fn new_real(&self, f: f32) -> Result<PdfObject, Error> {
        PdfObject::new_real(f)
    }

    pub fn new_string(&self, s: &str) -> Result<PdfObject, Error> {
        PdfObject::new_string(s)
    }

    pub fn new_name(&self, name: &str) -> Result<PdfObject, Error> {
        PdfObject::new_name(name)
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

    pub fn new_graft_map(&self) -> Result<PdfGraftMap, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_new_graft_map(context(), self.inner));
            Ok(PdfGraftMap::from_raw(inner))
        }
    }

    pub fn new_object_from_str(&self, src: &str) -> Result<PdfObject, Error> {
        let c_src = CString::new(src)?;
        unsafe {
            let inner = ffi_try!(mupdf_pdf_obj_from_str(
                context(),
                self.inner,
                c_src.as_ptr()
            ));
            Ok(PdfObject::from_raw(inner))
        }
    }

    pub fn graft_object(&self, obj: &PdfObject) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_graft_object(context(), self.inner, obj.inner));
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
            Ok(PdfObject::from_raw(inner))
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
            Ok(PdfObject::from_raw(inner))
        }
    }

    pub fn has_unsaved_changes(&self) -> bool {
        unsafe { pdf_has_unsaved_changes(context(), self.inner) != 0 }
    }

    pub fn is_dirty(&self) -> bool {
        self.has_unsaved_changes()
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

    pub fn is_js_supported(&self) -> Result<bool, Error> {
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
            Ok(PdfObject::from_raw(inner))
        }
    }

    pub fn catalog(&self) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_catalog(context(), self.inner));
            Ok(PdfObject::from_raw(inner))
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
        Permission::from_bits(bits as u32).unwrap_or_else(Permission::all)
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
            Ok(PdfObject::from_raw(inner))
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

    pub fn set_outlines(&mut self, toc: &[Outline]) -> Result<(), Error> {
        self.delete_outlines()?;

        if !toc.is_empty() {
            let mut outlines = self.new_dict()?;
            outlines.dict_put("Type", PdfObject::new_name("Outlines")?)?;
            // Now we access outlines indirectly
            let mut outlines = self.add_object(&outlines)?;
            self.walk_outlines_insert(toc, &mut outlines)?;
            self.catalog()?.dict_put("Outlines", outlines)?;
        }

        Ok(())
    }

    fn walk_outlines_insert(
        &mut self,
        down: &[Outline],
        parent: &mut PdfObject,
    ) -> Result<(), Error> {
        debug_assert!(!down.is_empty() && parent.is_indirect()?);

        // All the indirect references in the current level.
        let mut refs = Vec::new();

        for outline in down {
            let mut item = self.new_dict()?;
            item.dict_put("Title", PdfObject::new_string(&outline.title)?)?;
            item.dict_put("Parent", parent.clone())?;
            if let Some(dest) = outline
                .page
                .map(|page| {
                    let page = self.find_page(page as i32)?;

                    let matrix = page.page_ctm()?;
                    let fz_point = Point::new(outline.x, outline.y);
                    let Point { x, y } = fz_point.transform(&matrix);
                    let dest_kind = DestinationKind::XYZ {
                        left: Some(x),
                        top: Some(y),
                        zoom: None,
                    };
                    let dest = Destination::new(page, dest_kind);

                    let mut array = self.new_array()?;
                    dest.encode_into(&mut array)?;

                    Ok(array)
                })
                .or_else(|| outline.uri.as_deref().map(PdfObject::new_string))
                .transpose()?
            {
                item.dict_put("Dest", dest)?;
            }

            refs.push(self.add_object(&item)?);
            if !outline.down.is_empty() {
                self.walk_outlines_insert(&outline.down, refs.last_mut().unwrap())?;
            }
        }

        // NOTE: doing the same thing as mutation version of `slice::array_windows`
        for i in 0..down.len().saturating_sub(1) {
            let [prev, next, ..] = &mut refs[i..] else {
                unreachable!();
            };
            prev.dict_put("Next", next.clone())?;
            next.dict_put("Prev", prev.clone())?;
        }

        let mut refs = refs.into_iter();
        let first = refs.next().unwrap();
        let last = refs.next_back().unwrap_or_else(|| first.clone());

        parent.dict_put("First", first)?;
        parent.dict_put("Last", last)?;

        Ok(())
    }

    /// Delete `/Outlines` in document catalog and all the **outline items** it points to.
    ///
    /// Do nothing if document has no outlines.
    pub fn delete_outlines(&mut self) -> Result<(), Error> {
        if let Some(outlines) = self.catalog()?.get_dict("Outlines")? {
            if let Some(outline) = outlines.get_dict("First")? {
                self.walk_outlines_del(outline)?;
            }
            self.delete_object(outlines.as_indirect()?)?;
        }

        Ok(())
    }

    fn walk_outlines_del(&mut self, outline: PdfObject) -> Result<(), Error> {
        let mut cur = Some(outline);

        while let Some(item) = cur.take() {
            if let Some(down) = item.get_dict("First")? {
                self.walk_outlines_del(down)?;
            }
            cur = item.get_dict("Next")?;
            self.delete_object(item.as_indirect()?)?;
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

impl TryFrom<Document> for PdfDocument {
    type Error = Error;

    fn try_from(doc: Document) -> Result<Self, Self::Error> {
        let inner = unsafe { pdf_document_from_fz_document(context(), doc.inner) };
        if inner.is_null() {
            return Err(Error::InvalidPdfDocument);
        }
        Ok(Self { inner, doc })
    }
}

impl<'a> IntoIterator for &'a PdfDocument {
    type Item = Result<crate::Page, Error>;
    type IntoIter = crate::document::PageIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.doc.into_iter()
    }
}

#[cfg(test)]
mod test {
    use super::{PdfDocument, PdfWriteOptions, Permission};

    #[test]
    fn test_pdf_write_options_passwords() {
        let mut options = PdfWriteOptions::default();
        let owner_pwd = options.owner_password();
        let user_pwd = options.user_password();
        assert!(owner_pwd.is_empty());
        assert!(user_pwd.is_empty());

        options.set_owner_password("abc").set_user_password("def");
        let owner_pwd = options.owner_password();
        let user_pwd = options.user_password();
        assert_eq!(owner_pwd, "abc");
        assert_eq!(user_pwd, "def");
    }

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

        let catalog = doc.catalog().unwrap();
        assert!(!catalog.is_null().unwrap());
    }

    #[test]
    fn test_open_pdf_document_from_bytes() {
        use std::fs;
        use std::io::Read;

        let mut bytes = Vec::new();
        let mut file = fs::File::open("tests/files/dummy.pdf").unwrap();
        file.read_to_end(&mut bytes).unwrap();
        let doc = PdfDocument::from_bytes(&bytes).unwrap();
        assert!(!doc.needs_password().unwrap());
    }

    #[test]
    fn test_pdf_document_new_objs() {
        let pdf = PdfDocument::new();

        let obj = pdf.new_null();
        assert!(obj.is_null().unwrap());
        assert_eq!(obj.to_string(), "null");
        assert!(obj.document().is_none());

        let obj = pdf.new_bool(true);
        assert!(obj.is_bool().unwrap());
        assert!(obj.as_bool().unwrap());
        assert_eq!(obj.to_string(), "true");

        let obj = pdf.new_int(1).unwrap();
        assert!(obj.is_int().unwrap());
        assert!(obj.is_number().unwrap());
        assert_eq!(obj.as_int().unwrap(), 1);
        assert_eq!(obj.to_string(), "1");

        let obj = pdf.new_real(1.0).unwrap();
        assert!(obj.is_real().unwrap());
        assert!(obj.is_number().unwrap());
        assert_eq!(obj.as_float().unwrap(), 1.0);
        assert_eq!(obj.to_string(), "1");

        let obj = pdf.new_string("PDF").unwrap();
        assert!(obj.is_string().unwrap());
        assert_eq!(obj.as_string().unwrap(), "PDF");
        assert_eq!(obj.as_bytes().unwrap(), [80, 68, 70]);
        assert_eq!(obj.to_string(), "(PDF)");

        let obj = pdf.new_name("Type").unwrap();
        assert!(obj.is_name().unwrap());
        assert_eq!(obj.as_name().unwrap(), b"Type");
        assert_eq!(obj.to_string(), "/Type");

        let obj = pdf.new_array().unwrap();
        assert!(obj.is_array().unwrap());
        assert_eq!(obj.to_string(), "[]");
        assert!(obj.document().is_some());

        let obj = pdf.new_dict().unwrap();
        assert!(obj.is_dict().unwrap());
        assert_eq!(obj.to_string(), "<<>>");
        assert!(obj.document().is_some());

        let obj = pdf.new_object_from_str(r#"<</Author<FEFF004500760061006E00670065006C006F007300200056006C006100630068006F006700690061006E006E00690073>
        /Creator<FEFF005700720069007400650072>
        /Producer<FEFF004F00700065006E004F00660066006900630065002E006F0072006700200032002E0031>
        /CreationDate(D:20070223175637+02'00')>>"#).unwrap();
        assert!(obj.is_dict().unwrap());
    }

    #[test]
    fn test_pdf_object_array() {
        let pdf = PdfDocument::new();
        let mut obj = pdf.new_array().unwrap();
        obj.array_put(0, true.into()).unwrap();
        obj.array_put(1, pdf.new_int(1).unwrap()).unwrap();
        obj.array_push(pdf.new_string("abc").unwrap()).unwrap();
        let val0 = obj.get_array(0).unwrap().unwrap();
        assert!(val0.as_bool().unwrap());
        let val1 = obj.get_array(1).unwrap().unwrap();
        assert_eq!(val1.as_int().unwrap(), 1);
        let val2 = obj.get_array(2).unwrap().unwrap();
        assert_eq!(val2.as_string().unwrap(), "abc");
        assert_eq!(obj.len().unwrap(), 3);
        // delete
        obj.array_delete(2).unwrap();
        assert_eq!(obj.len().unwrap(), 2);
    }

    #[test]
    fn test_pdf_object_dict() {
        let pdf = PdfDocument::new();
        let mut obj = pdf.new_dict().unwrap();
        obj.dict_put("name", true.into()).unwrap();
        obj.dict_put(
            pdf.new_name("test").unwrap(),
            pdf.new_string("test").unwrap(),
        )
        .unwrap();
        let val0 = obj.get_dict("name").unwrap().unwrap();
        assert!(val0.as_bool().unwrap());
        let val1 = obj.get_dict("test").unwrap().unwrap();
        assert_eq!(val1.as_string().unwrap(), "test");
        obj.dict_delete("test").unwrap();
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

    #[test]
    fn test_pdf_document_find_page() {
        let doc = PdfDocument::open("tests/files/dummy.pdf").unwrap();
        let _page = doc.find_page(0).unwrap();
    }
}

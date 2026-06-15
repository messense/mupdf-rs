use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::convert::TryFrom;
use std::ffi::{c_char, c_int, CStr, CString};
use std::io::{self, Read, Write};
use std::ops::{Deref, DerefMut, Range, RangeInclusive};
use std::ptr::{self, NonNull};

use bitflags::bitflags;

use mupdf_sys::*;

use crate::pdf::{DocOperation, ExtractedImage, FontInfo, PdfGraftMap, PdfObject, PdfPage};
use crate::{
    context, from_enum, Buffer, CjkFontOrdering, Destination, Document, Error, FilePath, Font,
    Image, Outline, SimpleFontEncoding, Size, WriteMode,
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

from_enum! { c_int => c_int,
    #[derive(Debug, Copy, Clone, PartialEq, Default)]
    pub enum Encryption {
        Aes128 = PDF_ENCRYPT_AES_128,
        Aes256 = PDF_ENCRYPT_AES_256,
        Rc4_40 = PDF_ENCRYPT_RC4_40,
        Rc4_128 = PDF_ENCRYPT_RC4_128,
        Keep = PDF_ENCRYPT_KEEP,
        #[default]
        None = PDF_ENCRYPT_NONE,
        Unknown = PDF_ENCRYPT_UNKNOWN,
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
        Encryption::try_from(self.inner.do_encrypt).unwrap()
    }

    pub fn set_encryption(&mut self, value: Encryption) -> &mut Self {
        self.inner.do_encrypt = value.into();
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
        assert!(pwd.len() < self.inner.opwd_utf8.len());
        unsafe {
            ptr::copy_nonoverlapping(
                pwd.as_ptr().cast(),
                self.inner.opwd_utf8.as_mut_ptr(),
                pwd.len(),
            );
        }
        self.inner.opwd_utf8[pwd.len()] = 0;
        self
    }

    pub fn user_password(&self) -> &str {
        let c_pwd = unsafe { CStr::from_ptr(self.inner.upwd_utf8.as_ptr()) };
        c_pwd.to_str().unwrap()
    }

    pub fn set_user_password(&mut self, pwd: &str) -> &mut Self {
        assert!(pwd.len() < self.inner.upwd_utf8.len());
        unsafe {
            ptr::copy_nonoverlapping(
                pwd.as_ptr().cast(),
                self.inner.upwd_utf8.as_mut_ptr(),
                pwd.len(),
            );
        }
        self.inner.upwd_utf8[pwd.len()] = 0;
        self
    }
}

#[derive(Debug)]
pub struct PdfDocument {
    inner: *mut pdf_document,
    doc: Document,
    pub(crate) font_info_cache: RefCell<HashMap<i32, FontInfo>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EmbeddedFileOptions<'a> {
    pub filename: &'a str,
    pub mime_type: Option<&'a str>,
    pub created: Option<i64>,
    pub modified: Option<i64>,
    pub add_checksum: bool,
}

impl<'a> EmbeddedFileOptions<'a> {
    pub fn new(filename: &'a str) -> Self {
        Self {
            filename,
            mime_type: None,
            created: None,
            modified: None,
            add_checksum: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmbeddedFileInfo {
    pub name: String,
    pub xref: i32,
    pub filename: Option<String>,
    pub mime_type: Option<String>,
    pub size: usize,
    pub created: Option<i64>,
    pub modified: Option<i64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageRange {
    pub start: usize,
    pub end: usize,
}

impl PageRange {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(self) -> bool {
        self.start >= self.end
    }
}

impl From<Range<usize>> for PageRange {
    fn from(range: Range<usize>) -> Self {
        Self::new(range.start, range.end)
    }
}

impl From<RangeInclusive<usize>> for PageRange {
    fn from(range: RangeInclusive<usize>) -> Self {
        let (start, end) = range.into_inner();
        Self::new(start, end.saturating_add(1))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum PageSelection {
    #[default]
    All,
    Range(PageRange),
    Pages(Vec<usize>),
}

impl PageSelection {
    fn validated_indices(&self, page_count: usize) -> Result<Vec<usize>, Error> {
        let indices = match self {
            Self::All => (0..page_count).collect(),
            Self::Range(range) => {
                if range.is_empty() {
                    return Err(Error::InvalidArgument(
                        "page selection range must not be empty".to_owned(),
                    ));
                }
                if range.end > page_count {
                    return Err(Error::InvalidArgument(format!(
                        "page selection range {}..{} exceeds page count {page_count}",
                        range.start, range.end
                    )));
                }
                (range.start..range.end).collect()
            }
            Self::Pages(pages) => {
                if pages.is_empty() {
                    return Err(Error::InvalidArgument(
                        "page selection must not be empty".to_owned(),
                    ));
                }
                for page in pages {
                    if *page >= page_count {
                        return Err(Error::InvalidArgument(format!(
                            "page index {page} exceeds page count {page_count}"
                        )));
                    }
                }
                pages.clone()
            }
        };

        if indices.is_empty() {
            return Err(Error::InvalidArgument(
                "page selection must not be empty".to_owned(),
            ));
        }

        Ok(indices)
    }

    fn validated_unique_sorted_indices(&self, page_count: usize) -> Result<Vec<usize>, Error> {
        let indices = self.validated_indices(page_count)?;
        let unique: BTreeSet<_> = indices.iter().copied().collect();
        if unique.len() != indices.len() {
            return Err(Error::InvalidArgument(
                "page selection must not contain duplicates".to_owned(),
            ));
        }
        Ok(unique.into_iter().collect())
    }
}

impl From<PageRange> for PageSelection {
    fn from(range: PageRange) -> Self {
        Self::Range(range)
    }
}

impl From<Range<usize>> for PageSelection {
    fn from(range: Range<usize>) -> Self {
        Self::Range(range.into())
    }
}

impl From<RangeInclusive<usize>> for PageSelection {
    fn from(range: RangeInclusive<usize>) -> Self {
        Self::Range(range.into())
    }
}

impl From<Vec<usize>> for PageSelection {
    fn from(pages: Vec<usize>) -> Self {
        Self::Pages(pages)
    }
}

impl From<&[usize]> for PageSelection {
    fn from(pages: &[usize]) -> Self {
        Self::Pages(pages.to_vec())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum InsertPosition {
    #[default]
    Append,
    Before(usize),
    After(usize),
}

impl InsertPosition {
    fn resolve(self, page_count: usize) -> Result<usize, Error> {
        match self {
            Self::Append => Ok(page_count),
            Self::Before(index) if index <= page_count => Ok(index),
            Self::Before(index) => Err(Error::InvalidArgument(format!(
                "insert position Before({index}) exceeds page count {page_count}"
            ))),
            Self::After(index) if index < page_count => Ok(index + 1),
            Self::After(index) => Err(Error::InvalidArgument(format!(
                "insert position After({index}) exceeds page count {page_count}"
            ))),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InsertPdfOptions {
    pub source_pages: PageSelection,
    pub target: InsertPosition,
    pub rotate: Option<i32>,
    pub copy_links: bool,
    pub copy_annotations: bool,
    pub copy_widgets: bool,
}

impl Default for InsertPdfOptions {
    fn default() -> Self {
        Self {
            source_pages: PageSelection::All,
            target: InsertPosition::Append,
            rotate: None,
            copy_links: true,
            copy_annotations: true,
            copy_widgets: true,
        }
    }
}

impl InsertPdfOptions {
    fn validate_supported(&self) -> Result<(), Error> {
        if !self.copy_links || !self.copy_annotations || !self.copy_widgets {
            return Err(Error::InvalidArgument(
                "selective link, annotation, and widget copying is not supported yet".to_owned(),
            ));
        }
        if let Some(rotate) = self.rotate {
            if rotate % 90 != 0 {
                return Err(Error::InvalidArgument(
                    "inserted page rotation must be a multiple of 90 degrees".to_owned(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InsertPdfResult {
    pub inserted_pages: PageRange,
    pub page_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageLabelStyle {
    None,
    Decimal,
    UpperRoman,
    LowerRoman,
    UpperAlpha,
    LowerAlpha,
}

impl PageLabelStyle {
    fn into_raw(self) -> pdf_page_label_style {
        match self {
            Self::None => PDF_PAGE_LABEL_NONE,
            Self::Decimal => PDF_PAGE_LABEL_DECIMAL,
            Self::UpperRoman => PDF_PAGE_LABEL_ROMAN_UC,
            Self::LowerRoman => PDF_PAGE_LABEL_ROMAN_LC,
            Self::UpperAlpha => PDF_PAGE_LABEL_ALPHA_UC,
            Self::LowerAlpha => PDF_PAGE_LABEL_ALPHA_LC,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageLabelRule {
    pub index: usize,
    pub style: PageLabelStyle,
    pub prefix: String,
    pub start: i32,
}

impl PageLabelRule {
    pub fn new(index: usize, style: PageLabelStyle) -> Self {
        Self {
            index,
            style,
            prefix: String::new(),
            start: 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct OptionalContentRef {
    xref: i32,
}

impl OptionalContentRef {
    pub fn new(xref: i32) -> Result<Self, Error> {
        if xref <= 0 {
            return Err(Error::InvalidArgument(
                "optional content xref must be positive".to_owned(),
            ));
        }
        Ok(Self { xref })
    }

    pub fn xref(self) -> i32 {
        self.xref
    }
}

impl TryFrom<i32> for OptionalContentRef {
    type Error = Error;

    fn try_from(xref: i32) -> Result<Self, Self::Error> {
        Self::new(xref)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptionalContentGroup {
    pub reference: OptionalContentRef,
    pub name: Option<String>,
    pub enabled: bool,
}

impl Default for PdfDocument {
    fn default() -> Self {
        let inner = unsafe { pdf_create_document(context()) };
        let doc = unsafe { Document::from_raw(&mut (*inner).super_) };
        Self {
            inner,
            doc,
            font_info_cache: RefCell::new(HashMap::new()),
        }
    }
}

impl PdfDocument {
    pub(crate) unsafe fn from_raw(ptr: *mut pdf_document) -> Self {
        let doc = Document::from_raw(&mut (*ptr).super_);
        Self {
            inner: ptr,
            doc,
            font_info_cache: RefCell::new(HashMap::new()),
        }
    }

    pub(crate) fn as_raw(&self) -> *mut pdf_document {
        self.inner
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn open<P: AsRef<FilePath> + ?Sized>(p: &P) -> Result<Self, Error> {
        let doc = Document::open(p)?;
        Self::try_from(doc)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let len = bytes.len();
        let mut buf = Buffer::with_capacity(len);
        buf.write_all(bytes)?;
        unsafe { ffi_try!(mupdf_pdf_open_document_from_bytes(context(), buf.inner)) }
            .map(|inner| unsafe { Self::from_raw(inner) })
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
        unsafe { ffi_try!(mupdf_pdf_new_indirect(context(), self.inner, num, gen)) }
            .map(|inner| unsafe { PdfObject::from_raw(inner) })
    }

    pub fn new_array(&self) -> Result<PdfObject, Error> {
        unsafe { ffi_try!(mupdf_pdf_new_array(context(), self.inner, 0)) }
            .map(|inner| unsafe { PdfObject::from_raw(inner) })
    }

    pub fn new_array_with_capacity(&self, capacity: i32) -> Result<PdfObject, Error> {
        unsafe { ffi_try!(mupdf_pdf_new_array(context(), self.inner, capacity)) }
            .map(|inner| unsafe { PdfObject::from_raw(inner) })
    }

    pub fn new_dict(&self) -> Result<PdfObject, Error> {
        unsafe { ffi_try!(mupdf_pdf_new_dict(context(), self.inner, 0)) }
            .map(|inner| unsafe { PdfObject::from_raw(inner) })
    }

    pub fn new_dict_with_capacity(&self, capacity: i32) -> Result<PdfObject, Error> {
        unsafe { ffi_try!(mupdf_pdf_new_dict(context(), self.inner, capacity)) }
            .map(|inner| unsafe { PdfObject::from_raw(inner) })
    }

    pub fn new_graft_map(&self) -> Result<PdfGraftMap, Error> {
        unsafe { ffi_try!(mupdf_pdf_new_graft_map(context(), self.inner)) }
            .map(|inner| unsafe { PdfGraftMap::from_raw(inner) })
    }

    pub fn new_object_from_str(&self, src: &str) -> Result<PdfObject, Error> {
        let c_src = CString::new(src)?;
        unsafe {
            ffi_try!(mupdf_pdf_obj_from_str(
                context(),
                self.inner,
                c_src.as_ptr()
            ))
        }
        .map(|inner| unsafe { PdfObject::from_raw(inner) })
    }

    pub fn graft_object(&self, obj: &PdfObject) -> Result<PdfObject, Error> {
        unsafe { ffi_try!(mupdf_pdf_graft_object(context(), self.inner, obj.inner)) }
            .map(|inner| unsafe { PdfObject::from_raw(inner) })
    }

    pub fn add_object(&mut self, obj: &PdfObject) -> Result<PdfObject, Error> {
        unsafe { ffi_try!(mupdf_pdf_add_object(context(), self.inner, obj.inner)) }
            .map(|inner| unsafe { PdfObject::from_raw(inner) })
    }

    /// Adds a new indirect PDF stream object with the given buffer and optional dictionary.
    pub fn add_stream(
        &mut self,
        buf: &Buffer,
        obj: Option<&PdfObject>,
        compressed: bool,
    ) -> Result<PdfObject, Error> {
        let obj = obj.map_or(ptr::null_mut(), |obj| obj.inner);
        unsafe {
            ffi_try!(mupdf_pdf_add_stream(
                context(),
                self.inner,
                buf.inner,
                obj,
                compressed as c_int
            ))
        }
        .map(|inner| unsafe { PdfObject::from_raw(inner) })
    }

    pub fn create_object(&mut self) -> Result<PdfObject, Error> {
        unsafe { ffi_try!(mupdf_pdf_create_object(context(), self.inner)) }
            .map(|inner| unsafe { PdfObject::from_raw(inner) })
    }

    pub fn delete_object(&mut self, num: i32) -> Result<(), Error> {
        unsafe { ffi_try!(mupdf_pdf_delete_object(context(), self.inner, num)) }
    }

    pub fn add_image(&mut self, obj: &Image) -> Result<PdfObject, Error> {
        unsafe { ffi_try!(mupdf_pdf_add_image(context(), self.inner, obj.inner)) }
            .map(|inner| unsafe { PdfObject::from_raw(inner) })
    }

    pub fn add_font(&mut self, font: &Font) -> Result<PdfObject, Error> {
        unsafe { ffi_try!(mupdf_pdf_add_font(context(), self.inner, font.inner)) }
            .map(|inner| unsafe { PdfObject::from_raw(inner) })
    }

    pub fn add_cjk_font(
        &mut self,
        font: &Font,
        ordering: CjkFontOrdering,
        wmode: WriteMode,
        serif: bool,
    ) -> Result<PdfObject, Error> {
        unsafe {
            ffi_try!(mupdf_pdf_add_cjk_font(
                context(),
                self.inner,
                font.inner,
                ordering as i32,
                wmode as i32,
                serif
            ))
        }
        .map(|inner| unsafe { PdfObject::from_raw(inner) })
    }

    pub fn add_simple_font(
        &mut self,
        font: &Font,
        encoding: SimpleFontEncoding,
    ) -> Result<PdfObject, Error> {
        unsafe {
            ffi_try!(mupdf_pdf_add_simple_font(
                context(),
                self.inner,
                font.inner,
                encoding as i32
            ))
        }
        .map(|inner| unsafe { PdfObject::from_raw(inner) })
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
            ))
        }
    }

    pub fn enable_js(&mut self) -> Result<(), Error> {
        unsafe { ffi_try!(mupdf_pdf_enable_js(context(), self.inner)) }
    }

    pub fn disable_js(&mut self) -> Result<(), Error> {
        unsafe { ffi_try!(mupdf_pdf_disable_js(context(), self.inner)) }
    }

    pub fn is_js_supported(&self) -> Result<bool, Error> {
        unsafe { ffi_try!(mupdf_pdf_js_supported(context(), self.inner)) }
    }

    pub fn calculate_form(&mut self) -> Result<(), Error> {
        unsafe { ffi_try!(mupdf_pdf_calculate_form(context(), self.inner)) }
    }

    pub fn bake(&mut self, bake_annots: bool, bake_widgets: bool) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_bake_document(
                context(),
                self.inner,
                bake_annots,
                bake_widgets
            ))
        }
    }

    pub fn trailer(&self) -> Result<PdfObject, Error> {
        unsafe { ffi_try!(mupdf_pdf_trailer(context(), self.inner)) }
            .map(|inner| unsafe { PdfObject::from_raw(inner) })
    }

    pub fn load_name_tree(&self, d: PdfObject) -> Result<PdfObject, Error> {
        unsafe { ffi_try!(mupdf_pdf_load_name_tree(context(), self.inner, d.inner)) }
            .map(|inner| unsafe { PdfObject::from_raw(inner) })
    }

    pub fn catalog(&self) -> Result<PdfObject, Error> {
        unsafe { ffi_try!(mupdf_pdf_catalog(context(), self.inner)) }
            .map(|inner| unsafe { PdfObject::from_raw(inner) })
    }

    pub fn count_objects(&self) -> Result<u32, Error> {
        unsafe { ffi_try!(mupdf_pdf_count_objects(context(), self.inner)) }
            .map(|count| count as u32)
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
            ffi_try!(mupdf_pdf_write_document(
                context(),
                self.inner,
                options.inner
            ))
        }
        .map(|buf| unsafe { Buffer::from_raw(buf) })
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
        unsafe { ffi_try!(mupdf_pdf_lookup_page_obj(context(), self.inner, page_no)) }
            .map(|inner| unsafe { PdfObject::from_raw(inner) })
    }

    /// Given a page object reference, returns its zero-based page number.
    pub fn lookup_page_number(&self, page_obj: &PdfObject) -> Result<i32, Error> {
        unsafe {
            ffi_try!(mupdf_pdf_lookup_page_number(
                context(),
                self.inner,
                page_obj.inner
            ))
        }
    }

    /// Loads a PdfPage from a page index.
    /// Helper function to convert from Document page loading to PdfPage.
    pub fn load_pdf_page(&self, page_no: i32) -> Result<PdfPage, Error> {
        let page = self.doc.load_page(page_no)?;
        PdfPage::try_from(page)
    }

    pub fn new_page_at<T: Into<Size>>(&mut self, page_no: i32, size: T) -> Result<PdfPage, Error> {
        let size = size.into();
        let inner = unsafe {
            ffi_try!(mupdf_pdf_new_page(
                context(),
                self.inner,
                page_no,
                size.width,
                size.height
            ))
        }?;
        let inner = NonNull::new(inner).ok_or(Error::UnexpectedNullPtr)?;
        Ok(unsafe { PdfPage::from_raw(inner) })
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
            ))
        }
    }

    pub fn delete_page(&mut self, page_no: i32) -> Result<(), Error> {
        unsafe { ffi_try!(mupdf_pdf_delete_page(context(), self.inner, page_no)) }
    }

    fn ensure_embedded_files_array(&self) -> Result<PdfObject, Error> {
        let mut catalog = self.catalog()?;
        let mut names = match catalog.get_dict("Names")? {
            Some(names) if names.is_dict()? => names,
            _ => {
                let names = self.new_dict()?;
                catalog.dict_put_ref("Names", &names)?;
                names
            }
        };
        let mut embedded_files = match names.get_dict("EmbeddedFiles")? {
            Some(embedded_files) if embedded_files.is_dict()? => embedded_files,
            _ => {
                let embedded_files = self.new_dict()?;
                names.dict_put_ref("EmbeddedFiles", &embedded_files)?;
                embedded_files
            }
        };
        match embedded_files.get_dict("Names")? {
            Some(array) if array.is_array()? => Ok(array),
            _ => {
                let array = self.new_array()?;
                embedded_files.dict_put_ref("Names", &array)?;
                Ok(array)
            }
        }
    }

    fn embedded_files_array(&self) -> Result<Option<PdfObject>, Error> {
        let Some(names) = self.catalog()?.get_dict("Names")? else {
            return Ok(None);
        };
        if !names.is_dict()? {
            return Ok(None);
        }
        let Some(embedded_files) = names.get_dict("EmbeddedFiles")? else {
            return Ok(None);
        };
        if !embedded_files.is_dict()? {
            return Ok(None);
        }
        match embedded_files.get_dict("Names")? {
            Some(array) if array.is_array()? => Ok(Some(array)),
            _ => Ok(None),
        }
    }

    fn named_embedded_file_object(&self, name: &str) -> Result<Option<PdfObject>, Error> {
        let Some(array) = self.embedded_files_array()? else {
            return Ok(None);
        };
        let len = array.len()?;
        let mut index = 0;
        while index + 1 < len {
            let key_matches = array
                .get_array(index as i32)?
                .and_then(|key| key.as_string().ok().map(|key| key == name))
                .unwrap_or(false);
            if key_matches {
                return array.get_array((index + 1) as i32);
            }
            index += 2;
        }
        Ok(None)
    }

    fn filespec_info(&self, name: String, fs: PdfObject) -> Result<EmbeddedFileInfo, Error> {
        let is_embedded =
            unsafe { ffi_try!(mupdf_pdf_is_embedded_file(context(), fs.inner))? != 0 };
        if !is_embedded {
            return Err(Error::InvalidArgument(format!(
                "named file {name} is not an embedded file"
            )));
        }

        let params = unsafe { ffi_try!(mupdf_pdf_get_filespec_params(context(), fs.inner))? };
        let filename = if params.filename.is_null() {
            None
        } else {
            Some(
                unsafe { CStr::from_ptr(params.filename) }
                    .to_string_lossy()
                    .into_owned(),
            )
        };
        let mime_type = if params.mimetype.is_null() {
            None
        } else {
            Some(
                unsafe { CStr::from_ptr(params.mimetype) }
                    .to_string_lossy()
                    .into_owned(),
            )
        };

        Ok(EmbeddedFileInfo {
            name,
            xref: fs.as_indirect().unwrap_or(0),
            filename,
            mime_type,
            size: params.size.max(0) as usize,
            created: (params.created >= 0).then_some(params.created),
            modified: (params.modified >= 0).then_some(params.modified),
        })
    }

    fn put_named_embedded_file(&self, name: &str, filespec: &PdfObject) -> Result<(), Error> {
        let mut array = self.ensure_embedded_files_array()?;
        let len = array.len()?;
        let mut index = 0;
        while index + 1 < len {
            let key_matches = array
                .get_array(index as i32)?
                .and_then(|key| key.as_string().ok().map(|key| key == name))
                .unwrap_or(false);
            if key_matches {
                array.array_put((index + 1) as i32, filespec.clone())?;
                return Ok(());
            }
            index += 2;
        }
        array.array_push(PdfObject::new_string(name)?)?;
        array.array_push_ref(filespec)
    }

    pub fn add_embedded_file(
        &mut self,
        name: &str,
        contents: &[u8],
        options: EmbeddedFileOptions<'_>,
    ) -> Result<EmbeddedFileInfo, Error> {
        let filename = CString::new(options.filename)?;
        let mime_type = options.mime_type.map(CString::new).transpose()?;
        let contents = Buffer::from_bytes(contents)?;
        let fs = unsafe {
            ffi_try!(mupdf_pdf_add_embedded_file(
                context(),
                self.inner,
                filename.as_ptr(),
                mime_type
                    .as_ref()
                    .map_or(ptr::null(), |mime_type| mime_type.as_ptr()),
                contents.inner,
                options.created.unwrap_or(-1),
                options.modified.unwrap_or(-1),
                i32::from(options.add_checksum)
            ))
        }
        .map(|inner| unsafe { PdfObject::from_raw(inner) })?;
        let fs = if fs.is_indirect()? {
            fs
        } else {
            self.add_object(&fs)?
        };
        self.put_named_embedded_file(name, &fs)?;
        self.filespec_info(name.to_owned(), fs)
    }

    pub fn embedded_files(&self) -> Result<Vec<EmbeddedFileInfo>, Error> {
        let Some(array) = self.embedded_files_array()? else {
            return Ok(Vec::new());
        };

        let mut files = Vec::new();
        let len = array.len()?;
        let mut index = 0;
        while index + 1 < len {
            let Some(name) = array
                .get_array(index as i32)?
                .and_then(|name| name.as_string().ok().map(str::to_owned))
            else {
                index += 2;
                continue;
            };
            if let Some(filespec) = array.get_array((index + 1) as i32)? {
                files.push(self.filespec_info(name, filespec)?);
            }
            index += 2;
        }
        Ok(files)
    }

    pub fn embedded_file(&self, name: &str) -> Result<Option<EmbeddedFileInfo>, Error> {
        let Some(filespec) = self.named_embedded_file_object(name)? else {
            return Ok(None);
        };
        self.filespec_info(name.to_owned(), filespec).map(Some)
    }

    pub fn load_embedded_file(&self, name: &str) -> Result<Option<Vec<u8>>, Error> {
        let Some(filespec) = self.named_embedded_file_object(name)? else {
            return Ok(None);
        };
        let buffer = unsafe {
            ffi_try!(mupdf_pdf_load_embedded_file_contents(
                context(),
                filespec.inner
            ))
        }?;
        let mut buffer = unsafe { Buffer::from_raw(buffer) };
        let mut contents = Vec::with_capacity(buffer.len());
        buffer.read_to_end(&mut contents)?;
        Ok(Some(contents))
    }

    pub fn verify_embedded_file_checksum(&self, name: &str) -> Result<Option<bool>, Error> {
        let Some(filespec) = self.named_embedded_file_object(name)? else {
            return Ok(None);
        };
        unsafe {
            ffi_try!(mupdf_pdf_verify_embedded_file_checksum(
                context(),
                filespec.inner
            ))
        }
        .map(|valid| Some(valid != 0))
    }

    pub fn delete_embedded_file(&mut self, name: &str) -> Result<bool, Error> {
        let Some(mut array) = self.embedded_files_array()? else {
            return Ok(false);
        };
        let len = array.len()?;
        let mut index = 0;
        while index + 1 < len {
            let key_matches = array
                .get_array(index as i32)?
                .and_then(|key| key.as_string().ok().map(|key| key == name))
                .unwrap_or(false);
            if key_matches {
                array.array_delete((index + 1) as i32)?;
                array.array_delete(index as i32)?;
                return Ok(true);
            }
            index += 2;
        }
        Ok(false)
    }

    fn page_count_usize(&self) -> Result<usize, Error> {
        usize::try_from(self.page_count()?).map_err(|_| {
            Error::InvalidArgument("document page count cannot be represented as usize".to_owned())
        })
    }

    fn checked_page_index(&self, page: usize) -> Result<i32, Error> {
        let page_count = self.page_count_usize()?;
        if page >= page_count {
            return Err(Error::InvalidArgument(format!(
                "page index {page} exceeds page count {page_count}"
            )));
        }
        i32::try_from(page).map_err(|_| {
            Error::InvalidArgument(format!("page index {page} cannot be represented as i32"))
        })
    }

    fn checked_insert_index(&self, position: InsertPosition) -> Result<usize, Error> {
        position.resolve(self.page_count_usize()?)
    }

    pub(crate) fn checked_xref(&self, xref: i32) -> Result<(), Error> {
        if xref <= 0 {
            return Err(Error::InvalidArgument(format!(
                "xref {xref} must be positive"
            )));
        }
        let xref_len = i32::try_from(self.count_objects()?).map_err(|_| {
            Error::InvalidArgument("xref length cannot be represented as i32".to_owned())
        })?;
        if xref >= xref_len {
            return Err(Error::InvalidArgument(format!(
                "xref {xref} exceeds xref length {xref_len}"
            )));
        }
        Ok(())
    }

    fn graft_page_raw(
        &mut self,
        src: *mut pdf_document,
        source_index: usize,
        target_index: usize,
    ) -> Result<(), Error> {
        let source_index = i32::try_from(source_index).map_err(|_| {
            Error::InvalidArgument(format!(
                "source page index {source_index} cannot be represented as i32"
            ))
        })?;
        let target_index = i32::try_from(target_index).map_err(|_| {
            Error::InvalidArgument(format!(
                "target page index {target_index} cannot be represented as i32"
            ))
        })?;
        unsafe {
            ffi_try!(mupdf_pdf_graft_page(
                context(),
                self.inner,
                target_index,
                src,
                source_index
            ))
        }
    }

    fn delete_page_range_raw(&mut self, range: PageRange) -> Result<(), Error> {
        let start = i32::try_from(range.start).map_err(|_| {
            Error::InvalidArgument(format!(
                "range start {} cannot be represented as i32",
                range.start
            ))
        })?;
        let end = i32::try_from(range.end).map_err(|_| {
            Error::InvalidArgument(format!(
                "range end {} cannot be represented as i32",
                range.end
            ))
        })?;
        unsafe {
            ffi_try!(mupdf_pdf_delete_page_range(
                context(),
                self.inner,
                start,
                end
            ))
        }
    }

    pub fn insert_pdf(
        &mut self,
        src: &PdfDocument,
        options: InsertPdfOptions,
    ) -> Result<InsertPdfResult, Error> {
        options.validate_supported()?;

        let source_indices = options
            .source_pages
            .validated_indices(src.page_count_usize()?)?;
        let mut target_index = self.checked_insert_index(options.target)?;
        let inserted_start = target_index;

        let operation = DocOperation::begin(self, "Insert PDF pages")?;
        for source_index in source_indices.iter().copied() {
            operation
                .doc
                .graft_page_raw(src.as_raw(), source_index, target_index)?;
            if let Some(rotate) = options.rotate {
                operation
                    .doc
                    .load_pdf_page(i32::try_from(target_index).map_err(|_| {
                        Error::InvalidArgument(format!(
                            "target page index {target_index} cannot be represented as i32"
                        ))
                    })?)?
                    .set_rotation(rotate)?;
            }
            target_index += 1;
        }
        operation.commit()?;

        Ok(InsertPdfResult {
            inserted_pages: PageRange::new(inserted_start, target_index),
            page_count: target_index - inserted_start,
        })
    }

    pub fn copy_page(
        &mut self,
        source_index: usize,
        target: InsertPosition,
    ) -> Result<InsertPdfResult, Error> {
        self.checked_page_index(source_index)?;
        let target_index = self.checked_insert_index(target)?;

        let operation = DocOperation::begin(self, "Copy PDF page")?;
        let src = operation.doc.as_raw();
        operation
            .doc
            .graft_page_raw(src, source_index, target_index)?;
        operation.commit()?;

        Ok(InsertPdfResult {
            inserted_pages: PageRange::new(target_index, target_index + 1),
            page_count: 1,
        })
    }

    pub fn duplicate_page(&mut self, source_index: usize) -> Result<InsertPdfResult, Error> {
        self.copy_page(source_index, InsertPosition::After(source_index))
    }

    pub fn move_page(&mut self, source_index: usize, target_index: usize) -> Result<(), Error> {
        let source_index_i32 = self.checked_page_index(source_index)?;
        let page_count = self.page_count_usize()?;
        if target_index >= page_count {
            return Err(Error::InvalidArgument(format!(
                "target page index {target_index} exceeds page count {page_count}"
            )));
        }
        if source_index == target_index {
            return Ok(());
        }
        let target_index_i32 = i32::try_from(target_index).map_err(|_| {
            Error::InvalidArgument(format!(
                "target page index {target_index} cannot be represented as i32"
            ))
        })?;

        let operation = DocOperation::begin(self, "Move PDF page")?;
        let page = operation.doc.find_page(source_index_i32)?;
        unsafe {
            ffi_try!(mupdf_pdf_delete_page(
                context(),
                operation.doc.inner,
                source_index_i32
            ))?;
            ffi_try!(mupdf_pdf_insert_page(
                context(),
                operation.doc.inner,
                target_index_i32,
                page.inner
            ))?;
        }
        operation.commit()
    }

    pub fn delete_pages(&mut self, selection: impl Into<PageSelection>) -> Result<(), Error> {
        let mut pages = selection
            .into()
            .validated_unique_sorted_indices(self.page_count_usize()?)?;

        let operation = DocOperation::begin(self, "Delete PDF pages")?;
        while let Some(end_page) = pages.pop() {
            let mut start_page = end_page;
            while pages
                .last()
                .is_some_and(|previous| *previous + 1 == start_page)
            {
                start_page = pages.pop().unwrap();
            }
            operation
                .doc
                .delete_page_range_raw(PageRange::new(start_page, end_page + 1))?;
        }
        operation.commit()
    }

    pub fn select_pages(&mut self, selection: impl Into<PageSelection>) -> Result<(), Error> {
        let page_count = self.page_count_usize()?;
        let selected = selection
            .into()
            .validated_unique_sorted_indices(page_count)?;
        let selected: BTreeSet<_> = selected.into_iter().collect();
        let pages_to_delete: Vec<_> = (0..page_count)
            .filter(|page| !selected.contains(page))
            .collect();
        if pages_to_delete.is_empty() {
            return Ok(());
        }
        self.delete_pages(PageSelection::Pages(pages_to_delete))
    }

    pub fn reload_page(&self, page_index: usize) -> Result<PdfPage, Error> {
        self.load_pdf_page(self.checked_page_index(page_index)?)
    }

    pub fn page_label(&self, page_index: usize) -> Result<String, Error> {
        let page_index = self.checked_page_index(page_index)?;
        let mut buf = [0 as c_char; 128];
        unsafe {
            ffi_try!(mupdf_pdf_page_label(
                context(),
                self.inner,
                page_index,
                buf.as_mut_ptr(),
                buf.len()
            ))?
        };
        let label = unsafe { CStr::from_ptr(buf.as_ptr()) };
        Ok(label.to_string_lossy().into_owned())
    }

    pub fn set_page_label_rule(&mut self, rule: PageLabelRule) -> Result<(), Error> {
        let page_count = self.page_count_usize()?;
        if rule.index >= page_count {
            return Err(Error::InvalidArgument(format!(
                "page label index {} exceeds page count {page_count}",
                rule.index
            )));
        }
        if rule.start < 1 {
            return Err(Error::InvalidArgument(
                "page label start must be at least 1".to_owned(),
            ));
        }
        let index = i32::try_from(rule.index).map_err(|_| {
            Error::InvalidArgument(format!(
                "page label index {} cannot be represented as i32",
                rule.index
            ))
        })?;
        let prefix = CString::new(rule.prefix)?;
        unsafe {
            ffi_try!(mupdf_pdf_set_page_labels(
                context(),
                self.inner,
                index,
                rule.style.into_raw(),
                prefix.as_ptr(),
                rule.start
            ))
        }
    }

    pub fn xref_len(&self) -> Result<usize, Error> {
        self.count_objects().map(|count| count as usize)
    }

    pub fn xref_object(&self, xref: i32) -> Result<Option<PdfObject>, Error> {
        self.checked_xref(xref)?;
        self.new_indirect(xref, 0)?.resolve()
    }

    pub fn xref_stream(&self, xref: i32) -> Result<Vec<u8>, Error> {
        self.checked_xref(xref)?;
        self.new_indirect(xref, 0)?.read_stream()
    }

    pub fn xref_raw_stream(&self, xref: i32) -> Result<Vec<u8>, Error> {
        self.checked_xref(xref)?;
        self.new_indirect(xref, 0)?.read_raw_stream()
    }

    pub fn extract_image(&self, xref: i32) -> Result<ExtractedImage, Error> {
        self.checked_xref(xref)?;
        let obj = self.new_indirect(xref, 0)?;
        let Some(info) = PdfPage::image_info_from_object(String::new(), &obj)? else {
            return Err(Error::InvalidArgument(format!(
                "xref {xref} does not refer to an image XObject"
            )));
        };
        let encoded = obj.read_raw_stream()?;
        Ok(ExtractedImage {
            xref,
            width: info.width,
            height: info.height,
            bits_per_component: info.bits_per_component,
            color_space: info.color_space,
            filter: info.filter,
            encoded,
        })
    }

    fn optional_content_properties(&self) -> Result<Option<PdfObject>, Error> {
        match self.catalog()?.get_dict("OCProperties")? {
            Some(properties) if properties.is_dict()? => Ok(Some(properties)),
            _ => Ok(None),
        }
    }

    fn ensure_optional_content_properties(&self) -> Result<PdfObject, Error> {
        let mut catalog = self.catalog()?;
        match catalog.get_dict("OCProperties")? {
            Some(properties) if properties.is_dict()? => Ok(properties),
            _ => {
                let properties = self.new_dict()?;
                catalog.dict_put_ref("OCProperties", &properties)?;
                Ok(properties)
            }
        }
    }

    fn ensure_optional_content_default_config(
        &self,
        properties: &mut PdfObject,
    ) -> Result<PdfObject, Error> {
        match properties.get_dict("D")? {
            Some(config) if config.is_dict()? => Ok(config),
            _ => {
                let config = self.new_dict()?;
                properties.dict_put_ref("D", &config)?;
                Ok(config)
            }
        }
    }

    fn ensure_indirect_array_entry(
        &self,
        array_owner: &mut PdfObject,
        key: &str,
        reference: &PdfObject,
        xref: i32,
    ) -> Result<(), Error> {
        let mut array = match array_owner.get_dict(key)? {
            Some(array) if array.is_array()? => array,
            _ => {
                let array = self.new_array()?;
                array_owner.dict_put_ref(key, &array)?;
                array
            }
        };

        for index in 0..array.len()? {
            let Some(item) = array.get_array(index as i32)? else {
                continue;
            };
            if item.is_indirect()? && item.as_indirect()? == xref {
                return Ok(());
            }
        }

        array.array_push_ref(reference)
    }

    fn remove_indirect_array_entry(
        array_owner: &mut PdfObject,
        key: &str,
        xref: i32,
    ) -> Result<(), Error> {
        let Some(mut array) = array_owner.get_dict(key)? else {
            return Ok(());
        };
        if !array.is_array()? {
            return Ok(());
        }

        for index in (0..array.len()?).rev() {
            let Some(item) = array.get_array(index as i32)? else {
                continue;
            };
            if item.is_indirect()? && item.as_indirect()? == xref {
                array.array_delete(index as i32)?;
            }
        }
        Ok(())
    }

    fn optional_content_array_contains(
        array_owner: &PdfObject,
        key: &str,
        xref: i32,
    ) -> Result<bool, Error> {
        let Some(array) = array_owner.get_dict(key)? else {
            return Ok(false);
        };
        if !array.is_array()? {
            return Ok(false);
        }
        for index in 0..array.len()? {
            let Some(item) = array.get_array(index as i32)? else {
                continue;
            };
            if item.is_indirect()? && item.as_indirect()? == xref {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn register_optional_content_group_object(
        &self,
        oc_ref: &PdfObject,
        xref: i32,
    ) -> Result<(), Error> {
        let mut properties = self.ensure_optional_content_properties()?;
        self.ensure_indirect_array_entry(&mut properties, "OCGs", oc_ref, xref)?;
        let mut config = self.ensure_optional_content_default_config(&mut properties)?;
        self.ensure_indirect_array_entry(&mut config, "ON", oc_ref, xref)?;
        self.ensure_indirect_array_entry(&mut config, "Order", oc_ref, xref)
    }

    fn validate_optional_content_ref(
        &self,
        reference: OptionalContentRef,
    ) -> Result<PdfObject, Error> {
        self.checked_xref(reference.xref())?;
        let oc_ref = self.new_indirect(reference.xref(), 0)?;
        let Some(oc_obj) = oc_ref.resolve()? else {
            return Err(Error::InvalidArgument(format!(
                "optional content xref {} does not resolve to an object",
                reference.xref()
            )));
        };
        if !oc_obj.is_dict()? {
            return Err(Error::InvalidArgument(format!(
                "optional content xref {} does not refer to a dictionary",
                reference.xref()
            )));
        }
        let Some(type_obj) = oc_obj.get_dict("Type")? else {
            return Err(Error::InvalidArgument(format!(
                "optional content xref {} is missing /Type",
                reference.xref()
            )));
        };
        if !type_obj.is_name()? {
            return Err(Error::InvalidArgument(format!(
                "optional content xref {} has non-name /Type",
                reference.xref()
            )));
        }
        match type_obj.as_name()? {
            b"OCG" | b"OCMD" => Ok(oc_ref),
            _ => Err(Error::InvalidArgument(format!(
                "optional content xref {} must have /Type /OCG or /OCMD",
                reference.xref()
            ))),
        }
    }

    pub fn add_optional_content_group(
        &mut self,
        name: impl AsRef<str>,
    ) -> Result<OptionalContentRef, Error> {
        let mut ocg = self.new_dict_with_capacity(2)?;
        ocg.dict_put("Type", PdfObject::new_name("OCG")?)?;
        ocg.dict_put("Name", PdfObject::new_string(name.as_ref())?)?;
        let ocg = self.add_object(&ocg)?;
        let xref = ocg.as_indirect()?;
        self.register_optional_content_group_object(&ocg, xref)?;
        OptionalContentRef::new(xref)
    }

    pub fn optional_content_groups(&self) -> Result<Vec<OptionalContentGroup>, Error> {
        let Some(properties) = self.optional_content_properties()? else {
            return Ok(Vec::new());
        };
        let Some(ocgs) = properties.get_dict("OCGs")? else {
            return Ok(Vec::new());
        };
        if !ocgs.is_array()? {
            return Ok(Vec::new());
        }
        let default_config = properties.get_dict("D")?;

        let mut groups = Vec::new();
        for index in 0..ocgs.len()? {
            let Some(reference) = ocgs.get_array(index as i32)? else {
                continue;
            };
            let Ok(xref) = reference.as_indirect() else {
                continue;
            };
            let Some(group) = reference.resolve()? else {
                continue;
            };
            if !group.is_dict()? {
                continue;
            }
            let name = group
                .get_dict("Name")?
                .and_then(|name| name.as_string().ok().map(str::to_owned));
            let enabled = !matches!(
                default_config.as_ref(),
                Some(config) if Self::optional_content_array_contains(config, "OFF", xref)?
            );
            groups.push(OptionalContentGroup {
                reference: OptionalContentRef::new(xref)?,
                name,
                enabled,
            });
        }
        Ok(groups)
    }

    pub fn set_optional_content_enabled(
        &mut self,
        reference: OptionalContentRef,
        enabled: bool,
    ) -> Result<(), Error> {
        let oc_ref = self.validate_optional_content_ref(reference)?;
        let mut properties = self.ensure_optional_content_properties()?;
        self.ensure_indirect_array_entry(&mut properties, "OCGs", &oc_ref, reference.xref())?;
        let mut config = self.ensure_optional_content_default_config(&mut properties)?;
        if enabled {
            Self::remove_indirect_array_entry(&mut config, "OFF", reference.xref())?;
            self.ensure_indirect_array_entry(&mut config, "ON", &oc_ref, reference.xref())?;
        } else {
            Self::remove_indirect_array_entry(&mut config, "ON", reference.xref())?;
            self.ensure_indirect_array_entry(&mut config, "OFF", &oc_ref, reference.xref())?;
        }
        self.ensure_indirect_array_entry(&mut config, "Order", &oc_ref, reference.xref())
    }

    pub fn optional_content_enabled(&self, reference: OptionalContentRef) -> Result<bool, Error> {
        self.validate_optional_content_ref(reference)?;
        let Some(properties) = self.optional_content_properties()? else {
            return Ok(true);
        };
        let Some(config) = properties.get_dict("D")? else {
            return Ok(true);
        };
        Ok(!Self::optional_content_array_contains(
            &config,
            "OFF",
            reference.xref(),
        )?)
    }

    pub fn begin_operation(&self, op: &str) -> Result<(), Error> {
        let c_op = CString::new(op).map_err(|_| Error::InvalidUtf8)?;
        unsafe {
            ffi_try!(mupdf_pdf_begin_operation(
                context(),
                self.inner,
                c_op.as_ptr()
            ))
        }
    }

    pub fn end_operation(&self) -> Result<(), Error> {
        unsafe { ffi_try!(mupdf_pdf_end_operation(context(), self.inner)) }
    }

    pub fn abandon_operation(&self) -> Result<(), Error> {
        unsafe { ffi_try!(mupdf_pdf_abandon_operation(context(), self.inner)) }
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
                .dest
                .map(|dest| {
                    let page = self.find_page(dest.loc.page_number as i32)?;

                    let ctm = page.page_ctm()?;

                    // Use inverse current transformation matrix (CTM) to convert from user
                    // space to PDF page space.
                    //
                    // Since this is a local destination (not remote), we must transform
                    // coordinates from the shared user space back to the specific page space.
                    //
                    // This matches MuPDF's logic in `pdf_new_dest_from_link`
                    // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1328
                    let dest_kind = ctm
                        .invert()
                        .map(|inv_ctm| dest.kind.transform(&inv_ctm))
                        .unwrap_or(dest.kind);
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
        Ok(Self {
            inner,
            doc,
            font_info_cache: RefCell::new(HashMap::new()),
        })
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
    use crate::document::test_document;
    use crate::{Buffer, Size};

    use super::{
        EmbeddedFileOptions, InsertPdfOptions, InsertPosition, OptionalContentRef, PageLabelRule,
        PageLabelStyle, PageSelection, PdfDocument, PdfWriteOptions, Permission,
    };

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
        let doc = test_document!("../..", "files/dummy.pdf" as PdfDocument).unwrap();
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
        let bytes = include_bytes!("../../tests/files/dummy.pdf");
        let doc = PdfDocument::from_bytes(bytes).unwrap();
        assert!(!doc.needs_password().unwrap());
    }

    #[test]
    fn test_pdf_document_bake_document() {
        let mut doc = test_document!("../..", "files/dummy.pdf" as PdfDocument).unwrap();
        doc.bake(false, false).unwrap();
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
        let doc = test_document!("../..", "files/dummy.pdf" as PdfDocument).unwrap();
        let _page = doc.find_page(0).unwrap();
    }

    #[test]
    fn test_pdf_document_named_embedded_files() {
        let mut doc = PdfDocument::new();
        let info = doc
            .add_embedded_file(
                "payload",
                b"hello embedded",
                EmbeddedFileOptions {
                    mime_type: Some("text/plain"),
                    ..EmbeddedFileOptions::new("payload.txt")
                },
            )
            .unwrap();

        assert_eq!(info.name, "payload");
        assert_eq!(info.filename.as_deref(), Some("payload.txt"));
        assert_eq!(info.mime_type.as_deref(), Some("text/plain"));
        assert_eq!(info.size, b"hello embedded".len());

        let files = doc.embedded_files().unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "payload");
        assert_eq!(
            doc.load_embedded_file("payload").unwrap().unwrap(),
            b"hello embedded"
        );
        assert_eq!(
            doc.verify_embedded_file_checksum("payload").unwrap(),
            Some(true)
        );
        assert!(doc.embedded_file("payload").unwrap().is_some());

        assert!(doc.delete_embedded_file("payload").unwrap());
        assert!(doc.embedded_files().unwrap().is_empty());
        assert!(doc.load_embedded_file("payload").unwrap().is_none());
    }

    #[test]
    fn test_pdf_document_insert_copy_delete_and_select_pages() {
        let mut src = PdfDocument::new();
        src.new_page(Size::A4).unwrap();
        src.new_page(Size::A5).unwrap();

        let mut dst = PdfDocument::new();
        dst.new_page(Size::A6).unwrap();

        let result = dst
            .insert_pdf(
                &src,
                InsertPdfOptions {
                    source_pages: PageSelection::from(0..2),
                    target: InsertPosition::Append,
                    rotate: Some(90),
                    ..InsertPdfOptions::default()
                },
            )
            .unwrap();
        assert_eq!(result.page_count, 2);
        assert_eq!(result.inserted_pages.start, 1);
        assert_eq!(result.inserted_pages.end, 3);
        assert_eq!(dst.page_count().unwrap(), 3);
        assert_eq!(dst.load_pdf_page(1).unwrap().rotation().unwrap(), 90);

        dst.copy_page(0, InsertPosition::Append).unwrap();
        assert_eq!(dst.page_count().unwrap(), 4);

        dst.delete_pages(1..3).unwrap();
        assert_eq!(dst.page_count().unwrap(), 2);

        dst.select_pages(PageSelection::Pages(vec![0])).unwrap();
        assert_eq!(dst.page_count().unwrap(), 1);
    }

    #[test]
    fn test_pdf_document_page_labels() {
        let mut doc = PdfDocument::new();
        doc.new_page(Size::A4).unwrap();
        doc.new_page(Size::A4).unwrap();

        doc.set_page_label_rule(PageLabelRule {
            index: 0,
            style: PageLabelStyle::LowerRoman,
            prefix: "intro-".to_owned(),
            start: 1,
        })
        .unwrap();

        assert_eq!(doc.page_label(0).unwrap(), "intro-i");
        assert_eq!(doc.page_label(1).unwrap(), "intro-ii");
        assert!(doc
            .set_page_label_rule(PageLabelRule::new(2, PageLabelStyle::Decimal))
            .is_err());
        assert!(doc.page_label(2).is_err());
    }

    #[test]
    fn test_pdf_document_xref_helpers_reject_missing_xrefs() {
        let mut doc = PdfDocument::new();
        doc.new_page(Size::A4).unwrap();

        assert!(doc.xref_object(0).is_err());
        assert!(doc.xref_object(999).is_err());
        assert!(doc.xref_stream(999).is_err());
        assert!(doc.extract_image(999).is_err());
    }

    #[test]
    fn test_pdf_document_optional_content_groups() {
        let mut doc = PdfDocument::new();
        let oc = doc.add_optional_content_group("Layer 1").unwrap();
        assert!(oc.xref() > 0);

        let groups = doc.optional_content_groups().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].reference, oc);
        assert_eq!(groups[0].name.as_deref(), Some("Layer 1"));
        assert!(groups[0].enabled);
        assert!(doc.optional_content_enabled(oc).unwrap());

        doc.set_optional_content_enabled(oc, false).unwrap();
        assert!(!doc.optional_content_enabled(oc).unwrap());
        assert!(!doc.optional_content_groups().unwrap()[0].enabled);

        doc.set_optional_content_enabled(oc, true).unwrap();
        assert!(doc.optional_content_enabled(oc).unwrap());

        let mut page = doc.new_page(Size::A4).unwrap();
        let name = page.register_optional_content_ref(&mut doc, oc).unwrap();
        assert_eq!(name, "/P0");
        let properties = page
            .resources()
            .unwrap()
            .get_dict("Properties")
            .unwrap()
            .unwrap();
        assert!(properties.get_dict("P0").unwrap().is_some());

        let missing = OptionalContentRef::new(999).unwrap();
        assert!(doc.optional_content_enabled(missing).is_err());
        assert!(doc.set_optional_content_enabled(missing, true).is_err());
    }

    #[test]
    fn test_pdf_document_rejects_invalid_page_selection() {
        let mut doc = PdfDocument::new();
        doc.new_page(Size::A4).unwrap();

        assert!(doc.delete_pages(PageSelection::Pages(vec![0, 0])).is_err());
        assert!(doc.delete_pages(1..2).is_err());
    }

    #[test]
    fn test_pdf_document_add_stream_returns_indirect_stream() {
        let mut pdf = PdfDocument::new();

        for compressed in [false, true] {
            let buf = Buffer::from_bytes(b"hello world").unwrap();
            let obj = pdf.add_stream(&buf, None, compressed).unwrap();

            assert!(obj.is_indirect().unwrap());
            assert!(obj.as_indirect().unwrap() > 0);
            assert!(obj.is_stream().unwrap());
        }
    }

    #[test]
    fn test_pdf_document_add_stream_preserves_payload_bytes() {
        let mut pdf = PdfDocument::new();
        let payloads = [
            Vec::new(),
            b"q 1 0 0 1 10 10 cm Q\n".to_vec(),
            (0..4096)
                .scan(0x1234_5678_u32, |state, _| {
                    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    Some((*state >> 24) as u8)
                })
                .collect::<Vec<_>>(),
        ];

        for payload in payloads {
            let buf = Buffer::from_bytes(&payload).unwrap();
            let mut obj = pdf.add_stream(&buf, None, false).unwrap();
            assert_eq!(obj.read_stream().unwrap(), payload);

            let rewritten = Buffer::from_bytes(&payload).unwrap();
            obj.write_stream_buffer(&rewritten).unwrap();
            assert_eq!(obj.read_stream().unwrap(), payload);
        }
    }

    #[test]
    fn test_pdf_document_add_stream_accepts_optional_dict() {
        let mut pdf = PdfDocument::new();
        let mut dict = pdf.new_dict().unwrap();
        dict.dict_put("X-Test", pdf.new_string("foo").unwrap())
            .unwrap();

        let buf = Buffer::from_bytes(b"dict payload").unwrap();
        let obj = pdf.add_stream(&buf, Some(&dict), false).unwrap();
        let sentinel = obj.get_dict("X-Test").unwrap().unwrap();

        assert_eq!(sentinel.as_string().unwrap(), "foo");
        assert_eq!(sentinel.to_string(), "(foo)");
        assert_eq!(obj.read_stream().unwrap(), b"dict payload");
    }
}

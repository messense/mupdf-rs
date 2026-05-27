use std::collections::{hash_map::DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::str;
use std::{
    ffi::CStr,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use mupdf_sys::*;

use crate::link::LinkDestination;
use crate::pdf::links::{
    build_link_annotation, parse_external_link, CachedResolver, DestPageResolver,
};
use crate::pdf::{
    DocOperation, LinkAction, PdfAction, PdfAnnotation, PdfAnnotationType, PdfDestination,
    PdfDocument, PdfFilterOptions, PdfLink, PdfLinkAnnot, PdfObject,
};
use crate::{
    context, unsafe_impl_ffi_wrapper, Buffer, CjkFontOrdering, Error, FFIWrapper, Font, Matrix,
    Page, Rect, SimpleFontEncoding, WriteMode,
};

#[derive(Clone, Debug, PartialEq)]
pub struct FontInfo {
    pub ascender: f32,
    pub descender: f32,
    pub glyphs: Option<HashMap<u32, i32>>,
    pub simple: bool,
    pub ordering: Option<CjkFontOrdering>,
    pub name: String,
    pub encoding: SimpleFontEncoding,
    pub wmode: WriteMode,
    pub serif: bool,
    pub fontfile_hash: Option<u64>,
}

#[derive(Clone, Copy, Debug)]
pub struct InsertFontOptions<'a> {
    pub name: &'a str,
    pub fontfile: Option<&'a [u8]>,
    pub simple: bool,
    pub encoding: SimpleFontEncoding,
    pub ordering: Option<CjkFontOrdering>,
    pub wmode: WriteMode,
    pub serif: bool,
}

impl<'a> InsertFontOptions<'a> {
    pub fn new(name: &'a str) -> Self {
        Self {
            name,
            fontfile: None,
            simple: true,
            encoding: SimpleFontEncoding::Latin,
            ordering: None,
            wmode: WriteMode::Horizontal,
            serif: false,
        }
    }
}

fn canonical_base14_name(name: &str) -> Option<&'static str> {
    match name {
        "helv" | "Helvetica" => Some("Helvetica"),
        "heit" | "Helvetica-Oblique" => Some("Helvetica-Oblique"),
        "hebo" | "Helvetica-Bold" => Some("Helvetica-Bold"),
        "heboit" | "Helvetica-BoldOblique" => Some("Helvetica-BoldOblique"),
        "cour" | "Courier" => Some("Courier"),
        "coit" | "Courier-Oblique" => Some("Courier-Oblique"),
        "cobo" | "Courier-Bold" => Some("Courier-Bold"),
        "coboit" | "Courier-BoldOblique" => Some("Courier-BoldOblique"),
        "tiro" | "Times-Roman" => Some("Times-Roman"),
        "tibo" | "Times-Bold" => Some("Times-Bold"),
        "tiit" | "Times-Italic" => Some("Times-Italic"),
        "tibi" | "Times-BoldItalic" => Some("Times-BoldItalic"),
        "symb" | "Symbol" => Some("Symbol"),
        "zadb" | "ZapfDingbats" => Some("ZapfDingbats"),
        _ => None,
    }
}

#[derive(Debug)]
pub struct PdfPage {
    pub(crate) inner: NonNull<pdf_page>,
    // Technically, this struct is self-referential as `self.inner` and `(*self.page).inner` point
    // to the same location in memory (since the first field of `pdf_page` is an `fz_page`, and
    // `(*self.page).inner` is a pointer to the first field if `self.inner`). So, to avoid a
    // double-free situation, we're storing this `Page` as `ManuallyDrop` so its destructor is
    // never called, and we call `pdf_drop_page` on `inner` to drop its inner `fz_page` along with
    // all the other stuff it may or may not contain.
    // This also means that we need to make sure to not access `page` at all besides through the
    // `Deref` and `DerefMut` traits. If we use both `inner` and `page` in the body of a function,
    // we might run into some nasty mutable aliasing problems, so we need to make sure we're using
    // `Deref(Mut)` to ensure we aren't accessing any other fields whenever we mutably access
    // `page` (or that we aren't accessing `page` when we're mutably accessing `inner`).
    page: ManuallyDrop<Page>,
}

unsafe_impl_ffi_wrapper!(PdfPage, pdf_page, pdf_drop_page);

impl PdfPage {
    /// # Safety
    ///
    /// * `ptr` must point to a valid, well-aligned instance of [`pdf_page`]
    pub(crate) unsafe fn from_raw(ptr: NonNull<pdf_page>) -> Self {
        Self {
            inner: ptr,
            // This cast is safe because the first member of the `pdf_page` struct is a `fz_page`
            // SAFETY: Upheld by caller
            page: ManuallyDrop::new(unsafe { Page::from_non_null(ptr.cast()) }),
        }
    }

    pub fn create_annotation(
        &mut self,
        subtype: PdfAnnotationType,
    ) -> Result<PdfAnnotation, Error> {
        unsafe {
            ffi_try!(mupdf_pdf_create_annot(
                context(),
                self.as_mut_ptr(),
                subtype as i32
            ))
        }
        .map(|annot| {
            // SAFETY: `mupdf_pdf_create_annot` returns an owned pointer (refcount = 1)
            // and `self` (the page) is alive.
            unsafe { PdfAnnotation::from_raw(annot) }
        })
    }

    pub fn delete_annotation(&mut self, annot: PdfAnnotation) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_delete_annot(
                context(),
                self.as_mut_ptr(),
                annot.inner.as_ptr()
            ))
        }
    }

    pub fn annotations(&self) -> AnnotationIter<'_> {
        let next = unsafe { pdf_first_annot(context(), self.as_ptr().cast_mut()) };
        AnnotationIter {
            next: NonNull::new(next),
            marker: PhantomData,
        }
    }

    pub fn update(&mut self) -> Result<bool, Error> {
        unsafe { ffi_try!(mupdf_pdf_update_page(context(), self.as_mut_ptr())) }
    }

    pub fn redact(&mut self) -> Result<bool, Error> {
        unsafe { ffi_try!(mupdf_pdf_redact_page(context(), self.as_mut_ptr())) }
    }

    pub fn object(&self) -> PdfObject {
        unsafe { PdfObject::from_raw_keep_ref(self.as_ref().obj) }
    }

    pub(crate) fn document_handle(&self) -> Result<PdfDocument, Error> {
        let doc_ptr =
            NonNull::new(unsafe { (*self.inner.as_ptr()).doc }).ok_or(Error::UnexpectedNullPtr)?;
        Ok(unsafe { PdfDocument::from_raw(pdf_keep_document(context(), doc_ptr.as_ptr())) })
    }

    /// Returns this page's `/Contents` object.
    ///
    /// The returned object is the page dictionary value as-is: a single stream, an array of
    /// streams, or `None` when the page has no `/Contents` entry.
    pub fn contents(&self) -> Result<Option<PdfObject>, Error> {
        self.object().get_dict("Contents")
    }

    /// Returns this page's `/Resources` dictionary, creating and attaching one when missing.
    ///
    /// Missing or non-dictionary `/Resources` entries are replaced by a new empty indirect
    /// dictionary. Repeated calls return the same dictionary object without allocating again.
    pub fn resources(&self) -> Result<PdfObject, Error> {
        let mut page_obj = self.object();

        if let Some(resources) = page_obj.get_dict("Resources")? {
            if resources.is_dict()? {
                return Ok(resources);
            }
        }

        let mut doc = self.document_handle()?;
        let resources = doc.new_dict()?;
        let resources = doc.add_object(&resources)?;
        page_obj.dict_put("Resources", resources.clone())?;

        Ok(resources)
    }

    fn push_contents_into_array(
        contents_array: &mut PdfObject,
        contents: &PdfObject,
    ) -> Result<(), Error> {
        if contents.is_null()? {
            return Ok(());
        }

        if contents.is_array()? {
            for index in 0..contents.len()? {
                if let Some(item) = contents.get_array(index as i32)? {
                    contents_array.array_push(item)?;
                }
            }
        } else {
            contents_array.array_push(contents.clone())?;
        }

        Ok(())
    }

    /// Inserts a new content stream into this page's `/Contents` array.
    ///
    /// Creates a new PDF stream containing `bytes`, promotes a missing, null, or single-stream
    /// `/Contents` entry to an array, then appends it when `overlay` is true or prepends it when
    /// `overlay` is false. Returns the xref number of the newly created stream.
    ///
    /// This ports PyMuPDF's `JM_insert_contents` helper used by `Shape.commit`.
    pub fn insert_contents(
        &mut self,
        doc: &mut PdfDocument,
        bytes: &[u8],
        overlay: bool,
    ) -> Result<i32, Error> {
        assert_eq!(
            doc.as_raw(),
            unsafe { (*self.inner.as_ptr()).doc },
            "PdfPage ownership mismatch: the page is not attached to the provided PdfDocument"
        );

        let operation = DocOperation::begin(doc, "Insert contents")?;
        let mut page_obj = self.object();
        let old_contents = page_obj.get_dict("Contents")?;

        let stream_buf = Buffer::from_bytes(bytes)?;
        let new_stream = operation.doc.add_stream(&stream_buf, None, false)?;
        let new_xref = new_stream.as_indirect()?;

        let old_len = match old_contents.as_ref() {
            Some(contents) if contents.is_array()? => contents.len()? as i32,
            Some(contents) if !contents.is_null()? => 1,
            _ => 0,
        };
        let mut contents_array = operation.doc.new_array_with_capacity(old_len + 1)?;

        if overlay {
            if let Some(contents) = &old_contents {
                Self::push_contents_into_array(&mut contents_array, contents)?;
            }
            contents_array.array_push(new_stream)?;
        } else {
            contents_array.array_push(new_stream)?;
            if let Some(contents) = &old_contents {
                Self::push_contents_into_array(&mut contents_array, contents)?;
            }
        }

        page_obj.dict_put("Contents", contents_array)?;
        operation.commit()?;

        Ok(new_xref)
    }

    /// Wraps this page's existing content streams in a balanced PDF graphics state.
    ///
    /// Inserts a `q\n` content stream as an underlay and a `Q\n` content stream as an
    /// overlay. This intentionally is not idempotent: repeated calls add another balanced
    /// pair around the existing contents.
    pub fn wrap_contents(&mut self, doc: &mut PdfDocument) -> Result<(), Error> {
        self.insert_contents(doc, b"q\n", false)?;
        self.insert_contents(doc, b"Q\n", true)?;
        Ok(())
    }

    fn validate_opacity_value(value: f32, role: &str) -> Result<(), Error> {
        if value.is_finite() && (0.0..=1.0).contains(&value) {
            return Ok(());
        }

        Err(Error::InvalidArgument(format!(
            "{role} opacity must be in the range [0.0, 1.0]"
        )))
    }

    pub(crate) fn validate_opacity_pair(
        stroke_opacity: Option<f32>,
        fill_opacity: Option<f32>,
    ) -> Result<(), Error> {
        if let Some(stroke_opacity) = stroke_opacity {
            Self::validate_opacity_value(stroke_opacity, "stroke")?;
        }
        if let Some(fill_opacity) = fill_opacity {
            Self::validate_opacity_value(fill_opacity, "fill")?;
        }
        Ok(())
    }

    fn ext_gstate_alpha_matches(
        ext_gstate: &PdfObject,
        key: &str,
        expected: Option<f32>,
    ) -> Result<bool, Error> {
        let Some(actual) = ext_gstate.get_dict(key)? else {
            return Ok(expected.is_none());
        };

        let Some(expected) = expected else {
            return Ok(false);
        };

        Ok(actual.is_number()? && (actual.as_float()? - expected).abs() <= 1e-6)
    }

    fn ext_gstate_matches(
        ext_gstate: &PdfObject,
        stroke_opacity: Option<f32>,
        fill_opacity: Option<f32>,
    ) -> Result<bool, Error> {
        Ok(
            Self::ext_gstate_alpha_matches(ext_gstate, "CA", stroke_opacity)?
                && Self::ext_gstate_alpha_matches(ext_gstate, "ca", fill_opacity)?,
        )
    }

    fn find_ext_gstate_resource(
        ext_gstates: &PdfObject,
        stroke_opacity: Option<f32>,
        fill_opacity: Option<f32>,
    ) -> Result<Option<String>, Error> {
        for idx in 0..ext_gstates.dict_len()? {
            let Some(key) = ext_gstates.get_dict_key(idx as i32)? else {
                continue;
            };
            let Ok(key_name) = str::from_utf8(key.as_name()?) else {
                continue;
            };
            let Some(value) = ext_gstates.get_dict_val(idx as i32)? else {
                continue;
            };
            if value.is_dict()? && Self::ext_gstate_matches(&value, stroke_opacity, fill_opacity)? {
                return Ok(Some(format!("/{key_name}")));
            }
        }

        Ok(None)
    }

    fn next_ext_gstate_resource_slot(ext_gstates: Option<&PdfObject>) -> Result<String, Error> {
        let mut used = Vec::new();

        if let Some(ext_gstates) = ext_gstates {
            for idx in 0..ext_gstates.dict_len()? {
                let Some(key) = ext_gstates.get_dict_key(idx as i32)? else {
                    continue;
                };
                let Ok(key_name) = str::from_utf8(key.as_name()?) else {
                    continue;
                };
                let Some(index) = key_name
                    .strip_prefix('A')
                    .filter(|suffix| !suffix.is_empty())
                    .and_then(|suffix| suffix.parse::<usize>().ok())
                else {
                    continue;
                };
                used.push(index);
            }
        }

        let mut index = 0;
        while used.contains(&index) {
            index += 1;
        }
        Ok(format!("A{index}"))
    }

    pub(crate) fn validate_optional_content_xref(
        doc: &PdfDocument,
        oc_xref: i32,
    ) -> Result<(), Error> {
        let oc_ref = doc.new_indirect(oc_xref, 0)?;
        let Some(oc_obj) = oc_ref.resolve()? else {
            return Err(Error::InvalidArgument(format!(
                "optional content xref {oc_xref} does not resolve to an object"
            )));
        };
        if !oc_obj.is_dict()? {
            return Err(Error::InvalidArgument(format!(
                "optional content xref {oc_xref} must point to an OCG or OCMD dictionary"
            )));
        }

        let Some(type_obj) = oc_obj.get_dict("Type")? else {
            return Err(Error::InvalidArgument(format!(
                "optional content xref {oc_xref} is missing /Type"
            )));
        };
        if !type_obj.is_name()? {
            return Err(Error::InvalidArgument(format!(
                "optional content xref {oc_xref} has non-name /Type"
            )));
        }

        match type_obj.as_name()? {
            b"OCG" | b"OCMD" => Ok(()),
            _ => Err(Error::InvalidArgument(format!(
                "optional content xref {oc_xref} must have /Type /OCG or /OCMD"
            ))),
        }
    }

    fn find_optional_content_resource(
        properties: &PdfObject,
        oc_xref: i32,
    ) -> Result<Option<String>, Error> {
        for idx in 0..properties.dict_len()? {
            let Some(key) = properties.get_dict_key(idx as i32)? else {
                continue;
            };
            let Ok(key_name) = str::from_utf8(key.as_name()?) else {
                continue;
            };
            let Some(value) = properties.get_dict_val(idx as i32)? else {
                continue;
            };
            if value.is_indirect()? && value.as_indirect()? == oc_xref {
                return Ok(Some(format!("/{key_name}")));
            }
        }

        Ok(None)
    }

    fn next_optional_content_resource_slot(
        properties: Option<&PdfObject>,
    ) -> Result<String, Error> {
        let mut used = Vec::new();

        if let Some(properties) = properties {
            for idx in 0..properties.dict_len()? {
                let Some(key) = properties.get_dict_key(idx as i32)? else {
                    continue;
                };
                let Ok(key_name) = str::from_utf8(key.as_name()?) else {
                    continue;
                };
                let Some(index) = key_name
                    .strip_prefix('P')
                    .filter(|suffix| !suffix.is_empty())
                    .and_then(|suffix| suffix.parse::<usize>().ok())
                else {
                    continue;
                };
                used.push(index);
            }
        }

        let mut index = 0;
        while used.contains(&index) {
            index += 1;
        }
        Ok(format!("P{index}"))
    }

    /// Registers or reuses a page `/ExtGState` entry for stroke and fill opacity.
    ///
    /// Returns a PDF resource name (including the leading slash) when either opacity is set.
    /// Identical `(stroke_opacity, fill_opacity)` pairs reuse the same `/ExtGState` dictionary,
    /// while combined stroke and fill opacity values are stored in one dictionary.
    pub fn register_ext_gstate(
        &mut self,
        doc: &mut PdfDocument,
        stroke_opacity: Option<f32>,
        fill_opacity: Option<f32>,
    ) -> Result<Option<String>, Error> {
        assert_eq!(
            doc.as_raw(),
            unsafe { (*self.inner.as_ptr()).doc },
            "PdfPage ownership mismatch: the page is not attached to the provided PdfDocument"
        );

        Self::validate_opacity_pair(stroke_opacity, fill_opacity)?;
        if stroke_opacity.is_none() && fill_opacity.is_none() {
            return Ok(None);
        }

        let mut resources = self.resources()?;
        let existing_ext_gstates = match resources.get_dict("ExtGState")? {
            Some(ext_gstates) if ext_gstates.is_dict()? => Some(ext_gstates),
            _ => None,
        };

        if let Some(ext_gstates) = existing_ext_gstates.as_ref() {
            if let Some(name) =
                Self::find_ext_gstate_resource(ext_gstates, stroke_opacity, fill_opacity)?
            {
                return Ok(Some(name));
            }
        }

        let slot_name = Self::next_ext_gstate_resource_slot(existing_ext_gstates.as_ref())?;
        let mut ext_gstates = if let Some(ext_gstates) = existing_ext_gstates {
            ext_gstates
        } else {
            let ext_gstates = doc.new_dict()?;
            resources.dict_put_ref("ExtGState", &ext_gstates)?;
            ext_gstates
        };

        let mut ext_gstate = doc.new_dict_with_capacity(3)?;
        ext_gstate.dict_put("Type", PdfObject::new_name("ExtGState")?)?;
        if let Some(stroke_opacity) = stroke_opacity {
            ext_gstate.dict_put("CA", PdfObject::new_real(stroke_opacity)?)?;
        }
        if let Some(fill_opacity) = fill_opacity {
            ext_gstate.dict_put("ca", PdfObject::new_real(fill_opacity)?)?;
        }
        ext_gstates.dict_put(slot_name.as_str(), ext_gstate)?;

        Ok(Some(format!("/{slot_name}")))
    }

    /// Registers or reuses a page `/Properties` entry for optional content.
    ///
    /// Returns a PDF resource name (including the leading slash). The supplied xref must
    /// resolve to an OCG or OCMD dictionary. Reusing the same xref on the same page returns
    /// the existing `/Properties` slot without adding another resource entry.
    pub fn register_optional_content(
        &mut self,
        doc: &mut PdfDocument,
        oc_xref: i32,
    ) -> Result<String, Error> {
        assert_eq!(
            doc.as_raw(),
            unsafe { (*self.inner.as_ptr()).doc },
            "PdfPage ownership mismatch: the page is not attached to the provided PdfDocument"
        );

        Self::validate_optional_content_xref(doc, oc_xref)?;
        let oc_ref = doc.new_indirect(oc_xref, 0)?;

        let mut resources = self.resources()?;
        let existing_properties = match resources.get_dict("Properties")? {
            Some(properties) if properties.is_dict()? => Some(properties),
            _ => None,
        };

        if let Some(properties) = existing_properties.as_ref() {
            if let Some(name) = Self::find_optional_content_resource(properties, oc_xref)? {
                return Ok(name);
            }
        }

        let slot_name = Self::next_optional_content_resource_slot(existing_properties.as_ref())?;
        let mut properties = if let Some(properties) = existing_properties {
            properties
        } else {
            let properties = doc.new_dict()?;
            resources.dict_put_ref("Properties", &properties)?;
            properties
        };

        properties.dict_put_ref(slot_name.as_str(), &oc_ref)?;

        Ok(format!("/{slot_name}"))
    }

    fn cached_font_matches(info: &FontInfo, name: &str, opts: &InsertFontOptions<'_>) -> bool {
        let opts_simple = opts.simple && opts.ordering.is_none();
        if info.name != name
            || info.simple != opts_simple
            || info.ordering != opts.ordering
            || info.fontfile_hash != Self::fontfile_hash(opts.fontfile)
        {
            return false;
        }

        if info.simple && info.encoding != opts.encoding {
            return false;
        }

        if info.ordering.is_some() && (info.wmode != opts.wmode || info.serif != opts.serif) {
            return false;
        }

        true
    }

    fn fontfile_hash(fontfile: Option<&[u8]>) -> Option<u64> {
        fontfile.map(|bytes| {
            let mut hasher = DefaultHasher::new();
            bytes.hash(&mut hasher);
            hasher.finish()
        })
    }

    fn normalized_base_font_name(name: &str) -> &str {
        match name.split_once('+') {
            Some((prefix, suffix))
                if prefix.len() == 6 && prefix.bytes().all(|byte| byte.is_ascii_uppercase()) =>
            {
                suffix
            }
            _ => name,
        }
    }

    fn resource_font_base_name(font_obj: &PdfObject) -> Result<Option<String>, Error> {
        if let Some(base_font) = font_obj.get_dict("BaseFont")? {
            let base_font = str::from_utf8(base_font.as_name()?)
                .map(Self::normalized_base_font_name)
                .map(str::to_owned)
                .ok();
            if base_font.is_some() {
                return Ok(base_font);
            }
        }

        let Some(descendant_fonts) = font_obj.get_dict("DescendantFonts")? else {
            return Ok(None);
        };
        let Some(descendant_font) = descendant_fonts.get_array(0)? else {
            return Ok(None);
        };
        let Some(base_font) = descendant_font.get_dict("BaseFont")? else {
            return Ok(None);
        };

        Ok(str::from_utf8(base_font.as_name()?)
            .map(Self::normalized_base_font_name)
            .map(str::to_owned)
            .ok())
    }

    fn resource_simple_font_encoding(
        font_obj: &PdfObject,
    ) -> Result<Option<SimpleFontEncoding>, Error> {
        let Some(encoding) = font_obj.get_dict("Encoding")? else {
            return Ok(Some(SimpleFontEncoding::Latin));
        };

        if !encoding.is_dict()? {
            let Ok(encoding_name) = encoding.as_name() else {
                return Ok(None);
            };
            let Ok(encoding_name) = str::from_utf8(encoding_name) else {
                return Ok(None);
            };
            return match encoding_name {
                "WinAnsiEncoding" => Ok(Some(SimpleFontEncoding::Latin)),
                _ => Ok(None),
            };
        }

        let Some(differences) = encoding.get_dict("Differences")? else {
            return Ok(None);
        };

        for index in 0..differences.len()? {
            let Some(item) = differences.get_array(index as i32)? else {
                continue;
            };
            let Ok(name) = item.as_name() else {
                continue;
            };
            let Ok(name) = str::from_utf8(name) else {
                continue;
            };
            if name.contains("cyrillic") || name.starts_with("afii10") {
                return Ok(Some(SimpleFontEncoding::Cyrillic));
            }
            if matches!(
                name,
                "Alpha"
                    | "Beta"
                    | "Gamma"
                    | "Deltagreek"
                    | "Omegagreek"
                    | "alpha"
                    | "beta"
                    | "gamma"
                    | "delta"
                    | "mugreek"
                    | "omega"
                    | "tonos"
            ) {
                return Ok(Some(SimpleFontEncoding::Greek));
            }
        }

        Ok(None)
    }

    fn font_info_from_resource_font(
        font_obj: &PdfObject,
        canonical_name: &str,
        opts: &InsertFontOptions<'_>,
    ) -> Result<Option<FontInfo>, Error> {
        if opts.fontfile.is_some() || opts.ordering.is_some() || !opts.simple {
            return Ok(None);
        }

        let Some(base_name) = Self::resource_font_base_name(font_obj)? else {
            return Ok(None);
        };
        if base_name != canonical_name {
            return Ok(None);
        }

        let Some(encoding) = Self::resource_simple_font_encoding(font_obj)? else {
            return Ok(None);
        };
        let font = Font::new(canonical_name)?;
        let info = FontInfo {
            ascender: font.ascender(),
            descender: font.descender(),
            glyphs: None,
            simple: true,
            encoding,
            ordering: None,
            wmode: WriteMode::Horizontal,
            serif: false,
            fontfile_hash: None,
            name: canonical_name.to_owned(),
        };

        Ok(Self::cached_font_matches(&info, canonical_name, opts).then_some(info))
    }

    fn find_registered_page_font(
        doc: &PdfDocument,
        font_dict: &PdfObject,
        canonical_name: &str,
        opts: &InsertFontOptions<'_>,
    ) -> Result<Option<(String, i32, FontInfo)>, Error> {
        for idx in 0..font_dict.dict_len()? {
            let Some(key) = font_dict.get_dict_key(idx as i32)? else {
                continue;
            };
            let Ok(key_name) = str::from_utf8(key.as_name()?) else {
                continue;
            };
            let Some(value) = font_dict.get_dict_val(idx as i32)? else {
                continue;
            };
            let Ok(xref) = value.as_indirect() else {
                continue;
            };
            if xref <= 0 {
                continue;
            }

            if let Some(info) = doc.font_info_cache.borrow().get(&xref) {
                if Self::cached_font_matches(info, canonical_name, opts) {
                    return Ok(Some((format!("/{key_name}"), xref, info.clone())));
                }
                continue;
            }

            if let Some(info) = Self::font_info_from_resource_font(&value, canonical_name, opts)? {
                doc.font_info_cache.borrow_mut().insert(xref, info.clone());
                return Ok(Some((format!("/{key_name}"), xref, info)));
            }
        }

        Ok(None)
    }

    fn find_cached_document_font(
        doc: &PdfDocument,
        canonical_name: &str,
        opts: &InsertFontOptions<'_>,
    ) -> Option<(i32, FontInfo)> {
        doc.font_info_cache
            .borrow()
            .iter()
            .find_map(|(xref, info)| {
                Self::cached_font_matches(info, canonical_name, opts).then(|| (*xref, info.clone()))
            })
    }

    fn next_font_resource_slot(font_dict: Option<&PdfObject>) -> Result<String, Error> {
        let mut used = Vec::new();

        if let Some(font_dict) = font_dict {
            for idx in 0..font_dict.dict_len()? {
                let Some(key) = font_dict.get_dict_key(idx as i32)? else {
                    continue;
                };
                let Ok(key_name) = str::from_utf8(key.as_name()?) else {
                    continue;
                };
                let Some(index) = key_name
                    .strip_prefix('F')
                    .filter(|suffix| !suffix.is_empty())
                    .and_then(|suffix| suffix.parse::<usize>().ok())
                else {
                    continue;
                };
                used.push(index);
            }
        }

        let mut index = 0;
        while used.contains(&index) {
            index += 1;
        }
        Ok(format!("F{index}"))
    }

    fn build_font_object(
        doc: &mut PdfDocument,
        canonical_name: &str,
        opts: &InsertFontOptions<'_>,
    ) -> Result<(PdfObject, FontInfo), Error> {
        let font = if let Some(font_data) = opts.fontfile {
            Font::from_bytes(canonical_name, font_data)?
        } else {
            Font::new(canonical_name)?
        };

        let font_obj = if let Some(ordering) = opts.ordering {
            doc.add_cjk_font(&font, ordering, opts.wmode, opts.serif)?
        } else if opts.simple {
            doc.add_simple_font(&font, opts.encoding)?
        } else {
            doc.add_font(&font)?
        };

        let info = FontInfo {
            ascender: font.ascender(),
            descender: font.descender(),
            glyphs: None,
            simple: opts.simple && opts.ordering.is_none(),
            encoding: opts.encoding,
            ordering: opts.ordering,
            wmode: opts.wmode,
            serif: opts.serif,
            fontfile_hash: Self::fontfile_hash(opts.fontfile),
            name: canonical_name.to_owned(),
        };

        Ok((font_obj, info))
    }

    pub fn insert_font(
        &mut self,
        doc: &mut PdfDocument,
        opts: &InsertFontOptions<'_>,
    ) -> Result<(String, i32, FontInfo), Error> {
        assert_eq!(
            doc.as_raw(),
            unsafe { (*self.inner.as_ptr()).doc },
            "PdfPage ownership mismatch: the page is not attached to the provided PdfDocument"
        );

        let canonical_name = match opts.fontfile {
            Some(_) => opts.name.to_owned(),
            None => canonical_base14_name(opts.name)
                .ok_or_else(|| Error::InvalidArgument(format!("unsupported font: {}", opts.name)))?
                .to_owned(),
        };

        let mut resources = self.resources()?;
        let existing_font_dict = match resources.get_dict("Font")? {
            Some(font_dict) if font_dict.is_dict()? => Some(font_dict),
            _ => None,
        };

        if let Some(font_dict) = existing_font_dict.as_ref() {
            if let Some(existing) =
                Self::find_registered_page_font(doc, font_dict, &canonical_name, opts)?
            {
                return Ok(existing);
            }
        }

        let (font_obj, info) = if let Some((xref, info)) =
            Self::find_cached_document_font(doc, &canonical_name, opts)
        {
            (doc.new_indirect(xref, 0)?, info)
        } else {
            Self::build_font_object(doc, &canonical_name, opts)?
        };
        let xref = font_obj.as_indirect()?;

        let slot_name = Self::next_font_resource_slot(existing_font_dict.as_ref())?;
        let mut font_dict = if let Some(font_dict) = existing_font_dict {
            font_dict
        } else {
            let font_dict = doc.new_dict()?;
            resources.dict_put_ref("Font", &font_dict)?;
            font_dict
        };

        font_dict.dict_put_ref(slot_name.as_str(), &font_obj)?;
        doc.font_info_cache
            .borrow_mut()
            .entry(xref)
            .or_insert_with(|| info.clone());

        Ok((format!("/{slot_name}"), xref, info))
    }

    pub fn rotation(&self) -> Result<i32, Error> {
        if let Some(rotate) = self
            .object()
            .get_dict_inheritable(PdfObject::new_name("Rotate")?)?
        {
            return rotate.as_int();
        }
        Ok(0)
    }

    pub fn set_rotation(&mut self, rotate: i32) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_page_set_rotation(
                context(),
                self.as_mut_ptr(),
                rotate
            ))
        }
    }

    pub fn media_box(&self) -> Result<Rect, Error> {
        let rect = unsafe { mupdf_pdf_page_media_box(context(), self.as_ptr().cast_mut()) };
        Ok(rect.into())
    }

    pub fn crop_box(&self) -> Result<Rect, Error> {
        let bounds = self.bounds()?;
        let pos = unsafe { mupdf_pdf_page_crop_box_position(context(), self.as_ptr().cast_mut()) };
        let media_box = self.media_box()?;
        let x0 = pos.x;
        let y0 = media_box.height() - pos.y - bounds.height();
        let x1 = x0 + bounds.width();
        let y1 = y0 + bounds.height();
        Ok(Rect::new(x0, y0, x1, y1))
    }

    pub fn set_crop_box(&mut self, crop_box: Rect) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_page_set_crop_box(
                context(),
                self.as_mut_ptr(),
                crop_box.into()
            ))
        }
    }

    pub fn ctm(&self) -> Result<Matrix, Error> {
        unsafe {
            ffi_try!(mupdf_pdf_page_transform(
                context(),
                self.as_ptr().cast_mut()
            ))
        }
        .map(fz_matrix::into)
    }

    pub fn filter(&mut self, mut opt: PdfFilterOptions) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_filter_page_contents(
                context(),
                self.as_mut_ptr(),
                &raw mut opt.inner
            ))
        }
    }

    /// Returns an iterator over the link annotations on this page as [`PdfLink`] items.
    ///
    /// Each link's `bounds` are in MuPDF's Fitz coordinate space (origin at top-left,
    /// Y increasing downward). The `action` field is a structured [`PdfAction`] variant.
    ///
    /// Named destinations are resolved to concrete page numbers. Links with
    /// unresolvable named destinations are skipped.
    ///
    /// # Output mapping
    ///
    /// | Link pattern                                     | Resolved action                  |
    /// |--------------------------------------------------|----------------------------------|
    /// | Internal page link                               | `GoTo(Page { page, kind })`      |
    /// | Named destination (resolvable)                   | `GoTo(Page { page, kind })`      |
    /// | Named destination (unresolvable)                 | *(skipped)*                      |
    /// | `file://<path>.pdf#<params>`                     | `GoToR { file: Path(..), dest }` |
    /// | `file:<path>.pdf#<params>`                       | `GoToR { file: Path(..), dest }` |
    /// | `<scheme>://<host>/<..>.pdf#<params>` (external) | `GoToR { file: Url(..),  dest }` |
    /// | `<path>.pdf#<params>` (no scheme)                | `GoToR { file: Path(..), dest }` |
    /// | `file:<path>` (non-PDF)                          | `Launch(Path(..))`               |
    /// | `<local_path>` (non-external, non-PDF)           | `Launch(Path(..))`               |
    /// | `<scheme>://<..>` (non-PDF, external)            | `Uri(uri)`                       |
    pub fn resolved_links(&self) -> Result<PdfLinkIter, Error> {
        let links_head =
            unsafe { ffi_try!(mupdf_load_links(context(), self.inner.as_ptr().cast())) }?;

        let doc_ptr =
            NonNull::new(unsafe { (*self.inner.as_ptr()).doc }).ok_or(Error::UnexpectedNullPtr)?;

        let doc = unsafe { PdfDocument::from_raw(pdf_keep_document(context(), doc_ptr.as_ptr())) };

        Ok(PdfLinkIter {
            links_head,
            next: links_head,
            doc,
        })
    }

    /// Returns an iterator over the link annotation dictionaries on this page
    /// as [`PdfLinkAnnot`] items.
    ///
    /// Iterates the page's `/Annots` array and yields entries whose `Subtype` is `Link`.
    /// Non-link annotations are silently skipped.
    ///
    /// # Why not [`PdfPage::annotations`]?
    ///
    /// MuPDF's `pdf_sync_annots` ([`PdfPage::annotations`]) skips `Subtype=Link` entries, so
    /// [`PdfAnnotation`] of type [`PdfAnnotationType::Link`] can never be produced from a
    /// page via the annotation iterator. This method bypasses that by reading the `/Annots`
    /// array directly.
    pub fn link_annotations(&self) -> Result<PdfLinkAnnotIter, Error> {
        let annots = self.object().get_dict("Annots")?;
        let count = if let Some(a) = &annots { a.len()? } else { 0 };

        Ok(PdfLinkAnnotIter {
            annots,
            count,
            index: 0,
        })
    }

    /// Extracts all links from this page by reading the `/Annots` array directly.
    ///
    /// This is the symmetric counterpart to [`add_links`](Self::add_links): it reads the
    /// same annotation dictionaries that `add_links` writes, preserving:
    ///
    /// - The `/Dest` and `/A` destinations (as [`LinkAction::Dest`] and [`LinkAction::Action`])
    /// - Named destinations as [`PdfDestination::Named`] (not resolved to page numbers)
    /// - `Launch` actions exactly as specified in the PDF document
    /// - Does not clamp local coordinates to the page bounds
    ///
    /// Links with no recognizable action, as well as malformed or unparseable annotations,
    /// are silently skipped. If you need error reporting, iterate over [`PdfLinkAnnotIter`]
    /// manually and call [`PdfLinkAnnot::to_pdf_link`] on each item.
    pub fn links_from_annotations_lossy(&self) -> Result<Vec<PdfLink>, Error> {
        let doc_ptr =
            NonNull::new(unsafe { (*self.inner.as_ptr()).doc }).ok_or(Error::UnexpectedNullPtr)?;
        let doc = unsafe { PdfDocument::from_raw(pdf_keep_document(context(), doc_ptr.as_ptr())) };
        let page_no = doc.lookup_page_number(&self.object()).ok();
        let page_ctm = self.ctm()?;

        let mut result = Vec::new();
        for annot in self.link_annotations()?.flatten() {
            if let Ok(Some(link)) = annot.to_pdf_link(&doc, page_no, Some(&page_ctm)) {
                result.push(link);
            }
        }
        Ok(result)
    }

    /// Adds link annotations to this page using the page's own CTM for coordinate
    /// transformation.
    ///
    /// Convenience wrapper around [`add_links_with_inv_ctm`](Self::add_links_with_inv_ctm)
    /// that uses a cached resolver deriving inverse CTMs from
    /// `page_obj.page_ctm().invert()`. This is correct when all link coordinates
    /// (both annotation bounds and `GoTo` destinations) are in MuPDF's Fitz
    /// coordinate space.
    ///
    /// # In-memory link list
    ///
    /// Like [`add_links_with_inv_ctm`](Self::add_links_with_inv_ctm), this updates
    /// `/Annots` but does not refresh MuPDF's in-memory `fz_link` list. Use
    /// [`links_from_annotations_lossy`](Self::links_from_annotations_lossy) for immediate reads,
    /// or reload page before calling [`resolved_links`](Self::resolved_links).
    pub fn add_links(&mut self, doc: &mut PdfDocument, links: &[PdfLink]) -> Result<(), Error> {
        let mut cache = HashMap::new();
        let mut resolver = CachedResolver::new(&mut cache, |page_obj: &PdfObject| {
            Ok(page_obj.page_ctm()?.invert())
        });
        self.add_links_with_inv_ctm(
            doc,
            links,
            |page_obj| Ok(page_obj.page_ctm()?.invert()),
            &mut resolver,
        )
    }

    /// Adds link annotations to this page, using caller-provided inverse CTM functions
    /// for coordinate transformation.
    ///
    /// Each [`PdfLink`] is converted into a PDF link annotation dictionary and
    /// appended to the page's `/Annots` array.
    ///
    /// # Coordinate transforms
    ///
    /// [`PdfLink`] coordinates are in Fitz space. PDF annotations need PDF default
    /// user space. Two mechanisms provide the inverse CTM for each context:
    ///
    /// - `annot_inv_ctm(page_obj)` — for the annotation `/Rect` on *this* page.
    /// - `resolver` — a [`DestPageResolver`] that provides the destination page
    ///   object and its inverse CTM for `GoTo(Page { .. })` destinations.
    ///   `GoToR` coordinates are written as-is.
    ///
    /// # PdfAction -> annotation dictionary mapping
    ///
    /// | `PdfAction` variant         | `/S` (type) | `D` entry (`URI` for `Uri`) | `/F` entry    |
    /// |-----------------------------|-------------|-----------------------------|---------------|
    /// | `GoTo(Page { .. })`         | `GoTo`      | `[page_ref, /Kind, ...]`    | —             |
    /// | `GoTo(Named(..))`           | `GoTo`      | `(name)`                    | —             |
    /// | `Uri(..)`                   | `URI`       | `(uri)`                     | —             |
    /// | `GoToR { .. }` (explicit)   | `GoToR`     | `[page_int, /Kind, ...]`    | filespec dict |
    /// | `GoToR { .. }` (named)      | `GoToR`     | `(name)`                    | filespec dict |
    /// | `Launch(..)`                | `Launch`    | —                           | filespec dict |
    ///
    ///
    /// where:
    ///
    /// - `page_ref` is an indirect reference to the destination page object (local document)
    /// - `page_int` is the zero-based page number as an integer (remote document)
    /// - `/Kind` is the PDF destination type name (e.g. `/Fit`, `/XYZ`) followed by its parameters
    ///   (see [`crate::DestinationKind::encode_into`])
    /// - `filespec dict` is a file specification dictionary
    ///
    /// # In-memory link list
    ///
    /// This method modifies the page's `/Annots` dictionary but does not update
    /// MuPDF's in-memory `fz_link` list. As a result:
    ///
    /// - [`links_from_annotations_lossy`](Self::links_from_annotations_lossy) reflects the new
    ///   links immediately (reads the dict directly).
    /// - [`resolved_links`](Self::resolved_links) will **not** include the new links until
    ///   the page is reloaded.
    ///
    /// # Panics
    ///
    /// Panics if `self` does not belong to `doc` (ownership mismatch).
    pub fn add_links_with_inv_ctm(
        &mut self,
        doc: &mut PdfDocument,
        links: &[PdfLink],
        annot_inv_ctm: impl FnOnce(&PdfObject) -> Result<Option<Matrix>, Error>,
        resolver: &mut impl DestPageResolver,
    ) -> Result<(), Error> {
        if links.is_empty() {
            return Ok(());
        }

        assert_eq!(
            doc.as_raw(),
            unsafe { (*self.inner.as_ptr()).doc },
            "PdfPage ownership mismatch: the page is not attached to the provided PdfDocument"
        );

        let operation = DocOperation::begin(doc, "Add links")?;

        let mut page_obj = self.object();
        let annot_inv_ctm = annot_inv_ctm(&page_obj)?;

        let mut annots = match page_obj.get_dict("Annots")? {
            Some(annots) if annots.is_array()? => {
                if annots.is_indirect()? {
                    annots.copy_array()?
                } else {
                    annots
                }
            }
            _ => operation.doc.new_array()?,
        };

        for link in links {
            let annot =
                build_link_annotation(operation.doc, &page_obj, link, &annot_inv_ctm, resolver)?;
            let annot_indirect = operation.doc.add_object(&annot)?;
            annots.array_push(annot_indirect)?;
        }

        if annots.len()? > 0 {
            page_obj.dict_put("Annots", annots)?;
        }

        operation.commit()
    }
}

/// Iterator over link annotations on a PDF page, yielding [`PdfLink`] items.
///
/// Created by [`PdfPage::resolved_links`]. Links with unresolvable named destinations
/// or empty URIs are silently skipped.
///
/// See [`PdfPage::resolved_links`] for the full output mapping table.
#[derive(Debug)]
pub struct PdfLinkIter {
    links_head: *mut fz_link,
    next: *mut fz_link,
    doc: PdfDocument,
}

impl Drop for PdfLinkIter {
    fn drop(&mut self) {
        if !self.links_head.is_null() {
            unsafe {
                fz_drop_link(context(), self.links_head);
            }
        }
    }
}

impl Iterator for PdfLinkIter {
    type Item = Result<PdfLink, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.next.is_null() {
                return None;
            }

            let node = self.next;
            unsafe {
                self.next = (*node).next;
                let bounds = (*node).rect.into();
                let uri = CStr::from_ptr((*node).uri);

                let action = match LinkDestination::from_uri(&self.doc, uri) {
                    Ok(Some(dest)) => {
                        // In PDFs, `page_in_chapter` is equivalent to `page_number`. Using it directly
                        // allows the compiler to elide the `fz_page_number_from_location` FFI call.
                        LinkAction::Action(PdfAction::GoTo(PdfDestination::Page {
                            page: dest.loc.page_in_chapter,
                            kind: dest.kind,
                        }))
                    }
                    Ok(None) => match parse_external_link(uri.to_string_lossy().as_ref()) {
                        Some(action) => match action {
                            PdfAction::GoTo(PdfDestination::Named(_)) => {
                                // `LinkDestination::from_uri` already attempted to resolve named destinations.
                                // Reaching this point means the destination remains unresolved, so ignore and skip.
                                continue;
                            }
                            action => LinkAction::Action(action),
                        },
                        None => continue,
                    },
                    Err(e) => return Some(Err(e)),
                };

                return Some(Ok(PdfLink { bounds, action }));
            }
        }
    }
}

/// Iterator over link annotation dictionaries on a PDF page, yielding [`PdfLinkAnnot`] items.
///
/// Created by [`PdfPage::link_annotations`]. Non-link annotations (those without `Subtype=Link`)
/// are silently skipped. Each `Result::Err` stops iteration, callers should handle
/// errors appropriately.
///
/// See [`PdfPage::link_annotations`] for details on why this iterator exists alongside
/// [`PdfPage::annotations`].
#[derive(Debug)]
pub struct PdfLinkAnnotIter {
    annots: Option<PdfObject>,
    count: usize,
    index: usize,
}

impl Iterator for PdfLinkAnnotIter {
    type Item = Result<PdfLinkAnnot, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let annots = self.annots.as_ref()?;
        loop {
            if self.index >= self.count {
                return None;
            }
            let index = self.index;
            self.index += 1;
            let entry = match annots.get_array(index as i32) {
                Ok(Some(e)) => e,
                Ok(None) => continue,
                Err(e) => return Some(Err(e)),
            };
            let resolved = match entry.resolve() {
                Ok(Some(r)) => r,
                Ok(None) => continue,
                Err(e) => return Some(Err(e)),
            };
            let subtype = match resolved.get_dict("Subtype") {
                Ok(Some(s)) => s,
                Ok(None) => continue,
                Err(e) => return Some(Err(e)),
            };
            match subtype.as_name() {
                Ok(name) if name == b"Link" => return Some(Ok(PdfLinkAnnot::new(resolved))),
                Ok(_) => continue,
                Err(e) => return Some(Err(e)),
            }
        }
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

#[derive(Debug)]
pub struct AnnotationIter<'a> {
    next: Option<NonNull<pdf_annot>>,
    marker: PhantomData<&'a PdfPage>,
}

impl Iterator for AnnotationIter<'_> {
    type Item = PdfAnnotation;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.next?.as_ptr();
        unsafe {
            self.next = NonNull::new(pdf_next_annot(context(), node));
            // SAFETY: `node` is a borrowed pointer from the page's annotation
            // list. The PhantomData borrow guarantees the page outlives this
            // iterator. The yielded PdfAnnotation holds its own page refcount.
            Some(PdfAnnotation::from_raw_keep_ref(node))
        }
    }
}

impl TryFrom<Page> for PdfPage {
    type Error = Error;
    fn try_from(value: Page) -> Result<Self, Self::Error> {
        let pdf_page = unsafe { pdf_page_from_fz_page(context(), value.as_ptr().cast_mut()) };
        // We need to make sure to not run the destructor for `Page`, or else it'll be freed and
        // then this struct will be pointing to invalid memory when we try to use it.
        // ...god please give me linear types so that I can check this sort of transformation at
        // compile-time
        std::mem::forget(value);
        NonNull::new(pdf_page)
            .ok_or(Error::UnexpectedNullPtr)
            .map(|inner| unsafe { PdfPage::from_raw(inner) })
    }
}

#[cfg(test)]
mod test {
    use crate::document::test_document;
    use crate::pdf::{
        InsertFontOptions, PdfAnnotation, PdfAnnotationType, PdfDocument, PdfObject, PdfPage,
    };
    use crate::{Buffer, Matrix, Rect, SimpleFontEncoding, Size};

    fn contents_xrefs(page: &PdfPage) -> Vec<i32> {
        let contents = page.contents().unwrap().unwrap();
        assert!(contents.is_array().unwrap());
        (0..contents.len().unwrap())
            .map(|index| {
                contents
                    .get_array(index as i32)
                    .unwrap()
                    .unwrap()
                    .as_indirect()
                    .unwrap()
            })
            .collect()
    }

    fn load_dummy_page() -> (PdfDocument, PdfPage) {
        let doc = test_document!("../..", "files/dummy.pdf" as PdfDocument).unwrap();
        let page = PdfPage::try_from(doc.load_page(0).unwrap()).unwrap();
        (doc, page)
    }

    fn add_stream(doc: &mut PdfDocument, bytes: &[u8]) -> PdfObject {
        doc.add_stream(&Buffer::from_bytes(bytes).unwrap(), None, false)
            .unwrap()
    }

    fn default_font_options(name: &str) -> InsertFontOptions<'_> {
        InsertFontOptions::new(name)
    }

    fn page_font_dict(page: &PdfPage) -> PdfObject {
        page.resources().unwrap().get_dict("Font").unwrap().unwrap()
    }

    fn page_font_xref(page: &PdfPage, slot: &str) -> i32 {
        page_font_dict(page)
            .get_dict(slot)
            .unwrap()
            .unwrap()
            .as_indirect()
            .unwrap()
    }

    fn new_page_with_contents_array(
        doc: &mut PdfDocument,
        payloads: &[&[u8]],
    ) -> (PdfPage, Vec<i32>) {
        let page = doc.new_page(Size::A4).unwrap();
        let mut xrefs = Vec::with_capacity(payloads.len());
        let mut contents_array = doc.new_array_with_capacity(payloads.len() as i32).unwrap();

        for payload in payloads {
            let stream = add_stream(doc, payload);
            xrefs.push(stream.as_indirect().unwrap());
            contents_array.array_push(stream).unwrap();
        }

        page.object().dict_put("Contents", contents_array).unwrap();
        (page, xrefs)
    }

    #[test]
    fn test_page_properties() {
        let doc = test_document!("../..", "files/dummy.pdf" as PdfDocument).unwrap();
        let mut page0 = PdfPage::try_from(doc.load_page(0).unwrap()).unwrap();

        // CTM
        let ctm = page0.ctm().unwrap();
        assert_eq!(ctm, Matrix::new(1.0, 0.0, 0.0, -1.0, 0.0, 842.0));

        // Rotation
        let rotation = page0.rotation().unwrap();
        assert_eq!(rotation, 0);

        page0.set_rotation(90).unwrap();
        let rotation = page0.rotation().unwrap();
        assert_eq!(rotation, 90);
        page0.set_rotation(0).unwrap(); // reset rotation

        // MediaBox
        let media_box = page0.media_box().unwrap();
        assert_eq!(media_box, Rect::new(0.0, 0.0, 595.0, 842.0));

        // CropBox
        let crop_box = page0.crop_box().unwrap();
        assert_eq!(crop_box, Rect::new(0.0, 0.0, 595.0, 842.0));
        page0
            .set_crop_box(Rect::new(100.0, 100.0, 400.0, 400.0))
            .unwrap();
        let crop_box = page0.crop_box().unwrap();
        assert_eq!(crop_box, Rect::new(100.0, 100.0, 400.0, 400.0));
    }

    #[test]
    fn test_page_annotations() {
        let doc = test_document!("../..", "files/dummy.pdf" as PdfDocument).unwrap();
        let page0 = PdfPage::try_from(doc.load_page(0).unwrap()).unwrap();
        let annots: Vec<PdfAnnotation> = page0.annotations().collect();
        assert_eq!(annots.len(), 0);
    }

    #[test]
    fn test_page_annotations_keep_refs() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut created = page.create_annotation(PdfAnnotationType::Text).unwrap();
        let expected = Rect::new(10.0, 20.0, 110.0, 120.0);
        created.set_rect(expected).unwrap();
        drop(created);

        let annots: Vec<PdfAnnotation> = page.annotations().collect();
        assert_eq!(annots.len(), 1);
        assert_eq!(annots[0].r#type().unwrap(), PdfAnnotationType::Text);
        assert_eq!(annots[0].rect().unwrap(), expected);
    }

    #[test]
    fn test_page_contents_none_for_blank_page() {
        let mut doc = PdfDocument::new();
        let page = doc.new_page(Size::A4).unwrap();

        assert!(page.contents().unwrap().is_none());
    }

    #[test]
    fn test_page_contents_single_stream_fixture() {
        let doc = test_document!("../..", "files/dummy.pdf" as PdfDocument).unwrap();
        let page = PdfPage::try_from(doc.load_page(0).unwrap()).unwrap();
        let contents = page.contents().unwrap().unwrap();

        assert!(contents.is_stream().unwrap());
        assert!(contents.as_indirect().unwrap() > 0);
    }

    #[test]
    fn test_page_contents_multistream_array() {
        let mut doc = PdfDocument::new();
        let page = doc.new_page(Size::A4).unwrap();
        let first = doc
            .add_stream(&Buffer::from_bytes(b"q\n").unwrap(), None, false)
            .unwrap();
        let second = doc
            .add_stream(&Buffer::from_bytes(b"Q\n").unwrap(), None, false)
            .unwrap();
        let mut contents_array = doc.new_array_with_capacity(2).unwrap();
        contents_array.array_push(first).unwrap();
        contents_array.array_push(second).unwrap();
        page.object().dict_put("Contents", contents_array).unwrap();

        let contents = page.contents().unwrap().unwrap();
        assert!(contents.is_array().unwrap());
        assert_eq!(contents.len().unwrap(), 2);
    }

    #[test]
    fn test_page_resources_existing_dict_stable() {
        let doc = test_document!("../..", "files/dummy.pdf" as PdfDocument).unwrap();
        let page = PdfPage::try_from(doc.load_page(0).unwrap()).unwrap();

        let first = page.resources().unwrap();
        let second = page.resources().unwrap();

        assert!(first.is_dict().unwrap());
        assert_eq!(first.as_indirect().unwrap(), second.as_indirect().unwrap());
    }

    #[test]
    fn test_page_resources_auto_creates_missing_dict() {
        let mut doc = PdfDocument::new();
        let page = doc.new_page(Size::A4).unwrap();
        page.object().dict_delete("Resources").unwrap();
        assert!(page.object().get_dict("Resources").unwrap().is_none());

        let object_count_before = doc.count_objects().unwrap();
        let resources = page.resources().unwrap();

        assert!(resources.is_dict().unwrap());
        assert!(resources.as_indirect().unwrap() > 0);
        assert_eq!(resources.dict_len().unwrap(), 0);
        assert!(page.object().get_dict("Resources").unwrap().is_some());
        assert_eq!(doc.count_objects().unwrap(), object_count_before + 1);
    }

    #[test]
    fn test_page_resources_idempotent_after_auto_create() {
        let mut doc = PdfDocument::new();
        let page = doc.new_page(Size::A4).unwrap();
        page.object().dict_delete("Resources").unwrap();

        let first = page.resources().unwrap();
        let object_count_after_first = doc.count_objects().unwrap();
        let second = page.resources().unwrap();

        assert_eq!(first.as_indirect().unwrap(), second.as_indirect().unwrap());
        assert_eq!(doc.count_objects().unwrap(), object_count_after_first);
    }

    #[test]
    fn test_page_insert_contents_empty_page_creates_array() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();

        let xref = page.insert_contents(&mut doc, b"q Q\n", true).unwrap();

        assert!(xref > 0);
        assert_eq!(contents_xrefs(&page), vec![xref]);
    }

    #[test]
    fn test_page_insert_contents_promotes_single_stream_overlay() {
        let (mut doc, mut page) = load_dummy_page();
        let original_xref = page.contents().unwrap().unwrap().as_indirect().unwrap();

        let new_xref = page
            .insert_contents(&mut doc, b"q 1 0 0 rg 10 10 20 20 re f Q\n", true)
            .unwrap();

        assert_eq!(contents_xrefs(&page), vec![original_xref, new_xref]);
    }

    #[test]
    fn test_page_insert_contents_overlay_and_underlay_ordering() {
        let (mut doc, mut page) = load_dummy_page();
        let original_xref = page.contents().unwrap().unwrap().as_indirect().unwrap();
        let overlay_xref = page.insert_contents(&mut doc, b"overlay\n", true).unwrap();
        assert_eq!(contents_xrefs(&page), vec![original_xref, overlay_xref]);

        let (mut doc, mut page) = load_dummy_page();
        let original_xref = page.contents().unwrap().unwrap().as_indirect().unwrap();
        let underlay_xref = page
            .insert_contents(&mut doc, b"underlay\n", false)
            .unwrap();
        assert_eq!(contents_xrefs(&page), vec![underlay_xref, original_xref]);
    }

    #[test]
    fn test_page_insert_contents_preserves_prior_array_contents() {
        let mut doc = PdfDocument::new();
        let (mut page, original_xrefs) = new_page_with_contents_array(&mut doc, &[b"a\n", b"b\n"]);

        let c = page.insert_contents(&mut doc, b"c\n", true).unwrap();
        let d = page.insert_contents(&mut doc, b"d\n", true).unwrap();

        assert_eq!(
            contents_xrefs(&page),
            vec![original_xrefs[0], original_xrefs[1], c, d]
        );

        let mut doc = PdfDocument::new();
        let (mut page, original_xrefs) = new_page_with_contents_array(&mut doc, &[b"a\n", b"b\n"]);

        let c = page.insert_contents(&mut doc, b"c\n", false).unwrap();
        let d = page.insert_contents(&mut doc, b"d\n", true).unwrap();

        assert_eq!(
            contents_xrefs(&page),
            vec![c, original_xrefs[0], original_xrefs[1], d]
        );
    }

    #[test]
    fn test_page_insert_contents_returned_xref_bytes_round_trip() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let payload = b"q 1 0 0 rg 100 100 50 50 re f Q\n";

        let xref = page.insert_contents(&mut doc, payload, true).unwrap();
        let mut stream = doc.new_indirect(xref, 0).unwrap();

        assert_eq!(stream.as_indirect().unwrap(), xref);
        assert_eq!(stream.read_stream().unwrap(), payload);

        let rewritten = Buffer::from_bytes(b"rewritten\n").unwrap();
        stream.write_stream_buffer(&rewritten).unwrap();
        assert_eq!(stream.read_stream().unwrap(), b"rewritten\n");
    }

    #[test]
    fn test_page_insert_contents_distinct_streams_with_same_bytes() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let payload = b"q 0 1 0 rg 20 20 10 10 re f Q\n";

        let first = page.insert_contents(&mut doc, payload, true).unwrap();
        let second = page.insert_contents(&mut doc, payload, true).unwrap();

        assert_ne!(first, second);
        assert_eq!(contents_xrefs(&page), vec![first, second]);
        assert_eq!(
            doc.new_indirect(first, 0).unwrap().read_stream().unwrap(),
            payload
        );
        assert_eq!(
            doc.new_indirect(second, 0).unwrap().read_stream().unwrap(),
            payload
        );
    }

    fn contents_stream_bytes(page: &PdfPage) -> Vec<Vec<u8>> {
        let contents = page.contents().unwrap().unwrap();
        assert!(contents.is_array().unwrap());
        (0..contents.len().unwrap())
            .map(|index| {
                contents
                    .get_array(index as i32)
                    .unwrap()
                    .unwrap()
                    .read_stream()
                    .unwrap()
            })
            .collect()
    }

    #[test]
    fn test_page_wrap_contents_brackets_existing_stream() {
        let (mut doc, mut page) = load_dummy_page();
        let original_xref = page.contents().unwrap().unwrap().as_indirect().unwrap();

        page.wrap_contents(&mut doc).unwrap();

        let xrefs = contents_xrefs(&page);
        assert_eq!(xrefs.len(), 3);
        assert_eq!(xrefs[1], original_xref);

        let stream_bytes = contents_stream_bytes(&page);
        assert_eq!(stream_bytes[0], b"q\n");
        assert_eq!(stream_bytes[2], b"Q\n");
    }

    #[test]
    fn test_page_wrap_contents_nests_when_called_twice() {
        let (mut doc, mut page) = load_dummy_page();
        let original = page.contents().unwrap().unwrap();
        let original_xref = original.as_indirect().unwrap();
        let original_bytes = original.read_stream().unwrap();

        page.wrap_contents(&mut doc).unwrap();
        page.wrap_contents(&mut doc).unwrap();

        let xrefs = contents_xrefs(&page);
        assert_eq!(xrefs.len(), 5);
        assert_eq!(xrefs[2], original_xref);

        let stream_bytes = contents_stream_bytes(&page);
        assert_eq!(stream_bytes[0], b"q\n");
        assert_eq!(stream_bytes[1], b"q\n");
        assert_eq!(stream_bytes[2], original_bytes);
        assert_eq!(stream_bytes[3], b"Q\n");
        assert_eq!(stream_bytes[4], b"Q\n");
    }

    #[test]
    fn test_page_insert_font_helv_registers_f0_resource() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();

        let (slot, xref, info) = page
            .insert_font(&mut doc, &default_font_options("helv"))
            .unwrap();

        assert_eq!(slot, "/F0");
        assert!(xref > 0);
        assert_eq!(info.name, "Helvetica");
        assert!(info.simple);
        assert_eq!(page_font_xref(&page, "F0"), xref);
        assert_eq!(doc.font_info_cache.borrow().get(&xref), Some(&info));
    }

    #[test]
    fn test_page_insert_font_is_idempotent_for_same_name() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();

        let first = page
            .insert_font(&mut doc, &default_font_options("helv"))
            .unwrap();
        let second = page
            .insert_font(&mut doc, &default_font_options("helv"))
            .unwrap();

        assert_eq!(first, second);
        assert_eq!(page_font_dict(&page).dict_len().unwrap(), 1);
        assert_eq!(doc.font_info_cache.borrow().len(), 1);
    }

    #[test]
    fn test_page_insert_font_reuses_preexisting_font_after_reopen() {
        let mut source_doc = PdfDocument::new();
        let mut source_page = source_doc.new_page(Size::A4).unwrap();
        let inserted = source_page
            .insert_font(&mut source_doc, &default_font_options("helv"))
            .unwrap();
        assert_eq!(inserted.0, "/F0");
        assert_eq!(page_font_dict(&source_page).dict_len().unwrap(), 1);

        let mut bytes = Vec::new();
        source_doc.write_to(&mut bytes).unwrap();
        drop(source_page);
        drop(source_doc);

        let mut doc = PdfDocument::from_bytes(&bytes).unwrap();
        assert!(doc.font_info_cache.borrow().is_empty());
        let mut page = PdfPage::try_from(doc.load_page(0).unwrap()).unwrap();

        let first = page
            .insert_font(&mut doc, &default_font_options("helv"))
            .unwrap();
        let second = page
            .insert_font(&mut doc, &default_font_options("helv"))
            .unwrap();

        assert_eq!(first, second);
        assert_eq!(first.0, "/F0");
        assert_eq!(page_font_dict(&page).dict_len().unwrap(), 1);
        assert_eq!(page_font_xref(&page, "F0"), first.1);
        assert_eq!(doc.font_info_cache.borrow().get(&first.1), Some(&first.2));
    }

    #[test]
    fn test_page_insert_font_distinguishes_simple_encoding() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let latin = InsertFontOptions {
            encoding: SimpleFontEncoding::Latin,
            ..default_font_options("helv")
        };
        let greek = InsertFontOptions {
            encoding: SimpleFontEncoding::Greek,
            ..default_font_options("helv")
        };

        let first = page.insert_font(&mut doc, &latin).unwrap();
        let second = page.insert_font(&mut doc, &greek).unwrap();

        assert_ne!(first.0, second.0);
        assert_ne!(first.1, second.1);
        assert_eq!(first.2.encoding, SimpleFontEncoding::Latin);
        assert_eq!(second.2.encoding, SimpleFontEncoding::Greek);
        assert_eq!(page_font_dict(&page).dict_len().unwrap(), 2);
        assert_eq!(page_font_xref(&page, "F0"), first.1);
        assert_eq!(page_font_xref(&page, "F1"), second.1);
        assert_eq!(doc.font_info_cache.borrow().len(), 2);
    }

    #[test]
    fn test_page_insert_font_reuses_preexisting_greek_font_after_reopen() {
        let greek = InsertFontOptions {
            encoding: SimpleFontEncoding::Greek,
            ..default_font_options("helv")
        };
        let mut source_doc = PdfDocument::new();
        let mut source_page = source_doc.new_page(Size::A4).unwrap();
        source_page.insert_font(&mut source_doc, &greek).unwrap();

        let mut bytes = Vec::new();
        source_doc.write_to(&mut bytes).unwrap();
        drop(source_page);
        drop(source_doc);

        let mut doc = PdfDocument::from_bytes(&bytes).unwrap();
        assert!(doc.font_info_cache.borrow().is_empty());
        let mut page = PdfPage::try_from(doc.load_page(0).unwrap()).unwrap();

        let first = page.insert_font(&mut doc, &greek).unwrap();
        let second = page.insert_font(&mut doc, &greek).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.0, "/F0");
        assert_eq!(first.2.encoding, SimpleFontEncoding::Greek);
        assert_eq!(page_font_dict(&page).dict_len().unwrap(), 1);
    }

    #[test]
    fn test_page_insert_font_allocates_next_slot_for_new_font() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();

        let (helv_slot, helv_xref, _) = page
            .insert_font(&mut doc, &default_font_options("helv"))
            .unwrap();
        let (cour_slot, cour_xref, _) = page
            .insert_font(&mut doc, &default_font_options("cour"))
            .unwrap();

        assert_eq!(helv_slot, "/F0");
        assert_eq!(cour_slot, "/F1");
        assert_ne!(helv_xref, cour_xref);
        assert_eq!(page_font_xref(&page, "F0"), helv_xref);
        assert_eq!(page_font_xref(&page, "F1"), cour_xref);
    }

    #[test]
    fn test_page_insert_font_base14_alias_table() {
        let cases = [
            ("helv", "Helvetica"),
            ("heit", "Helvetica-Oblique"),
            ("hebo", "Helvetica-Bold"),
            ("heboit", "Helvetica-BoldOblique"),
            ("cour", "Courier"),
            ("coit", "Courier-Oblique"),
            ("cobo", "Courier-Bold"),
            ("coboit", "Courier-BoldOblique"),
            ("tiro", "Times-Roman"),
            ("tibo", "Times-Bold"),
            ("tiit", "Times-Italic"),
            ("tibi", "Times-BoldItalic"),
            ("symb", "Symbol"),
            ("zadb", "ZapfDingbats"),
        ];

        let mut doc = PdfDocument::new();
        for (alias, canonical) in cases {
            let mut page = doc.new_page(Size::A4).unwrap();
            let (_, _, info) = page
                .insert_font(&mut doc, &default_font_options(alias))
                .unwrap();
            assert_eq!(info.name, canonical, "alias {alias}");
            assert!(info.simple, "alias {alias}");
        }
    }

    #[test]
    fn test_page_insert_font_accepts_canonical_base14_name() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();

        let (slot, xref, info) = page
            .insert_font(&mut doc, &default_font_options("Helvetica"))
            .unwrap();

        assert_eq!(slot, "/F0");
        assert!(xref > 0);
        assert_eq!(info.name, "Helvetica");
        assert!(info.simple);
    }

    #[test]
    fn test_page_insert_font_skips_preexisting_f_slots() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut resources = page.resources().unwrap();
        let mut font_dict = doc.new_dict().unwrap();
        font_dict.dict_put("F0", doc.new_dict().unwrap()).unwrap();
        resources.dict_put("Font", font_dict).unwrap();

        let (slot, xref, _) = page
            .insert_font(&mut doc, &default_font_options("helv"))
            .unwrap();

        assert_eq!(slot, "/F1");
        assert_eq!(page_font_xref(&page, "F1"), xref);
        assert!(page_font_dict(&page).get_dict("F0").unwrap().is_some());
    }

    #[test]
    fn test_page_insert_font_shares_xref_across_pages() {
        let mut doc = PdfDocument::new();
        let mut first_page = doc.new_page(Size::A4).unwrap();
        let mut second_page = doc.new_page(Size::A4).unwrap();

        let first = first_page
            .insert_font(&mut doc, &default_font_options("helv"))
            .unwrap();
        let second = second_page
            .insert_font(&mut doc, &default_font_options("helv"))
            .unwrap();

        assert_eq!(first.1, second.1);
        assert_eq!(page_font_xref(&first_page, "F0"), first.1);
        assert_eq!(page_font_xref(&second_page, "F0"), first.1);
        assert_eq!(doc.font_info_cache.borrow().len(), 1);
    }

    #[test]
    fn test_page_insert_font_cache_is_not_duplicated() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();

        let (_, xref, _) = page
            .insert_font(&mut doc, &default_font_options("helv"))
            .unwrap();
        assert!(doc.font_info_cache.borrow().contains_key(&xref));
        let cache_len = doc.font_info_cache.borrow().len();

        page.insert_font(&mut doc, &default_font_options("helv"))
            .unwrap();

        assert_eq!(doc.font_info_cache.borrow().len(), cache_len);
    }

    #[test]
    fn test_page_insert_font_unknown_name_errors_without_mutation() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        assert!(page
            .resources()
            .unwrap()
            .get_dict("Font")
            .unwrap()
            .is_none());
        let cache_len = doc.font_info_cache.borrow().len();

        let result = page.insert_font(&mut doc, &default_font_options("not-a-font"));

        assert!(result.is_err());
        assert!(page
            .resources()
            .unwrap()
            .get_dict("Font")
            .unwrap()
            .is_none());
        assert_eq!(doc.font_info_cache.borrow().len(), cache_len);
    }
}

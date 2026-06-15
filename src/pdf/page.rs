use std::collections::{hash_map::DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::str;
use std::{
    ffi::{CStr, CString},
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use mupdf_sys::*;

use crate::drawing::Drawing;
use crate::link::LinkDestination;
use crate::pdf::annotation::bounding_rect_for_quads;
use crate::pdf::links::{
    build_link_annotation, parse_external_link, CachedResolver, DestPageResolver,
};
use crate::pdf::{
    AnnotationArea, AnnotationQuadPoints, DocOperation, LinkAction, OptionalContentRef, PdfAction,
    PdfAnnotation, PdfAnnotationType, PdfDestination, PdfDocument, PdfFilterOptions, PdfLink,
    PdfLinkAnnot, PdfObject, PdfRedactOptions, PdfWidget, PdfWidgetIter,
};
use crate::{
    context, unsafe_impl_ffi_wrapper, Buffer, CjkFontOrdering, Error, FFIWrapper, Font, Image,
    Matrix, Page, Pixmap, Point, Rect, SimpleFontEncoding, WriteMode,
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

#[derive(Clone, Copy, Debug)]
pub enum PageImageSource<'a> {
    Image(&'a Image),
    Pixmap(&'a Pixmap),
    Bytes {
        data: &'a [u8],
        format_hint: Option<&'a str>,
    },
    ExistingXref(i32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InsertImageOptions {
    pub overlay: bool,
    pub opacity: Option<f32>,
    pub optional_content: Option<OptionalContentRef>,
}

impl Default for InsertImageOptions {
    fn default() -> Self {
        Self {
            overlay: true,
            opacity: None,
            optional_content: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImagePlacement {
    pub name: String,
    pub xref: i32,
    pub rect: Rect,
    pub contents_xref: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PageImageInfo {
    pub name: String,
    pub xref: i32,
    pub width: u32,
    pub height: u32,
    pub bits_per_component: Option<i32>,
    pub color_space: Option<String>,
    pub filter: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExtractedImage {
    pub xref: i32,
    pub width: u32,
    pub height: u32,
    pub bits_per_component: Option<i32>,
    pub color_space: Option<String>,
    pub filter: Option<String>,
    pub encoded: Vec<u8>,
}

fn canonical_base14_name(name: &str) -> Option<&'static str> {
    match name.trim_start_matches('/').to_ascii_lowercase().as_str() {
        "helv" | "helvetica" => Some("Helvetica"),
        "heit" | "helvetica-oblique" => Some("Helvetica-Oblique"),
        "hebo" | "helvetica-bold" => Some("Helvetica-Bold"),
        "heboit" | "helvetica-boldoblique" => Some("Helvetica-BoldOblique"),
        "cour" | "courier" => Some("Courier"),
        "coit" | "courier-oblique" => Some("Courier-Oblique"),
        "cobo" | "courier-bold" => Some("Courier-Bold"),
        "coboit" | "courier-boldoblique" => Some("Courier-BoldOblique"),
        "tiro" | "times-roman" => Some("Times-Roman"),
        "tibo" | "times-bold" => Some("Times-Bold"),
        "tiit" | "times-italic" => Some("Times-Italic"),
        "tibi" | "times-bolditalic" => Some("Times-BoldItalic"),
        "symb" | "symbol" => Some("Symbol"),
        "zadb" | "zapfdingbats" => Some("ZapfDingbats"),
        _ => None,
    }
}

fn cjk_ordering_from_font_name(name: &str) -> Option<CjkFontOrdering> {
    match name.trim_start_matches('/').to_ascii_lowercase().as_str() {
        "china-t" => Some(CjkFontOrdering::AdobeCns),
        "china-s" => Some(CjkFontOrdering::AdobeGb),
        "japan" => Some(CjkFontOrdering::AdobeJapan),
        "korea" => Some(CjkFontOrdering::AdobeKorea),
        _ => None,
    }
}

fn cjk_font_name(ordering: CjkFontOrdering) -> &'static str {
    match ordering {
        CjkFontOrdering::AdobeCns => "china-t",
        CjkFontOrdering::AdobeGb => "china-s",
        CjkFontOrdering::AdobeJapan => "japan",
        CjkFontOrdering::AdobeKorea => "korea",
    }
}

fn format_pdf_number(value: f32) -> String {
    let mut value = format!("{value:.6}");
    while value.contains('.') && value.ends_with('0') {
        value.pop();
    }
    if value.ends_with('.') {
        value.pop();
    }
    if value == "-0" {
        value = "0".to_owned();
    }
    value
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

    fn create_configured_annotation<F>(
        &mut self,
        subtype: PdfAnnotationType,
        configure: F,
    ) -> Result<PdfAnnotation, Error>
    where
        F: FnOnce(&mut Self, &mut PdfAnnotation) -> Result<(), Error>,
    {
        let mut annot = self.create_annotation(subtype)?;
        if let Err(err) = configure(self, &mut annot) {
            let _ = unsafe {
                ffi_try!(mupdf_pdf_delete_annot(
                    context(),
                    self.as_mut_ptr(),
                    annot.inner.as_ptr()
                ))
            };
            return Err(err);
        }
        Ok(annot)
    }

    fn set_annotation_rect_entry(
        &self,
        annot: &mut PdfAnnotation,
        rect: Rect,
    ) -> Result<(), Error> {
        validate_non_empty_rect(rect, "annotation requires a non-empty valid rectangle")?;

        let inv_ctm = self.ctm()?.invert().ok_or(Error::NonInvertibleMatrix)?;
        let rect = rect.transform(&inv_ctm);
        let doc = self.document_handle()?;
        let mut rect_array = doc.new_array_with_capacity(4)?;
        rect.encode_into(&mut rect_array)?;
        annot.object().dict_put("Rect", rect_array)
    }

    fn create_annotation_with_rect(
        &mut self,
        subtype: PdfAnnotationType,
        rect: Rect,
    ) -> Result<PdfAnnotation, Error> {
        self.create_configured_annotation(subtype, |_, annot| annot.set_rect(rect))
    }

    pub fn add_text_annotation(
        &mut self,
        rect: Rect,
        contents: &str,
    ) -> Result<PdfAnnotation, Error> {
        self.create_configured_annotation(PdfAnnotationType::Text, |_, annot| {
            annot.set_rect(rect)?;
            annot.set_contents(contents)
        })
    }

    pub fn add_free_text_annotation(
        &mut self,
        rect: Rect,
        contents: &str,
    ) -> Result<PdfAnnotation, Error> {
        self.create_configured_annotation(PdfAnnotationType::FreeText, |_, annot| {
            annot.set_rect(rect)?;
            annot.set_contents(contents)
        })
    }

    pub fn add_caret_annotation(&mut self, rect: Rect) -> Result<PdfAnnotation, Error> {
        self.create_annotation_with_rect(PdfAnnotationType::Caret, rect)
    }

    pub fn add_line_annotation(
        &mut self,
        start: Point,
        end: Point,
    ) -> Result<PdfAnnotation, Error> {
        let rect = padded_bounding_rect_for_points(&[start, end])?;
        self.create_configured_annotation(PdfAnnotationType::Line, |page, annot| {
            annot.set_line(start, end)?;
            page.set_annotation_rect_entry(annot, rect)
        })
    }

    pub fn add_rect_annotation(&mut self, rect: Rect) -> Result<PdfAnnotation, Error> {
        self.create_annotation_with_rect(PdfAnnotationType::Square, rect)
    }

    pub fn add_square_annotation(&mut self, rect: Rect) -> Result<PdfAnnotation, Error> {
        self.add_rect_annotation(rect)
    }

    pub fn add_circle_annotation(&mut self, rect: Rect) -> Result<PdfAnnotation, Error> {
        self.create_annotation_with_rect(PdfAnnotationType::Circle, rect)
    }

    pub fn add_polygon_annotation(
        &mut self,
        points: impl IntoIterator<Item = Point>,
    ) -> Result<PdfAnnotation, Error> {
        self.add_vertex_annotation(PdfAnnotationType::Polygon, points, 3)
    }

    pub fn add_polyline_annotation(
        &mut self,
        points: impl IntoIterator<Item = Point>,
    ) -> Result<PdfAnnotation, Error> {
        self.add_vertex_annotation(PdfAnnotationType::PolyLine, points, 2)
    }

    fn add_vertex_annotation(
        &mut self,
        subtype: PdfAnnotationType,
        points: impl IntoIterator<Item = Point>,
        min_points: usize,
    ) -> Result<PdfAnnotation, Error> {
        let points: Vec<Point> = points.into_iter().collect();
        if points.len() < min_points {
            return Err(Error::InvalidArgument(format!(
                "{subtype:?} annotation requires at least {min_points} points"
            )));
        }
        let rect = padded_bounding_rect_for_points(&points)?;
        self.create_configured_annotation(subtype, |page, annot| {
            annot.set_vertices(points.iter().copied())?;
            page.set_annotation_rect_entry(annot, rect)
        })
    }

    pub fn add_ink_annotation<I, S>(&mut self, strokes: I) -> Result<PdfAnnotation, Error>
    where
        I: IntoIterator<Item = S>,
        S: IntoIterator<Item = Point>,
    {
        let strokes: Vec<Vec<Point>> = strokes
            .into_iter()
            .map(|stroke| stroke.into_iter().collect())
            .collect();
        if strokes.is_empty() {
            return Err(Error::InvalidArgument(
                "ink annotation requires at least one stroke".to_owned(),
            ));
        }
        if strokes.iter().any(Vec::is_empty) {
            return Err(Error::InvalidArgument(
                "ink annotation strokes must not be empty".to_owned(),
            ));
        }
        let points: Vec<Point> = strokes.iter().flatten().copied().collect();
        let rect = padded_bounding_rect_for_points(&points)?;
        self.create_configured_annotation(PdfAnnotationType::Ink, |page, annot| {
            annot.set_ink_list(strokes.iter().map(|stroke| stroke.iter().copied()))?;
            page.set_annotation_rect_entry(annot, rect)
        })
    }

    pub fn add_highlight_annotation(
        &mut self,
        quads: impl Into<AnnotationQuadPoints>,
    ) -> Result<PdfAnnotation, Error> {
        self.add_text_markup_annotation(PdfAnnotationType::Highlight, quads)
    }

    pub fn add_underline_annotation(
        &mut self,
        quads: impl Into<AnnotationQuadPoints>,
    ) -> Result<PdfAnnotation, Error> {
        self.add_text_markup_annotation(PdfAnnotationType::Underline, quads)
    }

    pub fn add_squiggly_annotation(
        &mut self,
        quads: impl Into<AnnotationQuadPoints>,
    ) -> Result<PdfAnnotation, Error> {
        self.add_text_markup_annotation(PdfAnnotationType::Squiggly, quads)
    }

    pub fn add_strikeout_annotation(
        &mut self,
        quads: impl Into<AnnotationQuadPoints>,
    ) -> Result<PdfAnnotation, Error> {
        self.add_text_markup_annotation(PdfAnnotationType::StrikeOut, quads)
    }

    fn add_text_markup_annotation(
        &mut self,
        subtype: PdfAnnotationType,
        quads: impl Into<AnnotationQuadPoints>,
    ) -> Result<PdfAnnotation, Error> {
        let quads = quads.into();
        let rect = validate_quad_area(
            quads.as_slice(),
            &format!("{subtype:?} annotation requires non-empty valid quad points"),
        )?;
        self.create_configured_annotation(subtype, |page, annot| {
            annot.set_quad_points(quads)?;
            page.set_annotation_rect_entry(annot, rect)
        })
    }

    pub fn add_stamp_annotation(
        &mut self,
        rect: Rect,
        icon_name: &str,
    ) -> Result<PdfAnnotation, Error> {
        self.create_configured_annotation(PdfAnnotationType::Stamp, |_, annot| {
            annot.set_rect(rect)?;
            annot.set_icon_name(icon_name)
        })
    }

    pub fn add_redact_annotation(
        &mut self,
        area: impl Into<AnnotationArea>,
    ) -> Result<PdfAnnotation, Error> {
        let area = validate_redaction_area(area.into())?;
        self.create_configured_annotation(PdfAnnotationType::Redact, |_, annot| {
            set_annotation_area(annot, area)
        })
    }

    pub fn delete_annotation(&mut self, annot: PdfAnnotation) -> Result<(), Error> {
        let annot_page = annot.page_ptr()?;
        if annot_page != self.as_mut_ptr() {
            return Err(Error::InvalidArgument(
                "annotation does not belong to this page".to_owned(),
            ));
        }
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

    pub fn widgets(&self) -> PdfWidgetIter<'_> {
        let next = unsafe { pdf_first_widget(context(), self.as_ptr().cast_mut()) };
        PdfWidgetIter {
            next: NonNull::new(next),
            marker: PhantomData,
        }
    }

    pub fn load_widget(&self, xref: i32) -> Result<Option<PdfWidget>, Error> {
        for widget in self.widgets() {
            if widget.xref()? == xref {
                return Ok(Some(widget));
            }
        }
        Ok(None)
    }

    pub fn delete_widget(&mut self, widget: PdfWidget) -> Result<(), Error> {
        self.delete_annotation(widget.into_annotation())
    }

    pub fn add_signature_widget(&mut self, name: &str) -> Result<PdfWidget, Error> {
        let name = CString::new(name)?;
        unsafe {
            ffi_try!(mupdf_pdf_create_signature_widget(
                context(),
                self.as_mut_ptr(),
                name.as_ptr()
            ))
        }
        .map(|widget| unsafe { PdfWidget::from_raw(widget) })
    }

    pub fn update(&mut self) -> Result<bool, Error> {
        unsafe { ffi_try!(mupdf_pdf_update_page(context(), self.as_mut_ptr())) }
    }

    pub fn redact(&mut self) -> Result<bool, Error> {
        unsafe { ffi_try!(mupdf_pdf_redact_page(context(), self.as_mut_ptr())) }
    }

    pub fn redact_with_options(&mut self, options: PdfRedactOptions) -> Result<bool, Error> {
        let mut raw = options.into_raw();
        unsafe {
            ffi_try!(mupdf_pdf_redact_page_with_options(
                context(),
                self.as_mut_ptr(),
                &raw mut raw
            ))
        }
    }

    /// Applies all redaction annotations on this page using PyMuPDF-like defaults.
    pub fn apply_redactions(&mut self) -> Result<bool, Error> {
        self.apply_redactions_with_options(PdfRedactOptions::pymupdf_default())
    }

    pub fn apply_redactions_with_options(
        &mut self,
        options: PdfRedactOptions,
    ) -> Result<bool, Error> {
        self.redact_with_options(options)
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

    /// Returns this page's direct `/Resources` dictionary, creating and attaching one when missing.
    ///
    /// When the page has no direct resources but inherits a resource dictionary from its page
    /// tree, the inherited dictionary is shallow-copied into a page-local indirect dictionary
    /// before being attached. This avoids shadowing inherited resources with an empty dictionary
    /// when new resources are added to the page. Non-dictionary direct `/Resources` entries are
    /// replaced by a new empty indirect dictionary. Repeated calls return the same dictionary
    /// object without allocating again.
    pub fn resources(&self) -> Result<PdfObject, Error> {
        let mut page_obj = self.object();

        match page_obj.get_dict("Resources")? {
            Some(resources) if resources.is_dict()? => return Ok(resources),
            Some(_) => {}
            None => {
                if let Some(resources) = page_obj.get_dict_inheritable("Resources")? {
                    if resources.is_dict()? {
                        let mut doc = self.document_handle()?;
                        let resources = resources.copy_dict()?;
                        let resources = doc.add_object(&resources)?;
                        page_obj.dict_put("Resources", resources.clone())?;
                        return Ok(resources);
                    }
                }
            }
        }

        let mut doc = self.document_handle()?;
        let resources = doc.new_dict()?;
        let resources = doc.add_object(&resources)?;
        page_obj.dict_put("Resources", resources.clone())?;

        Ok(resources)
    }

    pub(crate) fn assert_document_owner(&self, doc: &PdfDocument) {
        assert_eq!(
            doc.as_raw(),
            unsafe { (*self.inner.as_ptr()).doc },
            "PdfPage ownership mismatch: the page is not attached to the provided PdfDocument"
        );
    }

    fn object_is_image(obj: &PdfObject) -> Result<bool, Error> {
        let Some(subtype) = obj.get_dict("Subtype")? else {
            return Ok(false);
        };
        Ok(subtype.is_name()? && subtype.as_name()? == b"Image")
    }

    fn object_to_metadata_string(obj: PdfObject) -> String {
        obj.to_string()
    }

    pub(crate) fn image_info_from_object(
        name: String,
        obj: &PdfObject,
    ) -> Result<Option<PageImageInfo>, Error> {
        let resolved = obj.resolve()?.unwrap_or_else(|| obj.clone());
        if !Self::object_is_image(&resolved)? {
            return Ok(None);
        }

        let xref = obj.as_indirect().unwrap_or(0);
        let width = resolved
            .get_dict("Width")?
            .map(|width| width.as_int())
            .transpose()?
            .unwrap_or(0)
            .max(0) as u32;
        let height = resolved
            .get_dict("Height")?
            .map(|height| height.as_int())
            .transpose()?
            .unwrap_or(0)
            .max(0) as u32;
        let bits_per_component = resolved
            .get_dict("BitsPerComponent")?
            .map(|bpc| bpc.as_int())
            .transpose()?;
        let color_space = resolved
            .get_dict("ColorSpace")?
            .map(Self::object_to_metadata_string);
        let filter = resolved
            .get_dict("Filter")?
            .map(Self::object_to_metadata_string);

        Ok(Some(PageImageInfo {
            name,
            xref,
            width,
            height,
            bits_per_component,
            color_space,
            filter,
        }))
    }

    fn existing_image_resources(&self) -> Result<Option<PdfObject>, Error> {
        let page_obj = self.object();
        let direct_resources = page_obj.get_dict("Resources")?;
        let resources = match direct_resources {
            Some(resources) if resources.is_dict()? => Some(resources),
            Some(_) => None,
            None => match page_obj.get_dict_inheritable("Resources")? {
                Some(resources) if resources.is_dict()? => Some(resources),
                _ => None,
            },
        };

        let Some(resources) = resources else {
            return Ok(None);
        };
        match resources.get_dict("XObject")? {
            Some(xobjects) if xobjects.is_dict()? => Ok(Some(xobjects)),
            _ => Ok(None),
        }
    }

    fn find_image_resource_by_xref(
        xobjects: &PdfObject,
        xref: i32,
    ) -> Result<Option<String>, Error> {
        for idx in 0..xobjects.dict_len()? {
            let Some(key) = xobjects.get_dict_key(idx as i32)? else {
                continue;
            };
            let Ok(key_name) = str::from_utf8(key.as_name()?) else {
                continue;
            };
            let Some(value) = xobjects.get_dict_val(idx as i32)? else {
                continue;
            };
            if value.is_indirect()? && value.as_indirect()? == xref {
                return Ok(Some(key_name.to_owned()));
            }
        }
        Ok(None)
    }

    fn next_image_resource_slot(xobjects: Option<&PdfObject>) -> Result<String, Error> {
        let mut used = Vec::new();

        if let Some(xobjects) = xobjects {
            for idx in 0..xobjects.dict_len()? {
                let Some(key) = xobjects.get_dict_key(idx as i32)? else {
                    continue;
                };
                let Ok(key_name) = str::from_utf8(key.as_name()?) else {
                    continue;
                };
                let Some(index) = key_name
                    .strip_prefix("Im")
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
        Ok(format!("Im{index}"))
    }

    fn add_image_resource(
        &mut self,
        doc: &mut PdfDocument,
        image_obj: &PdfObject,
    ) -> Result<String, Error> {
        let xref = image_obj.as_indirect()?;
        let mut resources = self.resources()?;
        let existing_xobjects = match resources.get_dict("XObject")? {
            Some(xobjects) if xobjects.is_dict()? => Some(xobjects),
            _ => None,
        };

        if let Some(xobjects) = existing_xobjects.as_ref() {
            if let Some(name) = Self::find_image_resource_by_xref(xobjects, xref)? {
                return Ok(name);
            }
        }

        let slot_name = Self::next_image_resource_slot(existing_xobjects.as_ref())?;
        let mut xobjects =
            Self::resource_subdict_for_write(&mut resources, doc, "XObject", existing_xobjects)?;
        xobjects.dict_put_ref(slot_name.as_str(), image_obj)?;

        Ok(slot_name)
    }

    fn image_object_from_source(
        doc: &mut PdfDocument,
        source: PageImageSource<'_>,
    ) -> Result<PdfObject, Error> {
        match source {
            PageImageSource::Image(image) => doc.add_image(image),
            PageImageSource::Pixmap(pixmap) => {
                let image = Image::from_pixmap(pixmap)?;
                doc.add_image(&image)
            }
            PageImageSource::Bytes { data, .. } => {
                let image = Image::from_bytes(data)?;
                doc.add_image(&image)
            }
            PageImageSource::ExistingXref(xref) => {
                doc.checked_xref(xref)?;
                let obj = doc.new_indirect(xref, 0)?;
                let Some(resolved) = obj.resolve()? else {
                    return Err(Error::InvalidArgument(format!(
                        "image xref {xref} does not resolve to an object"
                    )));
                };
                if !Self::object_is_image(&resolved)? {
                    return Err(Error::InvalidArgument(format!(
                        "xref {xref} does not refer to an image XObject"
                    )));
                }
                Ok(obj)
            }
        }
    }

    pub fn images(&self) -> Result<Vec<PageImageInfo>, Error> {
        let Some(xobjects) = self.existing_image_resources()? else {
            return Ok(Vec::new());
        };

        let mut images = Vec::new();
        for idx in 0..xobjects.dict_len()? {
            let Some(key) = xobjects.get_dict_key(idx as i32)? else {
                continue;
            };
            let Ok(key_name) = str::from_utf8(key.as_name()?) else {
                continue;
            };
            let Some(value) = xobjects.get_dict_val(idx as i32)? else {
                continue;
            };
            if let Some(info) = Self::image_info_from_object(key_name.to_owned(), &value)? {
                images.push(info);
            }
        }
        Ok(images)
    }

    pub fn insert_image(
        &mut self,
        doc: &mut PdfDocument,
        rect: Rect,
        source: PageImageSource<'_>,
        options: InsertImageOptions,
    ) -> Result<ImagePlacement, Error> {
        self.assert_document_owner(doc);
        if rect.is_empty() {
            return Err(Error::InvalidArgument(
                "image insertion requires a non-empty rectangle".to_owned(),
            ));
        }
        if let Some(opacity) = options.opacity {
            Self::validate_opacity_value(opacity, "image")?;
        }
        if let Some(oc_ref) = options.optional_content {
            Self::validate_optional_content_xref(doc, oc_ref.xref())?;
        }

        let operation = DocOperation::begin(doc, "Insert image")?;
        let image_obj = Self::image_object_from_source(operation.doc, source)?;
        let xref = image_obj.as_indirect()?;
        let name = self.add_image_resource(operation.doc, &image_obj)?;
        let gs = match options.opacity {
            Some(opacity) => self.register_ext_gstate(operation.doc, None, Some(opacity))?,
            None => None,
        };
        let oc = match options.optional_content {
            Some(oc_ref) => Some(self.register_optional_content(operation.doc, oc_ref.xref())?),
            None => None,
        };

        let mut content = String::new();
        content.push_str("q\n");
        if let Some(gs) = &gs {
            content.push_str(gs);
            content.push_str(" gs\n");
        }
        if let Some(oc) = &oc {
            content.push_str(oc);
            content.push_str(" BDC\n");
        }
        content.push_str(&format!(
            "{} 0 0 {} {} {} cm\n/{name} Do\n",
            format_pdf_number(rect.width()),
            format_pdf_number(rect.height()),
            format_pdf_number(rect.x0),
            format_pdf_number(rect.y0)
        ));
        if oc.is_some() {
            content.push_str("EMC\n");
        }
        content.push_str("Q\n");

        let contents_xref =
            self.insert_contents_in_operation(operation.doc, content.as_bytes(), options.overlay)?;
        operation.commit()?;

        Ok(ImagePlacement {
            name,
            xref,
            rect,
            contents_xref,
        })
    }

    pub fn replace_image(
        &mut self,
        doc: &mut PdfDocument,
        xref: i32,
        source: PageImageSource<'_>,
    ) -> Result<ImagePlacement, Error> {
        self.assert_document_owner(doc);
        doc.checked_xref(xref)?;
        let operation = DocOperation::begin(doc, "Replace image")?;
        let resources = self.resources()?;
        let Some(mut xobjects) = resources.get_dict("XObject")? else {
            return Err(Error::InvalidArgument(
                "page has no image resources".to_owned(),
            ));
        };
        if !xobjects.is_dict()? {
            return Err(Error::InvalidArgument(
                "page XObject resources are not a dictionary".to_owned(),
            ));
        }
        let Some(name) = Self::find_image_resource_by_xref(&xobjects, xref)? else {
            return Err(Error::InvalidArgument(format!(
                "page does not reference image xref {xref}"
            )));
        };
        let image_obj = Self::image_object_from_source(operation.doc, source)?;
        let new_xref = image_obj.as_indirect()?;
        xobjects.dict_put_ref(name.as_str(), &image_obj)?;
        operation.commit()?;

        Ok(ImagePlacement {
            name,
            xref: new_xref,
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            contents_xref: 0,
        })
    }

    pub fn delete_image(&mut self, doc: &mut PdfDocument, xref: i32) -> Result<bool, Error> {
        self.assert_document_owner(doc);
        doc.checked_xref(xref)?;
        let operation = DocOperation::begin(doc, "Delete image")?;
        let resources = self.resources()?;
        let Some(mut xobjects) = resources.get_dict("XObject")? else {
            operation.commit()?;
            return Ok(false);
        };
        if !xobjects.is_dict()? {
            return Err(Error::InvalidArgument(
                "page XObject resources are not a dictionary".to_owned(),
            ));
        }

        let mut deleted = false;
        let keys: Vec<_> = (0..xobjects.dict_len()?)
            .filter_map(|idx| {
                let value = xobjects.get_dict_val(idx as i32).ok().flatten()?;
                if value.is_indirect().ok()? && value.as_indirect().ok()? == xref {
                    let key = xobjects.get_dict_key(idx as i32).ok().flatten()?;
                    let key = str::from_utf8(key.as_name().ok()?).ok()?.to_owned();
                    Some(key)
                } else {
                    None
                }
            })
            .collect();
        for key in keys {
            xobjects.dict_delete(key.as_str())?;
            deleted = true;
        }
        operation.commit()?;
        Ok(deleted)
    }

    fn resource_subdict_for_write(
        resources: &mut PdfObject,
        doc: &PdfDocument,
        key: &str,
        existing: Option<PdfObject>,
    ) -> Result<PdfObject, Error> {
        if let Some(existing) = existing {
            let copy = existing.copy_dict()?;
            resources.dict_put_ref(key, &copy)?;
            return Ok(copy);
        }

        let subdict = doc.new_dict()?;
        resources.dict_put_ref(key, &subdict)?;
        Ok(subdict)
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
        self.assert_document_owner(doc);

        let operation = DocOperation::begin(doc, "Insert contents")?;
        let new_xref = self.insert_contents_in_operation(operation.doc, bytes, overlay)?;
        operation.commit()?;

        Ok(new_xref)
    }

    pub(crate) fn insert_contents_in_operation(
        &mut self,
        doc: &mut PdfDocument,
        bytes: &[u8],
        overlay: bool,
    ) -> Result<i32, Error> {
        let mut page_obj = self.object();
        let old_contents = page_obj.get_dict("Contents")?;

        let stream_buf = Buffer::from_bytes(bytes)?;
        let new_stream = doc.add_stream(&stream_buf, None, false)?;
        let new_xref = new_stream.as_indirect()?;

        let old_len = match old_contents.as_ref() {
            Some(contents) if contents.is_array()? => contents.len()? as i32,
            Some(contents) if !contents.is_null()? => 1,
            _ => 0,
        };
        let mut contents_array = doc.new_array_with_capacity(old_len + 1)?;

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

        Ok(new_xref)
    }

    /// Wraps this page's existing content streams in a balanced PDF graphics state.
    ///
    /// Inserts a `q\n` content stream as an underlay and a `Q\n` content stream as an
    /// overlay. This intentionally is not idempotent: repeated calls add another balanced
    /// pair around the existing contents.
    pub fn wrap_contents(&mut self, doc: &mut PdfDocument) -> Result<(), Error> {
        self.assert_document_owner(doc);

        let operation = DocOperation::begin(doc, "Wrap contents")?;
        self.insert_contents_in_operation(operation.doc, b"q\n", false)?;
        self.insert_contents_in_operation(operation.doc, b"Q\n", true)?;
        operation.commit()
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

    fn array_contains_indirect_xref(array: &PdfObject, xref: i32) -> Result<bool, Error> {
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

    fn ensure_array_contains_indirect_ref(
        array: &mut PdfObject,
        reference: &PdfObject,
        xref: i32,
    ) -> Result<(), Error> {
        if !Self::array_contains_indirect_xref(array, xref)? {
            array.array_push_ref(reference)?;
        }
        Ok(())
    }

    fn ensure_optional_content_config_array(
        doc: &PdfDocument,
        default_config: &mut PdfObject,
        key: &str,
        oc_ref: &PdfObject,
        oc_xref: i32,
    ) -> Result<(), Error> {
        let mut array = match default_config.get_dict(key)? {
            Some(array) if array.is_array()? => array,
            _ => {
                let array = doc.new_array()?;
                default_config.dict_put_ref(key, &array)?;
                array
            }
        };
        Self::ensure_array_contains_indirect_ref(&mut array, oc_ref, oc_xref)
    }

    fn ensure_optional_content_catalog_registration(
        doc: &PdfDocument,
        oc_ref: &PdfObject,
        oc_xref: i32,
    ) -> Result<(), Error> {
        let Some(oc_obj) = oc_ref.resolve()? else {
            return Ok(());
        };
        let Some(type_obj) = oc_obj.get_dict("Type")? else {
            return Ok(());
        };
        if type_obj.as_name()? != b"OCG" {
            return Ok(());
        }

        let mut catalog = doc.catalog()?;
        let mut oc_properties = match catalog.get_dict("OCProperties")? {
            Some(oc_properties) if oc_properties.is_dict()? => oc_properties,
            _ => {
                let oc_properties = doc.new_dict()?;
                catalog.dict_put_ref("OCProperties", &oc_properties)?;
                oc_properties
            }
        };

        let mut ocgs = match oc_properties.get_dict("OCGs")? {
            Some(ocgs) if ocgs.is_array()? => ocgs,
            _ => {
                let ocgs = doc.new_array()?;
                oc_properties.dict_put_ref("OCGs", &ocgs)?;
                ocgs
            }
        };
        if !Self::array_contains_indirect_xref(&ocgs, oc_xref)? {
            ocgs.array_push_ref(oc_ref)?;
        }

        let mut default_config = match oc_properties.get_dict("D")? {
            Some(default_config) if default_config.is_dict()? => default_config,
            _ => {
                let default_config = doc.new_dict()?;
                oc_properties.dict_put_ref("D", &default_config)?;
                default_config
            }
        };
        Self::ensure_optional_content_config_array(
            doc,
            &mut default_config,
            "ON",
            oc_ref,
            oc_xref,
        )?;
        Self::ensure_optional_content_config_array(
            doc,
            &mut default_config,
            "Order",
            oc_ref,
            oc_xref,
        )
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
        self.assert_document_owner(doc);

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
        let mut ext_gstates = Self::resource_subdict_for_write(
            &mut resources,
            doc,
            "ExtGState",
            existing_ext_gstates,
        )?;

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
        self.assert_document_owner(doc);

        Self::validate_optional_content_xref(doc, oc_xref)?;
        let oc_ref = doc.new_indirect(oc_xref, 0)?;
        Self::ensure_optional_content_catalog_registration(doc, &oc_ref, oc_xref)?;

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
        let mut properties = Self::resource_subdict_for_write(
            &mut resources,
            doc,
            "Properties",
            existing_properties,
        )?;

        properties.dict_put_ref(slot_name.as_str(), &oc_ref)?;

        Ok(format!("/{slot_name}"))
    }

    pub fn register_optional_content_ref(
        &mut self,
        doc: &mut PdfDocument,
        reference: OptionalContentRef,
    ) -> Result<String, Error> {
        self.register_optional_content(doc, reference.xref())
    }

    fn cached_font_matches(info: &FontInfo, name: &str, opts: &InsertFontOptions<'_>) -> bool {
        let opts_simple = opts.simple && opts.ordering.is_none() && opts.fontfile.is_none();
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
        let font = match (opts.fontfile, opts.ordering) {
            (Some(font_data), _) => Font::from_bytes(canonical_name, font_data)?,
            (None, Some(ordering)) => Font::new_cjk(ordering)?,
            (None, None) => Font::new(canonical_name)?,
        };

        let simple = opts.simple && opts.ordering.is_none() && opts.fontfile.is_none();
        let glyphs = if opts.fontfile.is_some() && opts.ordering.is_none() {
            Some(Self::font_glyph_map(&font))
        } else {
            None
        };

        let font_obj = if let Some(ordering) = opts.ordering {
            doc.add_cjk_font(&font, ordering, opts.wmode, opts.serif)?
        } else if simple {
            doc.add_simple_font(&font, opts.encoding)?
        } else {
            doc.add_font(&font)?
        };

        let info = FontInfo {
            ascender: font.ascender(),
            descender: font.descender(),
            glyphs,
            simple,
            encoding: opts.encoding,
            ordering: opts.ordering,
            wmode: opts.wmode,
            serif: opts.serif,
            fontfile_hash: Self::fontfile_hash(opts.fontfile),
            name: canonical_name.to_owned(),
        };

        Ok((font_obj, info))
    }

    fn font_glyph_map(font: &Font) -> HashMap<u32, i32> {
        let mut glyphs = HashMap::new();
        for code in 0..=255_u32 {
            if let Ok(glyph) = font.encode_character(code as i32) {
                glyphs.insert(code, glyph);
            }
        }
        glyphs
    }

    pub fn insert_font(
        &mut self,
        doc: &mut PdfDocument,
        opts: &InsertFontOptions<'_>,
    ) -> Result<(String, i32, FontInfo), Error> {
        self.assert_document_owner(doc);

        let mut opts = *opts;
        let canonical_name = match opts.fontfile {
            Some(_) => opts.name.to_owned(),
            None => {
                if let Some(ordering) = opts
                    .ordering
                    .or_else(|| cjk_ordering_from_font_name(opts.name))
                {
                    opts.ordering = Some(ordering);
                    cjk_font_name(ordering).to_owned()
                } else {
                    canonical_base14_name(opts.name)
                        .ok_or_else(|| {
                            Error::InvalidArgument(format!("unsupported font: {}", opts.name))
                        })?
                        .to_owned()
                }
            }
        };

        let page_obj = self.object();
        let direct_resources = page_obj.get_dict("Resources")?;
        let existing_resources = match direct_resources {
            Some(resources) if resources.is_dict()? => Some(resources),
            Some(_) => None,
            None => match page_obj.get_dict_inheritable("Resources")? {
                Some(resources) if resources.is_dict()? => Some(resources),
                _ => None,
            },
        };
        let existing_font_dict = match existing_resources
            .as_ref()
            .map(|resources| resources.get_dict("Font"))
            .transpose()?
            .flatten()
        {
            Some(font_dict) if font_dict.is_dict()? => Some(font_dict),
            _ => None,
        };

        if let Some(font_dict) = existing_font_dict.as_ref() {
            if let Some(existing) =
                Self::find_registered_page_font(doc, font_dict, &canonical_name, &opts)?
            {
                return Ok(existing);
            }
        }

        let (font_obj, info) = if let Some((xref, info)) =
            Self::find_cached_document_font(doc, &canonical_name, &opts)
        {
            (doc.new_indirect(xref, 0)?, info)
        } else {
            Self::build_font_object(doc, &canonical_name, &opts)?
        };
        let xref = font_obj.as_indirect()?;

        let slot_name = Self::next_font_resource_slot(existing_font_dict.as_ref())?;
        let mut resources = self.resources()?;
        let mut font_dict =
            Self::resource_subdict_for_write(&mut resources, doc, "Font", existing_font_dict)?;

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
        self.set_rotation_raw(rotate)
    }

    fn set_rotation_raw(&self, rotate: i32) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_page_set_rotation(
                context(),
                self.as_ptr().cast_mut(),
                rotate
            ))
        }
    }

    /// Extracts the page's vector drawings in unrotated page coordinates.
    ///
    /// PyMuPDF temporarily clears PDF page rotation before extracting drawings; this mirrors that
    /// behavior for [`PdfPage`].
    pub fn drawings(&self) -> Result<Vec<Drawing>, Error> {
        let rotation = self.rotation()?;
        if rotation % 90 != 0 {
            return Err(Error::InvalidArgument(format!(
                "page rotation must be a multiple of 90, got {rotation}"
            )));
        }
        if rotation == 0 {
            return Deref::deref(self).drawings();
        }

        let guard = PdfPageRotationGuard::new(self, rotation)?;
        let drawings = Deref::deref(self).drawings();
        let restore = guard.restore();

        match (drawings, restore) {
            (Ok(drawings), Ok(())) => Ok(drawings),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
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

        self.assert_document_owner(doc);

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

fn set_annotation_area(annot: &mut PdfAnnotation, area: AnnotationArea) -> Result<(), Error> {
    match area {
        AnnotationArea::Rect(rect) => {
            let rect = validate_non_empty_rect(
                rect,
                "redaction annotation requires a non-empty valid area",
            )?;
            annot.set_rect(rect)
        }
        AnnotationArea::QuadPoints(quads) => {
            let rect = validate_quad_area(
                quads.as_slice(),
                "redaction annotation requires a non-empty valid area",
            )?;
            annot.set_rect(rect)?;
            annot.set_quad_points(quads)
        }
    }
}

fn validate_redaction_area(area: AnnotationArea) -> Result<AnnotationArea, Error> {
    match area {
        AnnotationArea::Rect(rect) => {
            validate_non_empty_rect(rect, "redaction annotation requires a non-empty valid area")
                .map(AnnotationArea::Rect)
        }
        AnnotationArea::QuadPoints(quads) => {
            validate_quad_area(
                quads.as_slice(),
                "redaction annotation requires a non-empty valid area",
            )?;
            Ok(AnnotationArea::QuadPoints(quads))
        }
    }
}

fn validate_quad_area(quads: &[crate::Quad], message: &str) -> Result<Rect, Error> {
    if quads.is_empty() {
        return Err(Error::InvalidArgument(message.to_owned()));
    }

    for quad in quads {
        validate_non_empty_rect(Rect::from(quad.clone()), message)?;
        if quad_area(quad).abs() <= f32::EPSILON {
            return Err(Error::InvalidArgument(message.to_owned()));
        }
    }

    let rect =
        bounding_rect_for_quads(quads).ok_or_else(|| Error::InvalidArgument(message.to_owned()))?;
    validate_non_empty_rect(rect, message)
}

fn quad_area(quad: &crate::Quad) -> f32 {
    let points = [quad.ul, quad.ur, quad.lr, quad.ll];
    let mut area = 0.0;
    for index in 0..points.len() {
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        area += current.x * next.y - next.x * current.y;
    }
    area * 0.5
}

fn validate_non_empty_rect(rect: Rect, message: &str) -> Result<Rect, Error> {
    if !rect.is_valid() || rect.is_empty() {
        return Err(Error::InvalidArgument(message.to_owned()));
    }
    Ok(rect)
}

const ANNOTATION_RECT_PADDING: f32 = 3.0;

fn padded_bounding_rect_for_points(points: &[Point]) -> Result<Rect, Error> {
    let mut iter = points.iter();
    let first = iter.next().ok_or_else(|| {
        Error::InvalidArgument("annotation requires at least one point".to_owned())
    })?;
    let (mut x0, mut y0, mut x1, mut y1) = (first.x, first.y, first.x, first.y);
    for point in iter {
        x0 = x0.min(point.x);
        y0 = y0.min(point.y);
        x1 = x1.max(point.x);
        y1 = y1.max(point.y);
    }
    validate_non_empty_rect(
        Rect::new(
            x0 - ANNOTATION_RECT_PADDING,
            y0 - ANNOTATION_RECT_PADDING,
            x1 + ANNOTATION_RECT_PADDING,
            y1 + ANNOTATION_RECT_PADDING,
        ),
        "annotation requires a non-empty valid point bounding box",
    )
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

struct PdfPageRotationGuard<'a> {
    page: &'a PdfPage,
    rotation: i32,
    had_direct_rotate: bool,
    restored: bool,
}

impl<'a> PdfPageRotationGuard<'a> {
    fn new(page: &'a PdfPage, rotation: i32) -> Result<Self, Error> {
        let had_direct_rotate = page.object().get_dict("Rotate")?.is_some();
        page.set_rotation_raw(0)?;
        Ok(Self {
            page,
            rotation,
            had_direct_rotate,
            restored: false,
        })
    }

    fn restore(mut self) -> Result<(), Error> {
        let result = self.restore_inner();
        if result.is_ok() {
            self.restored = true;
        }
        result
    }

    fn restore_inner(&self) -> Result<(), Error> {
        if self.had_direct_rotate {
            self.page.set_rotation_raw(self.rotation)?;
        } else {
            self.page.object().dict_delete("Rotate")?;
        }
        Ok(())
    }
}

impl Drop for PdfPageRotationGuard<'_> {
    fn drop(&mut self) {
        if !self.restored {
            let _ = self.restore_inner();
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
        InsertFontOptions, InsertImageOptions, PageImageSource, PdfAnnotation, PdfAnnotationType,
        PdfDocument, PdfObject, PdfPage,
    };
    use crate::shape::{Shape, TextOptions};
    use crate::{
        Buffer, CjkFontOrdering, Colorspace, Error, ImageFormat, Matrix, Pixel, Pixmap, Point,
        Rect, SimpleFontEncoding, Size,
    };

    const CUSTOM_FONT_BYTES: &[u8] = include_bytes!("../../tests/files/custom.ttf");

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

    fn test_pixmap(width: i32, height: i32, pixel: Pixel) -> Pixmap {
        let cs = Colorspace::device_rgb();
        let mut pixmap = Pixmap::new_with_w_h(&cs, width, height, false).unwrap();
        pixmap.set_rect(pixmap.rect(), pixel).unwrap();
        pixmap
    }

    fn default_font_options(name: &str) -> InsertFontOptions<'_> {
        InsertFontOptions::new(name)
    }

    fn custom_font_options<'a>(name: &'a str, bytes: &'a [u8]) -> InsertFontOptions<'a> {
        InsertFontOptions {
            fontfile: Some(bytes),
            ..InsertFontOptions::new(name)
        }
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

    fn page_font_dict_len_without_creating_resources(page: &PdfPage) -> Option<usize> {
        let resources = page.object().get_dict("Resources").unwrap()?;
        let font_dict = resources.get_dict("Font").unwrap()?;
        Some(font_dict.dict_len().unwrap() as usize)
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
    fn test_page_widgets_empty_and_signature_widget() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        assert_eq!(page.widgets().count(), 0);

        let widget = page.add_signature_widget("Sig1").unwrap();
        assert_eq!(widget.r#type().unwrap(), crate::WidgetType::Signature);
        assert_eq!(widget.name().unwrap().as_deref(), Some("Sig1"));
        assert!(!widget.is_signed().unwrap());

        let xref = widget.xref().unwrap();
        drop(widget);

        let widgets: Vec<_> = page.widgets().collect();
        assert_eq!(widgets.len(), 1);
        assert_eq!(widgets[0].xref().unwrap(), xref);
        assert!(page.load_widget(xref).unwrap().is_some());
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
    fn test_page_resources_copy_inherited_dict_when_missing() {
        let mut doc = PdfDocument::new();
        let page = doc.new_page(Size::A4).unwrap();
        let mut page_obj = page.object();
        let mut parent = page_obj.get_dict("Parent").unwrap().unwrap();
        page_obj.dict_delete("Resources").unwrap();

        let mut inherited_font = doc.new_dict().unwrap();
        inherited_font
            .dict_put("F9", doc.new_dict().unwrap())
            .unwrap();
        let mut inherited_resources = doc.new_dict().unwrap();
        inherited_resources
            .dict_put("Font", inherited_font.clone())
            .unwrap();
        parent
            .dict_put("Resources", inherited_resources.clone())
            .unwrap();

        let mut resources = page.resources().unwrap();
        resources
            .dict_put("X-Test", PdfObject::new_int(1).unwrap())
            .unwrap();

        assert!(page_obj
            .get_dict("Resources")
            .unwrap()
            .unwrap()
            .is_indirect()
            .unwrap());
        assert!(resources
            .get_dict("Font")
            .unwrap()
            .unwrap()
            .get_dict("F9")
            .unwrap()
            .is_some());
        assert!(inherited_resources.get_dict("X-Test").unwrap().is_none());
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

    #[test]
    fn test_page_insert_image_lists_and_extracts_resource() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let pixmap = test_pixmap(2, 3, Pixel::rgb(200, 10, 20));

        let placement = page
            .insert_image(
                &mut doc,
                Rect::new(10.0, 20.0, 30.0, 50.0),
                PageImageSource::Pixmap(&pixmap),
                InsertImageOptions::default(),
            )
            .unwrap();

        assert!(placement.xref > 0);
        assert_eq!(placement.name, "Im0");
        assert!(placement.contents_xref > 0);

        let images = page.images().unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].name, "Im0");
        assert_eq!(images[0].xref, placement.xref);
        assert_eq!(images[0].width, 2);
        assert_eq!(images[0].height, 3);

        let extracted = doc.extract_image(placement.xref).unwrap();
        assert_eq!(extracted.xref, placement.xref);
        assert_eq!(extracted.width, 2);
        assert_eq!(extracted.height, 3);
        assert!(!extracted.encoded.is_empty());
    }

    #[test]
    fn test_page_replace_and_delete_image_resource() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let first = test_pixmap(2, 2, Pixel::rgb(0, 0, 255));
        let second = test_pixmap(4, 1, Pixel::rgb(0, 255, 0));

        let placement = page
            .insert_image(
                &mut doc,
                Rect::new(0.0, 0.0, 20.0, 20.0),
                PageImageSource::Pixmap(&first),
                InsertImageOptions::default(),
            )
            .unwrap();
        let replacement = page
            .replace_image(&mut doc, placement.xref, PageImageSource::Pixmap(&second))
            .unwrap();

        assert_eq!(replacement.name, placement.name);
        assert_ne!(replacement.xref, placement.xref);

        let images = page.images().unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].xref, replacement.xref);
        assert_eq!(images[0].width, 4);
        assert_eq!(images[0].height, 1);

        assert!(page.delete_image(&mut doc, replacement.xref).unwrap());
        assert!(page.images().unwrap().is_empty());
        assert!(!page.delete_image(&mut doc, replacement.xref).unwrap());
        assert!(page
            .replace_image(&mut doc, 999, PageImageSource::Pixmap(&second))
            .is_err());
        assert!(page.delete_image(&mut doc, 999).is_err());
    }

    #[test]
    fn test_page_insert_image_from_encoded_bytes() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let pixmap = test_pixmap(2, 2, Pixel::rgb(255, 0, 0));
        let mut encoded = Vec::new();
        pixmap.write_to(&mut encoded, ImageFormat::PNG).unwrap();

        let placement = page
            .insert_image(
                &mut doc,
                Rect::new(0.0, 0.0, 10.0, 10.0),
                PageImageSource::Bytes {
                    data: &encoded,
                    format_hint: Some("png"),
                },
                InsertImageOptions::default(),
            )
            .unwrap();

        let images = page.images().unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].xref, placement.xref);
        assert_eq!(images[0].width, 2);
        assert_eq!(images[0].height, 2);
        assert!(page
            .insert_image(
                &mut doc,
                Rect::new(0.0, 0.0, 10.0, 10.0),
                PageImageSource::ExistingXref(999),
                InsertImageOptions::default(),
            )
            .is_err());
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
    fn test_page_insert_font_accepts_case_insensitive_base14_alias() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();

        let (slot, xref, info) = page
            .insert_font(&mut doc, &default_font_options("HELV"))
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
    fn test_page_insert_font_copies_inherited_font_dict_before_mutating() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut page_obj = page.object();
        let mut parent = page_obj.get_dict("Parent").unwrap().unwrap();
        page_obj.dict_delete("Resources").unwrap();

        let mut inherited_font = doc.new_dict().unwrap();
        inherited_font
            .dict_put("F9", doc.new_dict().unwrap())
            .unwrap();
        let mut inherited_resources = doc.new_dict().unwrap();
        inherited_resources
            .dict_put("Font", inherited_font.clone())
            .unwrap();
        parent.dict_put("Resources", inherited_resources).unwrap();

        let (slot, xref, _) = page
            .insert_font(&mut doc, &default_font_options("helv"))
            .unwrap();

        assert_eq!(slot, "/F0");
        assert_eq!(page_font_xref(&page, "F0"), xref);
        assert!(page_font_dict(&page).get_dict("F9").unwrap().is_some());
        assert!(inherited_font.get_dict("F0").unwrap().is_none());
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

    mod fonts {
        use super::*;

        #[test]
        fn custom_ttf_via_add_font() {
            let mut doc = PdfDocument::new();
            let mut page = doc.new_page(Size::A4).unwrap();
            let opts = custom_font_options("CustomPdfFont", CUSTOM_FONT_BYTES);

            let (slot, xref, info) = page.insert_font(&mut doc, &opts).unwrap();

            assert_eq!(slot, "/F0");
            assert!(xref > 0);
            assert_eq!(page_font_xref(&page, "F0"), xref);
            assert_eq!(info.name, "CustomPdfFont");
            assert!(!info.simple);
            assert_eq!(info.ordering, None);
            assert!(info.fontfile_hash.is_some());
            assert!(info
                .glyphs
                .as_ref()
                .is_some_and(|glyphs| glyphs.contains_key(&('A' as u32))));
            assert_eq!(doc.font_info_cache.borrow().get(&xref), Some(&info));

            let font_obj = doc.new_indirect(xref, 0).unwrap();
            let subtype = font_obj
                .get_dict("Subtype")
                .unwrap()
                .unwrap()
                .as_name()
                .unwrap()
                .to_vec();
            assert_eq!(subtype, b"Type0");
        }

        #[test]
        fn cjk_font_registers_with_ordering() {
            let mut doc = PdfDocument::new();
            let mut page = doc.new_page(Size::A4).unwrap();
            let opts = InsertFontOptions {
                fontfile: Some(CUSTOM_FONT_BYTES),
                ordering: Some(CjkFontOrdering::AdobeJapan),
                ..InsertFontOptions::new("CustomCjkFont")
            };

            let (_slot, _xref, info) = page.insert_font(&mut doc, &opts).unwrap();

            assert_eq!(info.ordering, Some(CjkFontOrdering::AdobeJapan));
            assert!(!info.simple);

            let mut doc = PdfDocument::new();
            let mut page = doc.new_page(Size::A4).unwrap();
            let mut shape = Shape::new(&mut page).unwrap();
            shape
                .insert_text(
                    Point::new(50.0, 100.0),
                    "A",
                    &TextOptions {
                        fontname: "CustomCjkFont".to_owned(),
                        fontfile: Some(CUSTOM_FONT_BYTES),
                        ordering: Some(CjkFontOrdering::AdobeJapan),
                        ..Default::default()
                    },
                )
                .unwrap();

            assert!(shape.text_cont().contains("[<0041>] TJ"));
        }

        #[test]
        fn cjk_font_alias_registers_without_fontfile() {
            let mut doc = PdfDocument::new();
            let mut page = doc.new_page(Size::A4).unwrap();
            let opts = InsertFontOptions::new("JAPAN");

            match page.insert_font(&mut doc, &opts) {
                Ok((slot, xref, info)) => {
                    assert_eq!(slot, "/F0");
                    assert!(xref > 0);
                    assert_eq!(info.name, "japan");
                    assert_eq!(info.ordering, Some(CjkFontOrdering::AdobeJapan));
                    assert!(!info.simple);
                }
                Err(Error::MuPdf(err)) if err.message.contains("builtin CJK font") => {}
                Err(err) => panic!("unexpected CJK alias error: {err:?}"),
            }
        }

        #[test]
        fn custom_font_shared_across_pages() {
            let mut doc = PdfDocument::new();
            let mut first_page = doc.new_page(Size::A4).unwrap();
            let mut second_page = doc.new_page(Size::A4).unwrap();
            let opts = custom_font_options("SharedCustomFont", CUSTOM_FONT_BYTES);

            let first = first_page.insert_font(&mut doc, &opts).unwrap();
            let second = second_page.insert_font(&mut doc, &opts).unwrap();

            assert_eq!(first.1, second.1);
            assert_eq!(page_font_xref(&first_page, "F0"), first.1);
            assert_eq!(page_font_xref(&second_page, "F0"), first.1);
            assert_eq!(doc.font_info_cache.borrow().len(), 1);
        }

        #[test]
        fn malformed_bytes_errors() {
            let mut doc = PdfDocument::new();
            let mut page = doc.new_page(Size::A4).unwrap();
            page.object().dict_delete("Resources").unwrap();
            let cache_len = doc.font_info_cache.borrow().len();
            let object_count = doc.count_objects().unwrap();
            let opts = custom_font_options("BrokenFont", b"not a font");

            let result = page.insert_font(&mut doc, &opts);

            assert!(result.is_err());
            assert!(page.object().get_dict("Resources").unwrap().is_none());
            assert_eq!(page_font_dict_len_without_creating_resources(&page), None);
            assert_eq!(doc.font_info_cache.borrow().len(), cache_len);
            assert_eq!(doc.count_objects().unwrap(), object_count);
        }
    }
}

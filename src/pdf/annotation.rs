use std::{
    convert::TryFrom,
    ffi::{c_char, c_int, c_uint, CStr, CString},
    ptr::NonNull,
};

use mupdf_sys::*;

use crate::{color::AnnotationColor, pdf::Intent};
use crate::{context, from_enum, Error};
use crate::{pdf::PdfFilterOptions, pdf::PdfObject, Matrix, Point, Quad, Rect};

from_enum! { pdf_annot_type => c_uint,
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum PdfAnnotationType {
        Text = PDF_ANNOT_TEXT,
        Link = PDF_ANNOT_LINK,
        FreeText = PDF_ANNOT_FREE_TEXT,
        Line = PDF_ANNOT_LINE,
        Square = PDF_ANNOT_SQUARE,
        Circle = PDF_ANNOT_CIRCLE,
        Polygon = PDF_ANNOT_POLYGON,
        PolyLine = PDF_ANNOT_POLY_LINE,
        Highlight = PDF_ANNOT_HIGHLIGHT,
        Underline = PDF_ANNOT_UNDERLINE,
        Squiggly = PDF_ANNOT_SQUIGGLY,
        StrikeOut = PDF_ANNOT_STRIKE_OUT,
        Redact = PDF_ANNOT_REDACT,
        Stamp = PDF_ANNOT_STAMP,
        Caret = PDF_ANNOT_CARET,
        Ink = PDF_ANNOT_INK,
        Popup = PDF_ANNOT_POPUP,
        FileAttachment = PDF_ANNOT_FILE_ATTACHMENT,
        Sound = PDF_ANNOT_SOUND,
        Movie = PDF_ANNOT_MOVIE,
        RichMedia = PDF_ANNOT_RICH_MEDIA,
        Widget = PDF_ANNOT_WIDGET,
        Screen = PDF_ANNOT_SCREEN,
        PrinterMark = PDF_ANNOT_PRINTER_MARK,
        TrapNet = PDF_ANNOT_TRAP_NET,
        Watermark = PDF_ANNOT_WATERMARK,
        ThreeD = PDF_ANNOT_3D,
        Projection = PDF_ANNOT_PROJECTION,
        Unknown = PDF_ANNOT_UNKNOWN,
    }
}

from_enum! { pdf_line_ending => pdf_line_ending,
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum LineEndingStyle {
        None = PDF_ANNOT_LE_NONE,
        Square = PDF_ANNOT_LE_SQUARE,
        Circle = PDF_ANNOT_LE_CIRCLE,
        Diamond = PDF_ANNOT_LE_DIAMOND,
        OpenArrow = PDF_ANNOT_LE_OPEN_ARROW,
        ClosedArrow = PDF_ANNOT_LE_CLOSED_ARROW,
        Butt = PDF_ANNOT_LE_BUTT,
        ROpenArrow = PDF_ANNOT_LE_R_OPEN_ARROW,
        RClosedArrow = PDF_ANNOT_LE_R_CLOSED_ARROW,
        Slash = PDF_ANNOT_LE_SLASH,
    }
}

from_enum! { pdf_border_style => pdf_border_style,
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum AnnotationBorderStyle {
        Solid = PDF_BORDER_STYLE_SOLID,
        Dashed = PDF_BORDER_STYLE_DASHED,
        Beveled = PDF_BORDER_STYLE_BEVELED,
        Inset = PDF_BORDER_STYLE_INSET,
        Underline = PDF_BORDER_STYLE_UNDERLINE,
    }
}

from_enum! { pdf_border_effect => pdf_border_effect,
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum AnnotationBorderEffect {
        None = PDF_BORDER_EFFECT_NONE,
        Cloudy = PDF_BORDER_EFFECT_CLOUDY,
    }
}

from_enum! { c_int => c_int,
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub enum AnnotationTextAlign {
        #[default]
        Left = PDF_ANNOT_Q_LEFT,
        Center = PDF_ANNOT_Q_CENTER,
        Right = PDF_ANNOT_Q_RIGHT,
    }
}

from_enum! { c_int => c_int,
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub enum PdfRedactImageMethod {
        None = PDF_REDACT_IMAGE_NONE,
        Remove = PDF_REDACT_IMAGE_REMOVE,
        #[default]
        Pixels = PDF_REDACT_IMAGE_PIXELS,
        RemoveUnlessInvisible = PDF_REDACT_IMAGE_REMOVE_UNLESS_INVISIBLE,
    }
}

from_enum! { c_int => c_int,
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub enum PdfRedactLineArtMethod {
        #[default]
        None = PDF_REDACT_LINE_ART_NONE,
        RemoveIfCovered = PDF_REDACT_LINE_ART_REMOVE_IF_COVERED,
        RemoveIfTouched = PDF_REDACT_LINE_ART_REMOVE_IF_TOUCHED,
    }
}

from_enum! { c_int => c_int,
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub enum PdfRedactTextMethod {
        #[default]
        Remove = PDF_REDACT_TEXT_REMOVE,
        None = PDF_REDACT_TEXT_NONE,
        RemoveInvisible = PDF_REDACT_TEXT_REMOVE_INVISIBLE,
    }
}

/// Options controlling how redaction annotations are applied.
///
/// The default matches MuPDF's `NULL` redaction options and therefore preserves the behavior of
/// [`PdfPage::redact`](crate::pdf::PdfPage::redact): no black boxes, pixel redaction for images,
/// no line-art redaction, and text removal. Use [`Self::pymupdf_default`] for PyMuPDF-like
/// defaults.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PdfRedactOptions {
    pub black_boxes: bool,
    pub image_method: PdfRedactImageMethod,
    pub line_art: PdfRedactLineArtMethod,
    pub text: PdfRedactTextMethod,
}

impl PdfRedactOptions {
    pub const fn mupdf_default() -> Self {
        Self {
            black_boxes: false,
            image_method: PdfRedactImageMethod::Pixels,
            line_art: PdfRedactLineArtMethod::None,
            text: PdfRedactTextMethod::Remove,
        }
    }

    pub const fn pymupdf_default() -> Self {
        Self {
            line_art: PdfRedactLineArtMethod::RemoveIfCovered,
            ..Self::mupdf_default()
        }
    }

    pub(crate) fn into_raw(self) -> pdf_redact_options {
        pdf_redact_options {
            black_boxes: c_int::from(self.black_boxes),
            image_method: self.image_method.into(),
            line_art: self.line_art.into(),
            text: self.text.into(),
        }
    }
}

impl Default for PdfRedactOptions {
    fn default() -> Self {
        Self::mupdf_default()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationDefaultAppearance {
    pub font_name: String,
    pub size: f32,
    pub color: Option<AnnotationColor>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationQuadPoints(Vec<Quad>);

impl AnnotationQuadPoints {
    pub fn new(quads: impl IntoIterator<Item = Quad>) -> Self {
        Self(quads.into_iter().collect())
    }

    pub fn as_slice(&self) -> &[Quad] {
        &self.0
    }

    pub fn into_vec(self) -> Vec<Quad> {
        self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<Quad> for AnnotationQuadPoints {
    fn from(quad: Quad) -> Self {
        Self(vec![quad])
    }
}

impl From<Vec<Quad>> for AnnotationQuadPoints {
    fn from(quads: Vec<Quad>) -> Self {
        Self(quads)
    }
}

impl From<&[Quad]> for AnnotationQuadPoints {
    fn from(quads: &[Quad]) -> Self {
        Self(quads.to_vec())
    }
}

impl<const N: usize> From<[Quad; N]> for AnnotationQuadPoints {
    fn from(quads: [Quad; N]) -> Self {
        Self(quads.into_iter().collect())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnnotationArea {
    Rect(Rect),
    QuadPoints(AnnotationQuadPoints),
}

impl From<Rect> for AnnotationArea {
    fn from(rect: Rect) -> Self {
        Self::Rect(rect)
    }
}

impl From<Quad> for AnnotationArea {
    fn from(quad: Quad) -> Self {
        Self::QuadPoints(quad.into())
    }
}

impl From<Vec<Quad>> for AnnotationArea {
    fn from(quads: Vec<Quad>) -> Self {
        Self::QuadPoints(quads.into())
    }
}

impl From<&[Quad]> for AnnotationArea {
    fn from(quads: &[Quad]) -> Self {
        Self::QuadPoints(quads.into())
    }
}

impl<const N: usize> From<[Quad; N]> for AnnotationArea {
    fn from(quads: [Quad; N]) -> Self {
        Self::QuadPoints(quads.into())
    }
}

#[derive(Debug)]
pub struct PdfAnnotation {
    pub(crate) inner: NonNull<pdf_annot>,
    /// Ref-counted page pointer that keeps the parent page (and transitively
    /// the document) alive for as long as this annotation exists.
    page: NonNull<pdf_page>,
}

impl PdfAnnotation {
    /// Create a `PdfAnnotation` from pointer.
    ///
    /// # Safety
    ///
    /// * `ptr` must be non-null and point to a valid `pdf_annot`.
    /// * `ptr` must be attached to a parent page.
    /// * The caller must own one reference to `ptr` (e.g. from a create call)
    ///   and must not drop it afterwards — this wrapper assumes ownership.
    /// * The parent page must be alive at the time of this call.
    pub(crate) unsafe fn from_raw(ptr: *mut pdf_annot) -> Self {
        let inner = NonNull::new(ptr).expect("PdfAnnotation::from_raw received a null pointer");
        let page = unsafe { Self::attached_page(inner) };
        unsafe {
            pdf_keep_page(context(), page.as_ptr());
        }
        Self { inner, page }
    }

    unsafe fn attached_page(annot: NonNull<pdf_annot>) -> NonNull<pdf_page> {
        let page = unsafe { pdf_annot_page(context(), annot.as_ptr()) };
        NonNull::new(page)
            .expect("PdfAnnotation::from_raw requires an annotation attached to a page")
    }

    /// Create a `PdfAnnotation` from a borrowed pointer.
    ///
    /// Increments the annotation's reference count before taking ownership.
    ///
    /// # Safety
    ///
    /// * `ptr` must be non-null and point to a valid `pdf_annot`.
    /// * `ptr` must be attached to a parent page.
    /// * The parent page must be alive at the time of this call.
    pub(crate) unsafe fn from_raw_keep_ref(ptr: *mut pdf_annot) -> Self {
        let inner =
            NonNull::new(ptr).expect("PdfAnnotation::from_raw_keep_ref received a null pointer");
        let page = unsafe { Self::attached_page(inner) };
        unsafe {
            pdf_keep_annot(context(), inner.as_ptr());
            pdf_keep_page(context(), page.as_ptr());
        }
        Self { inner, page }
    }

    fn is_attached(&self) -> bool {
        !unsafe { pdf_annot_page(context(), self.inner.as_ptr()) }.is_null()
    }

    pub(crate) fn ensure_attached(&self) -> Result<(), Error> {
        if !self.is_attached() {
            return Err(Error::InvalidArgument(
                "annotation is no longer attached to a page".to_owned(),
            ));
        }
        Ok(())
    }

    pub(crate) fn page_ptr(&self) -> Result<*mut pdf_page, Error> {
        let page = unsafe { pdf_annot_page(context(), self.inner.as_ptr()) };
        if page.is_null() {
            return Err(Error::InvalidArgument(
                "annotation is no longer attached to a page".to_owned(),
            ));
        }
        Ok(page)
    }

    /// Get the [`PdfAnnotationType`] of this annotation
    pub fn r#type(&self) -> Result<PdfAnnotationType, Error> {
        self.ensure_attached()?;
        unsafe { ffi_try!(mupdf_pdf_annot_type(context(), self.inner.as_ptr())) }.map(|subtype| {
            PdfAnnotationType::try_from(subtype).unwrap_or(PdfAnnotationType::Unknown)
        })
    }

    /// Returns the underlying annotation dictionary object.
    pub fn object(&self) -> PdfObject {
        unsafe { PdfObject::from_raw_keep_ref(pdf_annot_obj(context(), self.inner.as_ptr())) }
    }

    /// Returns the PDF xref number of the underlying annotation object.
    pub fn xref(&self) -> Result<i32, Error> {
        self.ensure_attached()?;
        self.object().as_indirect()
    }

    /// Returns the annotation's display bounds.
    pub fn bounds(&self) -> Result<Rect, Error> {
        self.ensure_attached()?;
        unsafe { ffi_try!(mupdf_pdf_bound_annot(context(), self.inner.as_ptr())) }.map(Into::into)
    }

    /// Regenerates the annotation appearance stream if needed.
    pub fn update(&mut self) -> Result<bool, Error> {
        self.ensure_attached()?;
        unsafe { ffi_try!(mupdf_pdf_update_annot(context(), self.inner.as_ptr())) }
            .map(|changed| changed != 0)
    }

    /// Applies this single redaction annotation using MuPDF's default redaction behavior.
    pub fn apply_redaction(self) -> Result<bool, Error> {
        self.apply_redaction_with_options(PdfRedactOptions::default())
    }

    /// Applies this single redaction annotation using the provided options.
    pub fn apply_redaction_with_options(self, options: PdfRedactOptions) -> Result<bool, Error> {
        self.ensure_attached()?;
        let mut raw = options.into_raw();
        unsafe {
            ffi_try!(mupdf_pdf_apply_redaction(
                context(),
                self.inner.as_ptr(),
                &raw mut raw
            ))
        }
    }

    /// Check if the annotation is hot (i.e. that the pointing device's cursor is hovering over the
    /// annotation)
    pub fn is_hot(&self) -> bool {
        if !self.is_attached() {
            return false;
        }
        unsafe { pdf_annot_hot(context(), self.inner.as_ptr()) != 0 }
    }

    /// Make this "hot" (see [`Self::is_hot()`])
    pub fn set_hot(&mut self, hot: bool) {
        if !self.is_attached() {
            return;
        }
        // Just kinda trusting it would be insane of them to throw here
        unsafe { pdf_set_annot_hot(context(), self.inner.as_ptr(), i32::from(hot)) }
    }

    pub fn is_active(&self) -> bool {
        if !self.is_attached() {
            return false;
        }
        unsafe { pdf_annot_active(context(), self.inner.as_ptr()) != 0 }
    }

    pub fn flags(&self) -> Result<AnnotationFlags, Error> {
        self.ensure_attached()?;
        unsafe { ffi_try!(mupdf_pdf_annot_flags(context(), self.inner.as_ptr())) }
            .map(AnnotationFlags::from_bits_retain)
    }

    pub fn color(&self) -> Result<Option<AnnotationColor>, Error> {
        self.ensure_attached()?;
        let mut color = [0.0; 4];
        let n = unsafe {
            ffi_try!(mupdf_pdf_annot_color(
                context(),
                self.inner.as_ptr(),
                color.as_mut_ptr()
            ))
        }?;
        annotation_color_from_components(n, color)
    }

    pub fn interior_color(&self) -> Result<Option<AnnotationColor>, Error> {
        self.ensure_attached()?;
        let mut color = [0.0; 4];
        let n = unsafe {
            ffi_try!(mupdf_pdf_annot_interior_color(
                context(),
                self.inner.as_ptr(),
                color.as_mut_ptr()
            ))
        }?;
        annotation_color_from_components(n, color)
    }

    pub fn set_line(&mut self, start: Point, end: Point) -> Result<(), Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_set_annot_line(
                context(),
                self.inner.as_ptr(),
                start.into(),
                end.into()
            ))
        }
    }

    pub fn line(&self) -> Result<(Point, Point), Error> {
        self.ensure_attached()?;
        let mut start = fz_point { x: 0.0, y: 0.0 };
        let mut end = fz_point { x: 0.0, y: 0.0 };
        unsafe {
            ffi_try!(mupdf_pdf_annot_line(
                context(),
                self.inner.as_ptr(),
                &raw mut start,
                &raw mut end
            ))
        }?;
        Ok((start.into(), end.into()))
    }

    pub fn line_ending_styles(&self) -> Result<(LineEndingStyle, LineEndingStyle), Error> {
        self.ensure_attached()?;
        let mut start = PDF_ANNOT_LE_NONE;
        let mut end = PDF_ANNOT_LE_NONE;
        unsafe {
            ffi_try!(mupdf_pdf_annot_line_ending_styles(
                context(),
                self.inner.as_ptr(),
                &raw mut start,
                &raw mut end
            ))
        }?;
        Ok((
            LineEndingStyle::try_from(start)?,
            LineEndingStyle::try_from(end)?,
        ))
    }

    pub fn set_line_ending_styles(
        &mut self,
        start: LineEndingStyle,
        end: LineEndingStyle,
    ) -> Result<(), Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_set_annot_line_ending_styles(
                context(),
                self.inner.as_ptr(),
                start.into(),
                end.into()
            ))
        }
    }

    pub fn set_color(&mut self, color: AnnotationColor) -> Result<(), Error> {
        self.ensure_attached()?;
        let (n, components) = annotation_color_components(color);
        unsafe {
            ffi_try!(mupdf_pdf_set_annot_color(
                context(),
                self.inner.as_ptr(),
                n,
                components.as_ptr()
            ))
        }
    }

    pub fn set_interior_color(&mut self, color: AnnotationColor) -> Result<(), Error> {
        self.ensure_attached()?;
        let (n, components) = annotation_color_components(color);
        unsafe {
            ffi_try!(mupdf_pdf_set_annot_interior_color(
                context(),
                self.inner.as_ptr(),
                n,
                components.as_ptr()
            ))
        }
    }

    pub fn set_flags(&mut self, flags: AnnotationFlags) -> Result<(), Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_set_annot_flags(
                context(),
                self.inner.as_ptr(),
                flags.bits()
            ))
        }
    }

    /// Set the bounding box of the annotation
    pub fn set_rect(&mut self, rect: Rect) -> Result<(), Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_set_annot_rect(
                context(),
                self.inner.as_ptr(),
                rect.into()
            ))
        }
    }

    /// Get the annotation rectangle.
    pub fn rect(&self) -> Result<Rect, Error> {
        self.ensure_attached()?;
        match unsafe { ffi_try!(mupdf_pdf_annot_rect(context(), self.inner.as_ptr())) } {
            Ok(rect) => Ok(rect.into()),
            Err(err) => {
                let rect = annotation_rect_from_dict(&self.object())?.ok_or(err)?;
                let ctm: Matrix =
                    unsafe { ffi_try!(mupdf_pdf_page_transform(context(), self.page_ptr()?)) }?
                        .into();
                Ok(rect.transform(&ctm))
            }
        }
    }

    /// Get the annotation display rectangle adjusted for `NoZoom` and `NoRotate` flags.
    pub fn display_rect(&self) -> Result<Rect, Error> {
        self.ensure_attached()?;
        unsafe { ffi_try!(mupdf_pdf_annot_display_rect(context(), self.inner.as_ptr())) }
            .map(Into::into)
    }

    pub fn quad_points(&self) -> Result<Vec<Quad>, Error> {
        self.ensure_attached()?;
        let count = unsafe {
            ffi_try!(mupdf_pdf_annot_quad_point_count(
                context(),
                self.inner.as_ptr()
            ))
        }?;
        (0..count)
            .map(|i| {
                unsafe {
                    ffi_try!(mupdf_pdf_annot_quad_point(
                        context(),
                        self.inner.as_ptr(),
                        i
                    ))
                }
                .map(Into::into)
            })
            .collect()
    }

    pub fn set_quad_points(&mut self, quads: impl Into<AnnotationQuadPoints>) -> Result<(), Error> {
        self.ensure_attached()?;
        let quads = quads.into();
        if quads.is_empty() {
            return Err(Error::InvalidArgument(
                "annotation quad points must not be empty".to_owned(),
            ));
        }
        let raw = raw_quads(quads.as_slice());
        unsafe {
            ffi_try!(mupdf_pdf_set_annot_quad_points(
                context(),
                self.inner.as_ptr(),
                raw.len().try_into()?,
                raw.as_ptr()
            ))
        }
    }

    pub fn clear_quad_points(&mut self) -> Result<(), Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_clear_annot_quad_points(
                context(),
                self.inner.as_ptr()
            ))
        }
    }

    pub fn add_quad_point(&mut self, quad: Quad) -> Result<(), Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_add_annot_quad_point(
                context(),
                self.inner.as_ptr(),
                quad.into()
            ))
        }
    }

    /* Name convention note regarding quads:
    I found confusing how quad-related functions were named in MuPDF,
    since there exists `pdf_anot_quad_point_count()`, which is not about
    counting the number of points but about counting the number
    of 4-point / 8-integer quads in a PDF quad array.

    The mupdf wrapper functions for quads are faithful to mupdf's naming
    convention, while on the Rust side `has_quads()` is favoured over
    `has_quad_points()`, likewise with `quad_count()` over `quad_point_count()`.
    This seems in line with the nomenclature in `quad.rs`
    */

    pub fn has_quads(&self) -> Result<bool, Error> {
      let result = unsafe {
            ffi_try!(
                mupdf_pdf_annot_has_quad_points(context(), self.inner.as_ptr())
            )
        }?;

        Ok(result != 0)
    }

    pub fn quad_count(&self) -> Result<u32, Error> {
        if !self.has_quads()? {
            // We could alternatively choose to error this. That was my first
            // instinct, but did not get to figuring it out yet.
            return Ok(0);
        }

        let count = unsafe {
            ffi_try!(mupdf_pdf_annot_quad_point_count(
                context(),
                self.inner.as_ptr()
            ))
        }?;

        Ok(count as u32)
    }

    // Deliberate choice to not make this one public (in favour of just
    // relying on `quads()`. Debatable.
    fn quad(&self, quad_index: u32) -> Result<Quad, Error> {
        unsafe {
            ffi_try!(
                mupdf_pdf_annot_quad_point(
                    context(),
                    self.inner.as_ptr(),
                    quad_index as c_int
                )
            )
        }
        .map(Into::into)
    }

    pub fn quads(&self) -> Result<Vec<Quad>, Error> {
        let quad_count = self.quad_count()?;

        let mut quad_vec = Vec::with_capacity(quad_count as usize);

        for i in 0..quad_count {
            quads_vec.push(self.quad(i)?);
        }

        Ok(quads_vec)
    }

    pub fn author(&self) -> Result<Option<&str>, Error> {
        self.ensure_attached()?;
        let ptr = unsafe { ffi_try!(mupdf_pdf_annot_author(context(), self.inner.as_ptr())) }?;
        if ptr.is_null() {
            return Ok(None);
        }
        let c_str = unsafe { CStr::from_ptr(ptr) };
        Ok(Some(c_str.to_str().map_err(|_| Error::InvalidUtf8)?))
    }

    pub fn set_author(&mut self, author: &str) -> Result<(), Error> {
        self.ensure_attached()?;
        let c_author = CString::new(author)?;
        unsafe {
            ffi_try!(mupdf_pdf_set_annot_author(
                context(),
                self.inner.as_ptr(),
                c_author.as_ptr()
            ))
        }
    }

    pub fn contents(&self) -> Result<Option<&str>, Error> {
        self.ensure_attached()?;
        let ptr = unsafe { ffi_try!(mupdf_pdf_annot_contents(context(), self.inner.as_ptr())) }?;
        if ptr.is_null() {
            return Ok(None);
        }
        let c_str = unsafe { CStr::from_ptr(ptr) };
        Ok(Some(c_str.to_str().map_err(|_| Error::InvalidUtf8)?))
    }

    pub fn set_contents(&mut self, contents: &str) -> Result<(), Error> {
        self.ensure_attached()?;
        let c_contents = CString::new(contents)?;
        unsafe {
            ffi_try!(mupdf_pdf_set_annot_contents(
                context(),
                self.inner.as_ptr(),
                c_contents.as_ptr()
            ))
        }
    }

    pub fn filter(&mut self, mut opt: PdfFilterOptions) -> Result<(), Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_filter_annot_contents(
                context(),
                self.inner.as_ptr(),
                &raw mut opt.inner
            ))
        }
    }

    pub fn set_popup(&mut self, rect: Rect) -> Result<(), Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_set_annot_popup(
                context(),
                self.inner.as_ptr(),
                fz_rect::from(rect)
            ))
        }
    }

    pub fn set_active(&mut self, active: bool) -> Result<(), Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_set_annot_active(
                context(),
                self.inner.as_ptr(),
                c_int::from(active)
            ))
        }
    }

    pub fn opacity(&self) -> Result<f32, Error> {
        self.ensure_attached()?;
        unsafe { ffi_try!(mupdf_pdf_annot_opacity(context(), self.inner.as_ptr())) }
    }

    pub fn set_opacity(&mut self, opacity: f32) -> Result<(), Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_set_annot_opacity(
                context(),
                self.inner.as_ptr(),
                opacity
            ))
        }
    }

    pub fn border_width(&self) -> Result<f32, Error> {
        self.ensure_attached()?;
        unsafe { ffi_try!(mupdf_pdf_annot_border_width(context(), self.inner.as_ptr())) }
    }

    pub fn set_border_width(&mut self, width: f32) -> Result<(), Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_set_annot_border_width(
                context(),
                self.inner.as_ptr(),
                width
            ))
        }
    }

    pub fn border_style(&self) -> Result<AnnotationBorderStyle, Error> {
        self.ensure_attached()?;
        unsafe { ffi_try!(mupdf_pdf_annot_border_style(context(), self.inner.as_ptr())) }
            .and_then(AnnotationBorderStyle::try_from)
    }

    pub fn set_border_style(&mut self, style: AnnotationBorderStyle) -> Result<(), Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_set_annot_border_style(
                context(),
                self.inner.as_ptr(),
                style.into()
            ))
        }
    }

    pub fn border_dash_pattern(&self) -> Result<Vec<f32>, Error> {
        self.ensure_attached()?;
        let count = unsafe {
            ffi_try!(mupdf_pdf_annot_border_dash_count(
                context(),
                self.inner.as_ptr()
            ))
        }?;
        (0..count)
            .map(|i| unsafe {
                ffi_try!(mupdf_pdf_annot_border_dash_item(
                    context(),
                    self.inner.as_ptr(),
                    i
                ))
            })
            .collect()
    }

    pub fn clear_border_dash_pattern(&mut self) -> Result<(), Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_clear_annot_border_dash(
                context(),
                self.inner.as_ptr()
            ))
        }
    }

    pub fn add_border_dash_item(&mut self, length: f32) -> Result<(), Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_add_annot_border_dash_item(
                context(),
                self.inner.as_ptr(),
                length
            ))
        }
    }

    pub fn set_border_dash_pattern(&mut self, lengths: &[f32]) -> Result<(), Error> {
        self.ensure_attached()?;
        self.clear_border_dash_pattern()?;
        for length in lengths {
            self.add_border_dash_item(*length)?;
        }
        Ok(())
    }

    pub fn border_effect(&self) -> Result<AnnotationBorderEffect, Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_annot_border_effect(
                context(),
                self.inner.as_ptr()
            ))
        }
        .and_then(AnnotationBorderEffect::try_from)
    }

    pub fn set_border_effect(&mut self, effect: AnnotationBorderEffect) -> Result<(), Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_set_annot_border_effect(
                context(),
                self.inner.as_ptr(),
                effect.into()
            ))
        }
    }

    pub fn border_effect_intensity(&self) -> Result<f32, Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_annot_border_effect_intensity(
                context(),
                self.inner.as_ptr()
            ))
        }
    }

    pub fn set_border_effect_intensity(&mut self, intensity: f32) -> Result<(), Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_set_annot_border_effect_intensity(
                context(),
                self.inner.as_ptr(),
                intensity
            ))
        }
    }

    pub fn quadding(&self) -> Result<AnnotationTextAlign, Error> {
        self.ensure_attached()?;
        unsafe { ffi_try!(mupdf_pdf_annot_quadding(context(), self.inner.as_ptr())) }
            .and_then(AnnotationTextAlign::try_from)
    }

    pub fn set_quadding(&mut self, align: AnnotationTextAlign) -> Result<(), Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_set_annot_quadding(
                context(),
                self.inner.as_ptr(),
                align.into()
            ))
        }
    }

    pub fn vertices(&self) -> Result<Vec<Point>, Error> {
        self.ensure_attached()?;
        let count =
            unsafe { ffi_try!(mupdf_pdf_annot_vertex_count(context(), self.inner.as_ptr())) }?;
        (0..count)
            .map(|i| {
                unsafe { ffi_try!(mupdf_pdf_annot_vertex(context(), self.inner.as_ptr(), i)) }
                    .map(Into::into)
            })
            .collect()
    }

    pub fn set_vertices(&mut self, points: impl IntoIterator<Item = Point>) -> Result<(), Error> {
        self.ensure_attached()?;
        let raw: Vec<fz_point> = points.into_iter().map(Into::into).collect();
        unsafe {
            ffi_try!(mupdf_pdf_set_annot_vertices(
                context(),
                self.inner.as_ptr(),
                raw.len().try_into()?,
                raw.as_ptr()
            ))
        }
    }

    pub fn clear_vertices(&mut self) -> Result<(), Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_clear_annot_vertices(
                context(),
                self.inner.as_ptr()
            ))
        }
    }

    pub fn add_vertex(&mut self, point: Point) -> Result<(), Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_add_annot_vertex(
                context(),
                self.inner.as_ptr(),
                point.into()
            ))
        }
    }

    pub fn set_vertex(&mut self, index: i32, point: Point) -> Result<(), Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_set_annot_vertex(
                context(),
                self.inner.as_ptr(),
                index,
                point.into()
            ))
        }
    }

    pub fn ink_list(&self) -> Result<Vec<Vec<Point>>, Error> {
        self.ensure_attached()?;
        let stroke_count = unsafe {
            ffi_try!(mupdf_pdf_annot_ink_list_count(
                context(),
                self.inner.as_ptr()
            ))
        }?;
        let mut strokes = Vec::with_capacity(stroke_count as usize);
        for i in 0..stroke_count {
            let vertex_count = unsafe {
                ffi_try!(mupdf_pdf_annot_ink_list_stroke_count(
                    context(),
                    self.inner.as_ptr(),
                    i
                ))
            }?;
            let mut stroke = Vec::with_capacity(vertex_count as usize);
            for k in 0..vertex_count {
                let point = unsafe {
                    ffi_try!(mupdf_pdf_annot_ink_list_stroke_vertex(
                        context(),
                        self.inner.as_ptr(),
                        i,
                        k
                    ))
                }?;
                stroke.push(point.into());
            }
            strokes.push(stroke);
        }
        Ok(strokes)
    }

    pub fn set_ink_list<I, S>(&mut self, strokes: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = S>,
        S: IntoIterator<Item = Point>,
    {
        self.ensure_attached()?;
        let mut counts = Vec::new();
        let mut points = Vec::new();
        for stroke in strokes {
            let stroke_points: Vec<Point> = stroke.into_iter().collect();
            if stroke_points.is_empty() {
                return Err(Error::InvalidArgument(
                    "ink annotation strokes must not be empty".to_owned(),
                ));
            }
            counts.push(c_int::try_from(stroke_points.len())?);
            points.extend(stroke_points.into_iter().map(fz_point::from));
        }
        if counts.is_empty() {
            return self.clear_ink_list();
        }
        unsafe {
            ffi_try!(mupdf_pdf_set_annot_ink_list(
                context(),
                self.inner.as_ptr(),
                counts.len().try_into()?,
                counts.as_ptr(),
                points.as_ptr()
            ))
        }
    }

    pub fn clear_ink_list(&mut self) -> Result<(), Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_clear_annot_ink_list(
                context(),
                self.inner.as_ptr()
            ))
        }
    }

    pub fn add_ink_list_stroke(&mut self) -> Result<(), Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_add_annot_ink_list_stroke(
                context(),
                self.inner.as_ptr()
            ))
        }
    }

    pub fn add_ink_list_stroke_vertex(&mut self, point: Point) -> Result<(), Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_add_annot_ink_list_stroke_vertex(
                context(),
                self.inner.as_ptr(),
                point.into()
            ))
        }
    }

    pub fn icon_name(&self) -> Result<Option<&str>, Error> {
        self.ensure_attached()?;
        let ptr = unsafe { ffi_try!(mupdf_pdf_annot_icon_name(context(), self.inner.as_ptr())) }?;
        if ptr.is_null() {
            return Ok(None);
        }
        let c_str = unsafe { CStr::from_ptr(ptr) };
        Ok(Some(c_str.to_str().map_err(|_| Error::InvalidUtf8)?))
    }

    pub fn set_icon_name(&mut self, name: &str) -> Result<(), Error> {
        self.ensure_attached()?;
        let c_name = CString::new(name)?;
        unsafe {
            ffi_try!(mupdf_pdf_set_annot_icon_name(
                context(),
                self.inner.as_ptr(),
                c_name.as_ptr()
            ))
        }
    }

    pub fn is_open(&self) -> Result<bool, Error> {
        self.ensure_attached()?;
        unsafe { ffi_try!(mupdf_pdf_annot_is_open(context(), self.inner.as_ptr())) }
            .map(|is_open| is_open != 0)
    }

    pub fn set_open(&mut self, is_open: bool) -> Result<(), Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_set_annot_is_open(
                context(),
                self.inner.as_ptr(),
                c_int::from(is_open)
            ))
        }
    }

    pub fn default_appearance(&self) -> Result<Option<AnnotationDefaultAppearance>, Error> {
        self.ensure_attached()?;
        let has_default_appearance = unsafe {
            ffi_try!(mupdf_pdf_annot_has_default_appearance(
                context(),
                self.inner.as_ptr()
            ))
        }? != 0;
        if !has_default_appearance {
            return Ok(None);
        }

        let mut font_name = vec![0 as c_char; 256];
        let mut size = 0.0;
        let mut n = 0;
        let mut color = [0.0; 4];
        unsafe {
            ffi_try!(mupdf_pdf_annot_default_appearance_unmapped(
                context(),
                self.inner.as_ptr(),
                font_name.as_mut_ptr(),
                font_name.len().try_into()?,
                &raw mut size,
                &raw mut n,
                color.as_mut_ptr()
            ))
        }?;
        let font_name = unsafe { CStr::from_ptr(font_name.as_ptr()) }
            .to_str()
            .map_err(|_| Error::InvalidUtf8)?
            .to_owned();
        Ok(Some(AnnotationDefaultAppearance {
            font_name,
            size,
            color: annotation_color_from_components(n, color)?,
        }))
    }

    pub fn set_default_appearance(
        &mut self,
        font_name: &str,
        size: f32,
        color: Option<AnnotationColor>,
    ) -> Result<(), Error> {
        self.ensure_attached()?;
        let c_font_name = CString::new(font_name)?;
        let (n, components) = color
            .map(annotation_color_components)
            .unwrap_or((0, [0.0; 4]));
        unsafe {
            ffi_try!(mupdf_pdf_set_annot_default_appearance(
                context(),
                self.inner.as_ptr(),
                c_font_name.as_ptr(),
                size,
                n,
                components.as_ptr()
            ))
        }
    }

    pub fn set_intent(&mut self, intent: Intent) -> Result<(), Error> {
        self.ensure_attached()?;
        unsafe {
            ffi_try!(mupdf_pdf_set_annot_intent(
                context(),
                self.inner.as_ptr(),
                pdf_intent::from(intent)
            ))
        }
    }
}

pub(crate) fn bounding_rect_for_quads(quads: &[Quad]) -> Option<Rect> {
    let mut iter = quads.iter().cloned().map(Rect::from);
    let first = iter.next()?;
    Some(iter.fold(first, |acc, rect| acc.r#union(&rect)))
}

fn annotation_rect_from_dict(object: &PdfObject) -> Result<Option<Rect>, Error> {
    let Some(rect) = object.get_dict("Rect")? else {
        return Ok(None);
    };
    if !rect.is_array()? || rect.len()? < 4 {
        return Ok(None);
    }

    let x0 = rect
        .get_array(0)?
        .ok_or(Error::UnexpectedNullPtr)?
        .as_float()?;
    let y0 = rect
        .get_array(1)?
        .ok_or(Error::UnexpectedNullPtr)?
        .as_float()?;
    let x1 = rect
        .get_array(2)?
        .ok_or(Error::UnexpectedNullPtr)?
        .as_float()?;
    let y1 = rect
        .get_array(3)?
        .ok_or(Error::UnexpectedNullPtr)?
        .as_float()?;

    Ok(Some(Rect::new(x0, y0, x1, y1)))
}

fn raw_quads(quads: &[Quad]) -> Vec<fz_quad> {
    quads.iter().cloned().map(Into::into).collect()
}

fn annotation_color_components(color: AnnotationColor) -> (c_int, [f32; 4]) {
    match color {
        AnnotationColor::Gray(g) => (1, [g, 0.0, 0.0, 0.0]),
        AnnotationColor::Rgb { red, green, blue } => (3, [red, green, blue, 0.0]),
        AnnotationColor::Cmyk {
            cyan,
            magenta,
            yellow,
            key,
        } => (4, [cyan, magenta, yellow, key]),
    }
}

fn annotation_color_from_components(
    n: c_int,
    color: [f32; 4],
) -> Result<Option<AnnotationColor>, Error> {
    match n {
        0 => Ok(None),
        1 => Ok(Some(AnnotationColor::Gray(color[0]))),
        3 => Ok(Some(AnnotationColor::Rgb {
            red: color[0],
            green: color[1],
            blue: color[2],
        })),
        4 => Ok(Some(AnnotationColor::Cmyk {
            cyan: color[0],
            magenta: color[1],
            yellow: color[2],
            key: color[3],
        })),
        _ => Err(Error::UnknownEnumVariant),
    }
}

impl Drop for PdfAnnotation {
    fn drop(&mut self) {
        unsafe {
            pdf_drop_annot(context(), self.inner.as_ptr());
            pdf_drop_page(context(), self.page.as_ptr());
        }
    }
}

bitflags::bitflags! {
    pub struct AnnotationFlags: i32 {
        const IS_INVISIBLE = PDF_ANNOT_IS_INVISIBLE as _;
        const IS_HIDDEN = PDF_ANNOT_IS_HIDDEN as _;
        const IS_PRINT = PDF_ANNOT_IS_PRINT as _;
        const NO_ZOOM = PDF_ANNOT_IS_NO_ZOOM as _;
        const NO_ROTATE = PDF_ANNOT_IS_NO_ROTATE as _;
        const NO_VIEW = PDF_ANNOT_IS_NO_VIEW as _;
        const IS_READ_ONLY = PDF_ANNOT_IS_READ_ONLY as _;
        const IS_LOCKED = PDF_ANNOT_IS_LOCKED as _;
        const IS_TOGGLE_NO_VIEW = PDF_ANNOT_IS_TOGGLE_NO_VIEW as _;
        const IS_LOCKED_CONTENTS = PDF_ANNOT_IS_LOCKED_CONTENTS as _;
    }
}

#[cfg(test)]
mod test {
    use super::PdfAnnotationType;
    use crate::pdf::PdfDocument;
    use crate::{Rect, Size};

    #[test]
    fn test_annotation_rect() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut annot = page.create_annotation(PdfAnnotationType::Text).unwrap();

        let expected = Rect::new(10.0, 20.0, 110.0, 120.0);
        annot.set_rect(expected).unwrap();

        assert_eq!(annot.rect().unwrap(), expected);
    }
}

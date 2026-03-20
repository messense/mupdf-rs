use std::ops::{Deref, DerefMut};

use super::{
    parse_link_action_from_annot_dict, set_link_action_on_annot_dict, DestPageResolver, LinkAction,
    PdfLink, SingleResolver,
};
use crate::pdf::{DocOperation, PdfDocument, PdfObject};
use crate::{Error, Matrix, Rect};

/// A link annotation dictionary in a PDF document, identified by `Subtype=Link`.
///
/// Yielded by [`PdfLinkAnnotIter`](crate::pdf::page::PdfLinkAnnotIter), which is
/// returned by [`PdfPage::link_annotations`](crate::pdf::PdfPage::link_annotations). Items are
/// produced by iterating the page's `/Annots` array and filtering for link annotations.
///
/// # Why not `PdfAnnotation`?
///
/// MuPDF's `pdf_sync_annots` ([`PdfPage::annotations`](crate::pdf::PdfPage::annotations))
/// skips `Subtype=Link` entries, so [`PdfAnnotation`](crate::pdf::PdfAnnotation) of type
/// [`PdfAnnotationType::Link`](crate::pdf::PdfAnnotationType::Link) can never be produced
/// from a page via the annotation iterator. This type bypasses that by working directly
/// with the annotation dictionary object retrieved from the page's `/Annots` array.
///
/// Implements [`Deref<Target = PdfObject>`] for access to raw dictionary operations.
#[derive(Debug)]
pub struct PdfLinkAnnot {
    inner: PdfObject,
}

impl PdfLinkAnnot {
    pub(crate) fn new(obj: PdfObject) -> Self {
        Self { inner: obj }
    }

    /// Reads the link action from this annotation's dictionary, preserving the `/Dest` vs
    /// `/A` distinction and named destinations as-is.
    ///
    /// Returns `Ok(None)` if the annotation has no recognizable action entry.
    ///
    /// `page_no` is the 0-based page number where this annotation resides, used to resolve
    /// relative `Named` actions (`PrevPage`, `NextPage`). Pass `None` if the page number is
    /// unknown. Absolute named actions (`FirstPage`, `LastPage`) are always resolved.
    ///
    /// Unlike [`PdfPage::resolved_links`](crate::pdf::PdfPage::resolved_links), this method:
    /// - Does not resolve named destinations to concrete page numbers
    /// - Preserves `Launch` actions exactly as specified in the PDF document
    /// - Preserves whether the original entry was `/Dest` or `/A`
    /// - Does not clamp coordinates to the page bounds
    pub fn action(
        &self,
        doc: &PdfDocument,
        page_no: Option<i32>,
    ) -> Result<Option<LinkAction>, Error> {
        parse_link_action_from_annot_dict(&self.inner, doc, page_no)
    }

    /// Reads the annotation's `/Rect` entry and transforms it from PDF user space to Fitz
    /// coordinate space using `page_ctm`.
    ///
    /// `page_ctm` should be the page's Current Transformation Matrix obtained from
    /// [`PdfPage::ctm`](crate::pdf::PdfPage::ctm) or
    /// [`PdfObject::page_ctm`](crate::pdf::PdfObject::page_ctm).
    /// Pass `None` to skip the transformation and return raw PDF coordinates.
    pub fn rect(&self, page_ctm: Option<&Matrix>) -> Result<Rect, Error> {
        let rect_arr = self.inner.get_dict("Rect")?.ok_or_else(|| {
            Error::InvalidDestination("link annotation missing /Rect entry".into())
        })?;
        let x0 = rect_arr
            .get_array(0)?
            .ok_or_else(|| Error::InvalidDestination("/Rect[0] missing".into()))?
            .as_float()?;
        let y0 = rect_arr
            .get_array(1)?
            .ok_or_else(|| Error::InvalidDestination("/Rect[1] missing".into()))?
            .as_float()?;
        let x1 = rect_arr
            .get_array(2)?
            .ok_or_else(|| Error::InvalidDestination("/Rect[2] missing".into()))?
            .as_float()?;
        let y1 = rect_arr
            .get_array(3)?
            .ok_or_else(|| Error::InvalidDestination("/Rect[3] missing".into()))?
            .as_float()?;
        let pdf_rect = Rect::new(x0, y0, x1, y1);
        Ok(page_ctm
            .map(|ctm| pdf_rect.transform(ctm))
            .unwrap_or(pdf_rect))
    }

    /// Replaces the link action on this annotation dictionary.
    ///
    /// - [`LinkAction::Dest`] — writes a `/Dest` entry and removes `/A`.
    /// - [`LinkAction::Action`] — writes an `/A` dictionary and removes `/Dest`.
    ///
    /// `GoTo` destination coordinates are transformed from Fitz space to PDF user space
    /// using each destination page's own CTM (obtained on-demand via a fresh lookup).
    ///
    /// For bulk updates targeting many pages, prefer [`set_action_with_resolver`](Self::set_action_with_resolver)
    /// with a [`CachedResolver`](super::CachedResolver) to avoid redundant page lookups.
    pub fn set_action(&mut self, doc: &mut PdfDocument, action: &LinkAction) -> Result<(), Error> {
        let mut resolver =
            SingleResolver::new(|page_obj: &PdfObject| Ok(page_obj.page_ctm()?.invert()));
        self.set_action_with_resolver(doc, action, &mut resolver)
    }

    /// Like [`set_action`](Self::set_action) but with a caller-provided [`DestPageResolver`]
    /// for `GoTo` destination coordinate transformation.
    ///
    /// Use this when updating many annotations with destinations on shared pages — pass a
    /// [`CachedResolver`](super::CachedResolver) to avoid per-annotation page lookups.
    pub fn set_action_with_resolver(
        &mut self,
        doc: &mut PdfDocument,
        action: &LinkAction,
        resolver: &mut impl DestPageResolver,
    ) -> Result<(), Error> {
        let operation = DocOperation::begin(doc, "Set link action")?;

        let _ = self.inner.dict_delete("AA");
        match action {
            LinkAction::Action(_) => {
                let _ = self.inner.dict_delete("Dest");
            }
            LinkAction::Dest(_) => {
                let _ = self.inner.dict_delete("A");
            }
        }

        set_link_action_on_annot_dict(operation.doc, &mut self.inner, action, resolver)?;

        operation.commit()
    }

    /// Writes `rect` (in Fitz coordinate space) as the annotation's `/Rect` entry.
    ///
    /// `annot_inv_ctm` is the inverse of the page's CTM (see
    /// [`Matrix::invert`](crate::Matrix::invert) on
    /// [`PdfPage::ctm`](crate::pdf::PdfPage::ctm)).
    /// Pass `None` to write coordinates as-is without transformation.
    pub fn set_rect(
        &mut self,
        doc: &mut PdfDocument,
        rect: Rect,
        annot_inv_ctm: Option<&Matrix>,
    ) -> Result<(), Error> {
        let operation = DocOperation::begin(doc, "Set link rectangle")?;

        let pdf_rect = annot_inv_ctm
            .map(|inv_ctm| rect.transform(inv_ctm))
            .unwrap_or(rect);
        let mut rect_array = operation.doc.new_array_with_capacity(4)?;
        pdf_rect.encode_into(&mut rect_array)?;
        self.inner.dict_put("Rect", rect_array)?;

        operation.commit()
    }

    /// Combines [`rect`](Self::rect) and [`action`](Self::action) into a [`PdfLink`].
    ///
    /// Returns `Ok(None)` if the annotation has no recognizable action.
    pub fn to_pdf_link(
        &self,
        doc: &PdfDocument,
        page_no: Option<i32>,
        page_ctm: Option<&Matrix>,
    ) -> Result<Option<PdfLink>, Error> {
        let Some(action) = self.action(doc, page_no)? else {
            return Ok(None);
        };
        let bounds = self.rect(page_ctm)?;
        Ok(Some(PdfLink { bounds, action }))
    }

    /// Consumes this wrapper and returns the underlying [`PdfObject`].
    pub fn into_inner(self) -> PdfObject {
        self.inner
    }
}

impl Deref for PdfLinkAnnot {
    type Target = PdfObject;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for PdfLinkAnnot {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

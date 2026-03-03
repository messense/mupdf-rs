use std::collections::HashMap;
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
use crate::{context, unsafe_impl_ffi_wrapper, Error, FFIWrapper, Matrix, Page, Rect};

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
        .map(|annot| unsafe { PdfAnnotation::from_raw(annot) })
    }

    pub fn delete_annotation(&mut self, annot: &PdfAnnotation) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_delete_annot(
                context(),
                self.as_mut_ptr(),
                annot.inner
            ))
        }
    }

    pub fn annotations(&self) -> AnnotationIter {
        let next = unsafe { pdf_first_annot(context(), self.as_ptr().cast_mut()) };
        AnnotationIter { next }
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
pub struct AnnotationIter {
    next: *mut pdf_annot,
}

impl Iterator for AnnotationIter {
    type Item = PdfAnnotation;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next.is_null() {
            return None;
        }
        let node = self.next;
        unsafe {
            self.next = pdf_next_annot(context(), node);
            Some(PdfAnnotation::from_raw(node))
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
    use crate::pdf::{PdfAnnotation, PdfDocument, PdfPage};
    use crate::{Matrix, Rect};

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
}

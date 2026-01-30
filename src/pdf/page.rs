use std::collections::HashMap;
use std::{
    ffi::CStr,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use mupdf_sys::*;

use crate::link::LinkDestination;
use crate::pdf::links::{build_link_annotation, parse_external_link};
use crate::pdf::{
    PdfAction, PdfAnnotation, PdfAnnotationType, PdfDestination, PdfDocument, PdfFilterOptions,
    PdfLink, PdfObject,
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
    pub fn pdf_links(&self) -> Result<PdfLinkIter, Error> {
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

    /// Adds link annotations to this page, using caller-provided inverse CTM functions
    /// for coordinate transformation.
    ///
    /// Each [`PdfLink`] is converted into a PDF link annotation dictionary and
    /// appended to the page's `/Annots` array.
    ///
    /// # Coordinate transforms
    ///
    /// [`PdfLink`] coordinates are in Fitz space. PDF annotations need PDF default
    /// user space. Two callbacks provide the inverse CTM for each context:
    ///
    /// - `annot_inv_ctm(page_obj)` — for the annotation `/Rect` on *this* page.
    /// - `dest_inv_ctm(page_obj)` — for `GoTo(Page { .. })` destination coordinates
    ///   on each *destination* page. `GoToR` coordinates are written as-is.
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
    /// # Panics
    ///
    /// Panics if `self` does not belong to `doc` (ownership mismatch).
    pub fn add_links_with_inv_ctm(
        &mut self,
        doc: &mut PdfDocument,
        links: &[PdfLink],
        annot_inv_ctm: impl FnOnce(&PdfObject) -> Result<Option<Matrix>, Error>,
        mut dest_inv_ctm: impl FnMut(&PdfObject) -> Result<Option<Matrix>, Error>,
    ) -> Result<(), Error> {
        if links.is_empty() {
            return Ok(());
        }

        assert_eq!(
            doc.as_raw(),
            unsafe { (*self.inner.as_ptr()).doc },
            "PdfPage ownership mismatch: the page is not attached to the provided PdfDocument"
        );

        let mut page_obj = self.object();
        let annot_inv_ctm = annot_inv_ctm(&page_obj)?;
        let mut annots = match page_obj.get_dict("Annots")? {
            Some(a) => a,
            None => doc.new_array()?,
        };

        let mut page_cache: HashMap<u32, (PdfObject, Option<Matrix>)> = HashMap::new();

        for link in links {
            let annot = build_link_annotation(
                doc,
                &page_obj,
                link,
                &annot_inv_ctm,
                &mut dest_inv_ctm,
                &mut page_cache,
            )?;
            let annot_indirect = doc.add_object(&annot)?;
            annots.array_push(annot_indirect)?;
        }

        if annots.len()? > 0 {
            page_obj.dict_put("Annots", annots)?;
        }

        Ok(())
    }

    /// Adds link annotations to this page using the page's own CTM for coordinate
    /// transformation.
    ///
    /// Convenience wrapper around [`add_links_with_inv_ctm`](Self::add_links_with_inv_ctm)
    /// that derives both inverse CTMs from `page_obj.page_ctm().invert()`. This is
    /// correct when all link coordinates (both annotation bounds and `GoTo` destinations)
    /// are in MuPDF's Fitz coordinate space.
    pub fn add_links(&mut self, doc: &mut PdfDocument, links: &[PdfLink]) -> Result<(), Error> {
        self.add_links_with_inv_ctm(
            doc,
            links,
            |page_obj| Ok(page_obj.page_ctm()?.invert()),
            |page_obj| Ok(page_obj.page_ctm()?.invert()),
        )
    }
}

/// Iterator over link annotations on a PDF page, yielding [`PdfLink`] items.
///
/// Created by [`PdfPage::pdf_links`]. Links with unresolvable named destinations
/// or empty URIs are silently skipped.
///
/// See [`PdfPage::pdf_links`] for the full output mapping table.
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
                        PdfAction::GoTo(PdfDestination::Page {
                            page: dest.loc.page_in_chapter,
                            kind: dest.kind,
                        })
                    }
                    Ok(None) => match parse_external_link(uri.to_string_lossy().as_ref()) {
                        Some(action) => match action {
                            PdfAction::GoTo(PdfDestination::Named(_)) => {
                                // `LinkDestination::from_uri` already attempted to resolve named destinations.
                                // Reaching this point means the destination remains unresolved, so ignore and skip.
                                continue;
                            }
                            action => action,
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

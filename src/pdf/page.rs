use std::{
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use mupdf_sys::*;

use crate::pdf::{PdfAnnotation, PdfAnnotationType, PdfFilterOptions, PdfObject};
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
    pub(crate) unsafe fn from_raw(ptr: NonNull<pdf_page>) -> Self {
        Self {
            inner: ptr,
            // This cast is safe because the first member of the `pdf_page` struct is a `fz_page`
            page: ManuallyDrop::new(Page::from_raw(ptr.cast())),
        }
    }

    pub fn create_annotation(
        &mut self,
        subtype: PdfAnnotationType,
    ) -> Result<PdfAnnotation, Error> {
        unsafe {
            let annot = ffi_try!(mupdf_pdf_create_annot(
                context(),
                self.as_mut_ptr(),
                subtype as i32
            ));
            Ok(PdfAnnotation::from_raw(annot))
        }
    }

    pub fn delete_annotation(&mut self, annot: &PdfAnnotation) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_delete_annot(
                context(),
                self.as_mut_ptr(),
                annot.inner
            ));
        }
        Ok(())
    }

    pub fn annotations(&self) -> AnnotationIter {
        let next = unsafe { pdf_first_annot(context(), self.as_ptr() as *mut _) };
        AnnotationIter { next }
    }

    pub fn update(&mut self) -> Result<bool, Error> {
        let ret = unsafe { ffi_try!(mupdf_pdf_update_page(context(), self.as_mut_ptr())) };
        Ok(ret)
    }

    pub fn redact(&mut self) -> Result<bool, Error> {
        let ret = unsafe { ffi_try!(mupdf_pdf_redact_page(context(), self.as_mut_ptr())) };
        Ok(ret)
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
            ));
        }
        Ok(())
    }

    pub fn media_box(&self) -> Result<Rect, Error> {
        let rect = unsafe { mupdf_pdf_page_media_box(context(), self.as_ptr() as *mut _) };
        Ok(rect.into())
    }

    pub fn crop_box(&self) -> Result<Rect, Error> {
        let bounds = self.bounds()?;
        let pos = unsafe { mupdf_pdf_page_crop_box_position(context(), self.as_ptr() as *mut _) };
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
            ));
        }
        Ok(())
    }

    pub fn ctm(&self) -> Result<Matrix, Error> {
        let m = unsafe { ffi_try!(mupdf_pdf_page_transform(context(), self.as_ptr() as *mut _)) };
        Ok(m.into())
    }

    pub fn filter(&mut self, mut opt: PdfFilterOptions) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_filter_page_contents(
                context(),
                self.as_mut_ptr(),
                &mut opt.inner as *mut _
            ))
        }

        Ok(())
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
        let pdf_page = unsafe { pdf_page_from_fz_page(context(), value.as_ptr() as *mut _) };
        NonNull::new(pdf_page)
            .ok_or(Error::UnexpectedNullPtr)
            .map(|inner| unsafe { PdfPage::from_raw(inner) })
    }
}

#[cfg(test)]
mod test {
    use crate::pdf::{PdfAnnotation, PdfDocument, PdfPage};
    use crate::{Matrix, Rect};

    #[test]
    fn test_page_properties() {
        let doc = PdfDocument::open("tests/files/dummy.pdf").unwrap();
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
        let doc = PdfDocument::open("tests/files/dummy.pdf").unwrap();
        let page0 = PdfPage::try_from(doc.load_page(0).unwrap()).unwrap();
        let annots: Vec<PdfAnnotation> = page0.annotations().collect();
        assert_eq!(annots.len(), 0);
    }
}

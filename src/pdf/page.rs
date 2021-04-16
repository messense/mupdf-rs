use std::ops::{Deref, DerefMut};

use mupdf_sys::*;

use crate::pdf::{PdfAnnotation, PdfAnnotationType, PdfFilterOptions, PdfObject};
use crate::{context, Error, Matrix, Page, Rect};

#[derive(Debug)]
pub struct PdfPage {
    pub(crate) inner: *mut pdf_page,
    page: Page,
}

impl PdfPage {
    pub(crate) unsafe fn from_raw(ptr: *mut pdf_page) -> Self {
        Self {
            inner: ptr,
            page: Page::from_raw(ptr as *mut fz_page),
        }
    }

    pub fn create_annotation(
        &mut self,
        subtype: PdfAnnotationType,
    ) -> Result<PdfAnnotation, Error> {
        unsafe {
            let annot = ffi_try!(mupdf_pdf_create_annot(
                context(),
                self.inner,
                subtype as i32
            ));
            Ok(PdfAnnotation::from_raw(annot))
        }
    }

    pub fn delete_annotation(&mut self, annot: &PdfAnnotation) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_delete_annot(context(), self.inner, annot.inner));
        }
        Ok(())
    }

    pub fn annotations(&self) -> AnnotationIter {
        let next = unsafe { pdf_first_annot(context(), self.inner) };
        AnnotationIter { next }
    }

    pub fn update(&mut self) -> Result<bool, Error> {
        let ret = unsafe { ffi_try!(mupdf_pdf_update_page(context(), self.inner)) };
        Ok(ret)
    }

    pub fn redact(&mut self) -> Result<bool, Error> {
        let ret = unsafe { ffi_try!(mupdf_pdf_redact_page(context(), self.inner)) };
        Ok(ret)
    }

    pub fn object(&self) -> PdfObject {
        unsafe { PdfObject::from_raw_keep_ref((*self.inner).obj) }
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
            ffi_try!(mupdf_pdf_page_set_rotation(context(), self.inner, rotate));
        }
        Ok(())
    }

    pub fn media_box(&self) -> Result<Rect, Error> {
        let rect = unsafe { mupdf_pdf_page_media_box(context(), self.inner) };
        Ok(rect.into())
    }

    pub fn crop_box(&self) -> Result<Rect, Error> {
        let bounds = self.bounds()?;
        let pos = unsafe { mupdf_pdf_page_crop_box_position(context(), self.inner) };
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
                self.inner,
                crop_box.into()
            ));
        }
        Ok(())
    }

    pub fn ctm(&self) -> Result<Matrix, Error> {
        let m = unsafe { ffi_try!(mupdf_pdf_page_transform(context(), self.inner)) };
        Ok(m.into())
    }

    pub fn filter(&mut self, opt: PdfFilterOptions) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_filter_page_contents(
                context(),
                self.inner,
                opt.inner
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

impl From<Page> for PdfPage {
    fn from(page: Page) -> Self {
        let ptr = page.inner;
        Self {
            inner: ptr as *mut pdf_page,
            page,
        }
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
            self.next = (*node).next;
            Some(PdfAnnotation::from_raw(node))
        }
    }
}

#[cfg(test)]
mod test {
    use crate::pdf::{PdfAnnotation, PdfDocument, PdfPage};
    use crate::{Matrix, Rect};

    #[test]
    fn test_page_properties() {
        let doc = PdfDocument::open("tests/files/dummy.pdf").unwrap();
        let mut page0 = PdfPage::from(doc.load_page(0).unwrap());

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
        let page0 = PdfPage::from(doc.load_page(0).unwrap());
        let annots: Vec<PdfAnnotation> = page0.annotations().collect();
        assert_eq!(annots.len(), 0);
    }
}

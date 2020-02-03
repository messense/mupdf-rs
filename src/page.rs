use std::io::Read;

use mupdf_sys::*;

use crate::{
    context, Buffer, Colorspace, Cookie, Device, DisplayList, Error, Matrix, Pixmap, Rect,
    TextPage, TextPageOptions,
};

#[derive(Debug)]
pub struct Page {
    pub(crate) inner: *mut fz_page,
}

impl Page {
    pub(crate) unsafe fn from_raw(raw: *mut fz_page) -> Self {
        Self { inner: raw }
    }

    pub fn bounds(&self) -> Result<Rect, Error> {
        let rect = unsafe { ffi_try!(mupdf_bound_page(context(), self.inner)) };
        Ok(rect.into())
    }

    pub fn to_pixmap(
        &self,
        ctm: &Matrix,
        cs: &Colorspace,
        alpha: f32,
        show_extras: bool,
    ) -> Result<Pixmap, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_page_to_pixmap(
                context(),
                self.inner,
                ctm.into(),
                cs.inner,
                alpha,
                show_extras
            ));
            Ok(Pixmap::from_raw(inner))
        }
    }

    pub fn to_svg(&self, ctm: &Matrix) -> Result<String, Error> {
        let mut buf = unsafe {
            let inner = ffi_try!(mupdf_page_to_svg(
                context(),
                self.inner,
                ctm.into(),
                ptr::null_mut()
            ));
            Buffer::from_raw(inner)
        };
        let mut svg = String::new();
        buf.read_to_string(&mut svg)?;
        Ok(svg)
    }

    pub fn to_svg_with_cookie(&self, ctm: &Matrix, cookie: &Cookie) -> Result<String, Error> {
        let mut buf = unsafe {
            let inner = ffi_try!(mupdf_page_to_svg(
                context(),
                self.inner,
                ctm.into(),
                cookie.inner
            ));
            Buffer::from_raw(inner)
        };
        let mut svg = String::new();
        buf.read_to_string(&mut svg)?;
        Ok(svg)
    }

    pub fn to_text_page(&self, opts: TextPageOptions) -> Result<TextPage, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_page_to_text_page(
                context(),
                self.inner,
                opts.bits() as _
            ));
            Ok(TextPage::from_raw(inner))
        }
    }

    pub fn to_display_list(&self, annotations: bool) -> Result<DisplayList, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_page_to_display_list(
                context(),
                self.inner,
                annotations
            ));
            Ok(DisplayList::from_raw(inner))
        }
    }

    pub fn run(&self, device: &Device, ctm: &Matrix) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_run_page(
                context(),
                self.inner,
                device.dev,
                ctm.into(),
                ptr::null_mut()
            ))
        }
        Ok(())
    }

    pub fn run_with_cookie(
        &self,
        device: &Device,
        ctm: &Matrix,
        cookie: &Cookie,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_run_page(
                context(),
                self.inner,
                device.dev,
                ctm.into(),
                cookie.inner
            ))
        }
        Ok(())
    }

    pub fn run_contents(&self, device: &Device, ctm: &Matrix) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_run_page_contents(
                context(),
                self.inner,
                device.dev,
                ctm.into(),
                ptr::null_mut()
            ))
        }
        Ok(())
    }

    pub fn run_contents_with_cookie(
        &self,
        device: &Device,
        ctm: &Matrix,
        cookie: &Cookie,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_run_page_contents(
                context(),
                self.inner,
                device.dev,
                ctm.into(),
                cookie.inner
            ))
        }
        Ok(())
    }

    pub fn run_annotations(&self, device: &Device, ctm: &Matrix) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_run_page_annots(
                context(),
                self.inner,
                device.dev,
                ctm.into(),
                ptr::null_mut()
            ))
        }
        Ok(())
    }

    pub fn run_annotations_with_cookie(
        &self,
        device: &Device,
        ctm: &Matrix,
        cookie: &Cookie,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_run_page_annots(
                context(),
                self.inner,
                device.dev,
                ctm.into(),
                cookie.inner
            ))
        }
        Ok(())
    }

    pub fn run_widgets(&self, device: &Device, ctm: &Matrix) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_run_page_widgets(
                context(),
                self.inner,
                device.dev,
                ctm.into(),
                ptr::null_mut()
            ))
        }
        Ok(())
    }

    pub fn run_widgets_with_cookie(
        &self,
        device: &Device,
        ctm: &Matrix,
        cookie: &Cookie,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_run_page_widgets(
                context(),
                self.inner,
                device.dev,
                ctm.into(),
                cookie.inner
            ))
        }
        Ok(())
    }

    pub fn to_html(&self) -> Result<String, Error> {
        let mut buf = unsafe {
            let inner = ffi_try!(mupdf_page_to_html(context(), self.inner));
            Buffer::from_raw(inner)
        };
        let mut out = String::new();
        buf.read_to_string(&mut out)?;
        Ok(out)
    }

    pub fn to_xhtml(&self) -> Result<String, Error> {
        let mut buf = unsafe {
            let inner = ffi_try!(mupdf_page_to_xhtml(context(), self.inner));
            Buffer::from_raw(inner)
        };
        let mut out = String::new();
        buf.read_to_string(&mut out)?;
        Ok(out)
    }

    pub fn to_xml(&self) -> Result<String, Error> {
        let mut buf = unsafe {
            let inner = ffi_try!(mupdf_page_to_xml(context(), self.inner));
            Buffer::from_raw(inner)
        };
        let mut out = String::new();
        buf.read_to_string(&mut out)?;
        Ok(out)
    }

    pub fn to_text(&self) -> Result<String, Error> {
        let mut buf = unsafe {
            let inner = ffi_try!(mupdf_page_to_text(context(), self.inner));
            Buffer::from_raw(inner)
        };
        let mut out = String::new();
        buf.read_to_string(&mut out)?;
        Ok(out)
    }
}

impl Drop for Page {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                fz_drop_page(context(), self.inner);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{Document, Matrix};

    #[test]
    fn test_page_to_svg() {
        let doc = Document::open("tests/files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let svg = page0.to_svg(&Matrix::IDENTITY).unwrap();
        assert!(!svg.is_empty());
    }

    #[test]
    fn test_page_to_html() {
        let doc = Document::open("tests/files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let html = page0.to_html().unwrap();
        assert!(!html.is_empty());
    }

    #[test]
    fn test_page_to_xhtml() {
        let doc = Document::open("tests/files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let xhtml = page0.to_xhtml().unwrap();
        assert!(!xhtml.is_empty());
    }

    #[test]
    fn test_page_to_xml() {
        let doc = Document::open("tests/files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let xml = page0.to_xml().unwrap();
        assert!(!xml.is_empty());
    }

    #[test]
    fn test_page_to_text() {
        let doc = Document::open("tests/files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let text = page0.to_text().unwrap();
        assert!(!text.is_empty());
    }

    #[test]
    fn test_page_to_display_list() {
        let doc = Document::open("tests/files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let _dl = page0.to_display_list(true).unwrap();
        let _dl = page0.to_display_list(false).unwrap();
    }

    #[test]
    fn test_page_to_text_page() {
        use crate::TextPageOptions;

        let doc = Document::open("tests/files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let _tp = page0
            .to_text_page(TextPageOptions::PRESERVE_IMAGES)
            .unwrap();
    }
}

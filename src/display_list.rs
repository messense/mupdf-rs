use std::ffi::CString;
use std::slice;

use mupdf_sys::*;

use crate::{
    context, Colorspace, Cookie, Device, Error, Image, Matrix, Pixmap, Quad, Rect, TextPage,
    TextPageOptions,
};

#[derive(Debug)]
pub struct DisplayList {
    pub(crate) inner: *mut fz_display_list,
}

impl DisplayList {
    pub(crate) unsafe fn from_raw(ptr: *mut fz_display_list) -> Self {
        Self { inner: ptr }
    }

    pub fn new(media_box: Rect) -> Result<Self, Error> {
        let inner = unsafe { ffi_try!(mupdf_new_display_list(context(), media_box.into())) };
        Ok(Self { inner })
    }

    pub fn bounds(&self) -> Rect {
        let rect = unsafe { fz_bound_display_list(context(), self.inner) };
        rect.into()
    }

    pub fn to_pixmap(&self, ctm: &Matrix, cs: &Colorspace, alpha: bool) -> Result<Pixmap, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_display_list_to_pixmap(
                context(),
                self.inner,
                ctm.into(),
                cs.inner,
                alpha
            ));
            Ok(Pixmap::from_raw(inner))
        }
    }

    pub fn to_text_page(&self, opts: TextPageOptions) -> Result<TextPage, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_display_list_to_text_page(
                context(),
                self.inner,
                opts.bits() as _
            ));
            Ok(TextPage::from_raw(inner))
        }
    }

    pub fn to_image(&self, width: f32, height: f32) -> Result<Image, Error> {
        Image::from_display_list(self, width, height)
    }

    pub fn run(&self, device: &Device, ctm: &Matrix, area: Rect) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_display_list_run(
                context(),
                self.inner,
                device.dev,
                ctm.into(),
                area.into(),
                ptr::null_mut()
            ));
        }
        Ok(())
    }

    pub fn run_with_cookie(
        &self,
        device: &Device,
        ctm: &Matrix,
        area: Rect,
        cookie: &Cookie,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_display_list_run(
                context(),
                self.inner,
                device.dev,
                ctm.into(),
                area.into(),
                cookie.inner
            ));
        }
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        unsafe { fz_display_list_is_empty(context(), self.inner) > 0 }
    }

    pub fn search(&self, needle: &str, hit_max: u32) -> Result<Vec<Quad>, Error> {
        struct Quads(*mut fz_quad);

        impl Drop for Quads {
            fn drop(&mut self) {
                if !self.0.is_null() {
                    unsafe { fz_free(context(), self.0 as _) };
                }
            }
        }

        let c_needle = CString::new(needle)?;
        let hit_max = if hit_max < 1 { 16 } else { hit_max };
        let mut hit_count = 0;
        unsafe {
            let quads = Quads(ffi_try!(mupdf_search_display_list(
                context(),
                self.inner,
                c_needle.as_ptr(),
                hit_max as _,
                &mut hit_count
            )));
            if hit_count == 0 {
                return Ok(Vec::new());
            }
            let items = slice::from_raw_parts(quads.0, hit_count as usize);
            Ok(items.iter().map(|quad| (*quad).into()).collect())
        }
    }
}

impl Drop for DisplayList {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                fz_drop_display_list(context(), self.inner);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::Document;

    #[test]
    fn test_display_list_search() {
        use crate::{Point, Quad};

        let doc = Document::open("tests/files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let list = page0.to_display_list(false).unwrap();
        let hits = list.search("Dummy", 1).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(
            hits,
            [Quad {
                ul: Point {
                    x: 56.8,
                    y: 69.32512
                },
                ur: Point {
                    x: 115.85405,
                    y: 69.32512
                },
                ll: Point {
                    x: 56.8,
                    y: 87.311844
                },
                lr: Point {
                    x: 115.85405,
                    y: 87.311844
                }
            }]
        );

        let hits = list.search("Not Found", 1).unwrap();
        assert_eq!(hits.len(), 0);
    }
}

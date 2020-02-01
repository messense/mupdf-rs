use mupdf_sys::*;

use crate::{context, ColorSpace, Device, Error, Matrix, Pixmap, Rect, TextPage, TextPageOptions};

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

    pub fn to_pixmap(&self, ctm: &Matrix, cs: &ColorSpace, alpha: bool) -> Result<Pixmap, Error> {
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

    pub fn run(&self, device: &Device, ctm: &Matrix, area: Rect) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_display_list_run(
                context(),
                self.inner,
                device.dev,
                ctm.into(),
                area.into()
            ));
        }
        Ok(())
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

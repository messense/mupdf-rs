use mupdf_sys::*;

use crate::{context, ColorSpace, Error, Matrix, Pixmap, Rect};

#[derive(Debug)]
pub struct DisplayList {
    pub(crate) inner: *mut fz_display_list,
}

impl DisplayList {
    pub fn new(media_box: Rect) -> Result<Self, Error> {
        let inner = unsafe { ffi_try!(mupdf_new_display_list(context(), media_box.into())) };
        Ok(Self { inner })
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

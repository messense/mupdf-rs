use mupdf_sys::*;

use crate::{context, ColorSpace, Error, Matrix, Pixmap, Rect};

#[derive(Debug)]
pub struct Page {
    inner: *mut fz_page,
}

impl Page {
    pub fn bounds(&self) -> Result<Rect, Error> {
        let rect = unsafe { ffi_try!(mupdf_bound_page(context(), self.inner)) };
        Ok(rect.into())
    }

    pub fn to_pixmap(
        &self,
        ctm: &Matrix,
        cs: &ColorSpace,
        alpha: f32,
        show_extras: bool,
    ) -> Result<Pixmap, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_page_to_pixmap(
                context(),
                self.inner,
                ctm.to_fz_matrix(),
                cs.inner,
                alpha,
                show_extras
            ));
            Ok(Pixmap::from_raw(inner))
        }
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

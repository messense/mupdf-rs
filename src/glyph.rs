use mupdf_sys::*;

use crate::{context, IRect};

#[derive(Debug)]
pub struct Glyph {
    pub(crate) inner: *mut fz_glyph,
}

impl Glyph {
    pub fn width(&self) -> i32 {
        unsafe { fz_glyph_width(context(), self.inner) }
    }

    pub fn height(&self) -> i32 {
        unsafe { fz_glyph_height(context(), self.inner) }
    }

    pub fn bounds(&self) -> IRect {
        let bbox = unsafe { fz_glyph_bbox(context(), self.inner) };
        bbox.into()
    }
}

impl Drop for Glyph {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                fz_drop_glyph(context(), self.inner);
            }
        }
    }
}

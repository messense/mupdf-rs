use std::convert::TryFrom;

use mupdf_sys::*;

use crate::{context, Error, IRect, Pixmap};

#[derive(Debug)]
pub struct Glyph {
    pub(crate) inner: *mut fz_glyph,
}

impl Glyph {
    pub fn from_pixmap(pixmap: &Pixmap) -> Result<Self, Error> {
        let inner = unsafe { ffi_try!(mupdf_new_glyph_from_pixmap(context(), pixmap.inner)) };
        Ok(Self { inner })
    }

    pub fn width(&self) -> i32 {
        unsafe { fz_glyph_width(context(), self.inner) }
    }

    pub fn height(&self) -> i32 {
        unsafe { fz_glyph_height(context(), self.inner) }
    }

    pub fn size(&self) -> usize {
        unsafe { (*self.inner).size }
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

impl TryFrom<Pixmap> for Glyph {
    type Error = Error;

    fn try_from(pixmap: Pixmap) -> Result<Self, Self::Error> {
        Self::from_pixmap(&pixmap)
    }
}

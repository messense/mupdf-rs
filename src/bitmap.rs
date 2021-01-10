use std::convert::TryFrom;
use std::slice;

use mupdf_sys::*;

use crate::{context, Error, Pixmap};

/// Bitmaps have 1 bit per component.
/// Only used for creating halftoned versions of contone buffers, and saving out.
/// Samples are stored msb first, akin to pbms.
#[derive(Debug)]
pub struct Bitmap {
    pub(crate) inner: *mut fz_bitmap,
}

impl Bitmap {
    pub fn from_pixmap(pixmap: &Pixmap) -> Result<Self, Error> {
        let inner = unsafe { ffi_try!(mupdf_new_bitmap_from_pixmap(context(), pixmap.inner)) };
        Ok(Self { inner })
    }

    /// Width of the region in pixels.
    pub fn width(&self) -> u32 {
        unsafe { (*self.inner).w as u32 }
    }

    /// Height of the region in pixels.
    pub fn height(&self) -> u32 {
        unsafe { (*self.inner).h as u32 }
    }

    pub fn stride(&self) -> i32 {
        unsafe { (*self.inner).stride }
    }

    pub fn n(&self) -> i32 {
        unsafe { (*self.inner).n }
    }

    /// Horizontal and vertical resolution in dpi (dots per inch).
    pub fn resolution(&self) -> (i32, i32) {
        unsafe {
            let x_res = (*self.inner).xres;
            let y_res = (*self.inner).yres;
            (x_res, y_res)
        }
    }

    pub fn samples(&self) -> &[u8] {
        let len = (self.width() * self.height()) as usize;
        unsafe { slice::from_raw_parts((*self.inner).samples, len) }
    }

    pub fn samples_mut(&mut self) -> &mut [u8] {
        let len = (self.width() * self.height()) as usize;
        unsafe { slice::from_raw_parts_mut((*self.inner).samples, len) }
    }
}

impl Drop for Bitmap {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                fz_drop_bitmap(context(), self.inner);
            }
        }
    }
}

impl TryFrom<Pixmap> for Bitmap {
    type Error = Error;

    fn try_from(pixmap: Pixmap) -> Result<Self, Self::Error> {
        Self::from_pixmap(&pixmap)
    }
}

#[cfg(test)]
mod test {
    use crate::{Bitmap, Colorspace, Pixmap};

    #[test]
    fn test_new_bitmap() {
        let cs = Colorspace::device_gray();
        let mut pixmap = Pixmap::new_with_w_h(&cs, 100, 100, false).expect("Pixmap::new_with_w_h");
        pixmap.clear().unwrap();
        let bitmap = Bitmap::from_pixmap(&pixmap).unwrap();
        assert_eq!(bitmap.width(), 100);
        assert_eq!(bitmap.n(), 1);
    }

    #[test]
    fn test_new_bitmap_error() {
        let cs = Colorspace::device_rgb();
        let mut pixmap = Pixmap::new_with_w_h(&cs, 100, 100, false).expect("Pixmap::new_with_w_h");
        pixmap.clear().unwrap();
        assert!(Bitmap::from_pixmap(&pixmap).is_err());
    }
}

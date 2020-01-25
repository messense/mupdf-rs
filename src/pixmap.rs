use std::ffi::CString;

use mupdf_sys::*;

use crate::{context, ColorSpace, Error, Rect};

#[derive(Debug)]
pub struct Pixmap {
    inner: *mut fz_pixmap,
}

impl Pixmap {
    pub fn new(
        cs: &ColorSpace,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        alpha: bool,
    ) -> Result<Self, Error> {
        let ctx = context();
        let inner = unsafe { ffi_try!(mupdf_new_pixmap(ctx, cs.inner, x, y, w, h, alpha)) };
        Ok(Self { inner })
    }

    pub fn new_with_rect(cs: &ColorSpace, rect: Rect, alpha: bool) -> Result<Self, Error> {
        let x = rect.x0 as i32;
        let y = rect.y0 as i32;
        let w = (rect.x1 - rect.x0) as i32;
        let h = (rect.y1 - rect.y0) as i32;
        Self::new(cs, x, y, w, h, alpha)
    }

    pub fn new_with_w_h(cs: &ColorSpace, w: i32, h: i32, alpha: bool) -> Result<Self, Error> {
        Self::new(cs, 0, 0, w, h, alpha)
    }

    pub fn x(&self) -> i32 {
        unsafe { (*self.inner).x }
    }

    pub fn y(&self) -> i32 {
        unsafe { (*self.inner).y }
    }

    pub fn width(&self) -> i32 {
        unsafe { (*self.inner).w }
    }

    pub fn height(&self) -> i32 {
        unsafe { (*self.inner).h }
    }

    pub fn stride(&self) -> isize {
        unsafe { (*self.inner).stride }
    }

    pub fn number_of_components(&self) -> usize {
        unsafe { usize::from((*self.inner).n) }
    }

    pub fn alpha(&self) -> bool {
        unsafe { (*self.inner).alpha > 0 }
    }

    pub fn color_space(&self) -> ColorSpace {
        unsafe { ColorSpace::from_raw((*self.inner).colorspace) }
    }

    pub fn resolution(&self) -> (i32, i32) {
        unsafe {
            let x_res = (*self.inner).xres;
            let y_res = (*self.inner).yres;
            (x_res, y_res)
        }
    }

    pub fn clear(&mut self) {
        unsafe {
            mupdf_clear_pixmap(context(), self.inner);
        }
    }

    pub fn clear_with_value(&mut self, value: i32) {
        unsafe {
            mupdf_clear_pixmap_with_value(context(), self.inner, value);
        }
    }

    pub fn save_as_png(&self, filename: &str) {
        let c_filename = CString::new(filename).unwrap();
        unsafe {
            mupdf_save_pixmap_as_png(context(), self.inner, c_filename.as_ptr());
        }
    }

    pub fn invert(&mut self) {
        unsafe {
            mupdf_invert_pixmap(context(), self.inner);
        }
    }

    pub fn gamma(&mut self, gamma: f32) {
        unsafe {
            mupdf_gamma_pixmap(context(), self.inner, gamma);
        }
    }
}

impl Drop for Pixmap {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe { fz_drop_pixmap(context(), self.inner) };
        }
    }
}

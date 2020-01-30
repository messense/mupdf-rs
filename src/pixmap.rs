use std::ffi::CString;

use mupdf_sys::*;

use crate::{context, ColorSpace, Error, Rect};

#[derive(Debug)]
pub struct Pixmap {
    pub(crate) inner: *mut fz_pixmap,
}

impl Pixmap {
    pub(crate) unsafe fn from_raw(pixmap: *mut fz_pixmap) -> Self {
        Self { inner: pixmap }
    }

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

    pub fn width(&self) -> u32 {
        unsafe { (*self.inner).w as u32 }
    }

    pub fn height(&self) -> u32 {
        unsafe { (*self.inner).h as u32 }
    }

    pub fn stride(&self) -> isize {
        unsafe { (*self.inner).stride }
    }

    pub fn number_of_components(&self) -> u8 {
        unsafe { (*self.inner).n }
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

    pub fn clear(&mut self) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_clear_pixmap(context(), self.inner));
        }
        Ok(())
    }

    pub fn clear_with(&mut self, value: i32) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_clear_pixmap_with_value(context(), self.inner, value));
        }
        Ok(())
    }

    pub fn save_as_png(&self, filename: &str) -> Result<(), Error> {
        let c_filename = CString::new(filename)?;
        unsafe {
            ffi_try!(mupdf_save_pixmap_as_png(
                context(),
                self.inner,
                c_filename.as_ptr()
            ));
        }
        Ok(())
    }

    pub fn invert(&mut self) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_invert_pixmap(context(), self.inner));
        }
        Ok(())
    }

    pub fn gamma(&mut self, gamma: f32) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_gamma_pixmap(context(), self.inner, gamma));
        }
        Ok(())
    }

    pub fn shrink(&mut self, factor: i32) -> Result<(), Error> {
        if factor >= 1 {
            unsafe {
                fz_subsample_pixmap(context(), self.inner, factor);
            }
        }
        Ok(())
    }
}

impl Drop for Pixmap {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe { fz_drop_pixmap(context(), self.inner) };
        }
    }
}

#[cfg(test)]
mod test {
    use super::{ColorSpace, Pixmap};

    #[test]
    fn test_pixmap_color_space() {
        let cs = ColorSpace::device_rgb();
        let pixmap = Pixmap::new_with_w_h(&cs, 100, 100, false).expect("Pixmap::new_with_w_h");
        let pixmap_cs = pixmap.color_space();
        assert_eq!(cs, pixmap_cs);
    }

    #[test]
    fn test_pixmap_clear() {
        let cs = ColorSpace::device_rgb();
        let mut pixmap = Pixmap::new_with_w_h(&cs, 100, 100, false).expect("Pixmap::new_with_w_h");
        pixmap.clear().unwrap();
        pixmap.clear_with(1).unwrap();
    }

    #[test]
    fn test_pixmap_resolution() {
        let cs = ColorSpace::device_rgb();
        let pixmap = Pixmap::new_with_w_h(&cs, 100, 100, false).expect("Pixmap::new_with_w_h");
        let resolution = pixmap.resolution();
        assert_eq!(resolution, (96, 96));
    }

    #[test]
    fn test_pixmap_invert() {
        let cs = ColorSpace::device_rgb();
        let mut pixmap = Pixmap::new_with_w_h(&cs, 100, 100, false).expect("Pixmap::new_with_w_h");
        pixmap.invert().unwrap();
    }

    #[test]
    fn test_pixmap_gamma() {
        let cs = ColorSpace::device_rgb();
        let mut pixmap = Pixmap::new_with_w_h(&cs, 100, 100, false).expect("Pixmap::new_with_w_h");
        pixmap.gamma(1.0).unwrap();
    }
}

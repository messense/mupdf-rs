use mupdf_sys::*;

use crate::{context, ColorSpace, Rect};

#[derive(Debug)]
pub struct Pixmap {
    inner: *mut fz_pixmap,
}

impl Pixmap {
    pub fn new(cs: &ColorSpace, x: i32, y: i32, w: i32, h: i32, alpha: bool) -> Self {
        let ctx = context();
        let inner = unsafe { mupdf_new_pixmap(ctx, cs.inner, x, y, w, h, alpha) };
        Self { inner }
    }

    pub fn new_with_rect(cs: &ColorSpace, rect: Rect, alpha: bool) -> Self {
        let x = rect.x0 as i32;
        let y = rect.y0 as i32;
        let w = (rect.x1 - rect.x0) as i32;
        let h = (rect.y1 - rect.y0) as i32;
        Self::new(cs, x, y, w, h, alpha)
    }

    pub fn new_with_w_h(cs: &ColorSpace, w: i32, h: i32, alpha: bool) -> Self {
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

    pub fn clear(&mut self) {
        unsafe {
            mupdf_clear_pixmap(context(), self.inner);
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

#[cfg(test)]
mod test {
    use super::{ColorSpace, Pixmap};

    #[test]
    fn test_pixmap_color_space() {
        let cs = ColorSpace::device_rgb();
        let pixmap = Pixmap::new_with_w_h(&cs, 100, 100, false);
        let pixmap_cs = pixmap.color_space();
        assert_eq!(cs, pixmap_cs);
    }
}

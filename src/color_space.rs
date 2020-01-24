use mupdf_sys::*;

use crate::Context;

#[derive(Debug)]
pub struct ColorSpace {
    inner: *mut fz_colorspace,
}

impl ColorSpace {
    pub fn device_gray() -> Self {
        let inner = unsafe { fz_device_gray(Context::get().inner) };
        Self { inner }
    }

    pub fn device_rgb() -> Self {
        let inner = unsafe { fz_device_rgb(Context::get().inner) };
        Self { inner }
    }

    pub fn device_bgr() -> Self {
        let inner = unsafe { fz_device_bgr(Context::get().inner) };
        Self { inner }
    }

    pub fn device_cmyk() -> Self {
        let inner = unsafe { fz_device_cmyk(Context::get().inner) };
        Self { inner }
    }
}

impl Drop for ColorSpace {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe { fz_drop_colorspace(Context::get().inner, self.inner) };
        }
    }
}

#[cfg(test)]
mod test {
    use super::ColorSpace;

    #[test]
    fn test_color_space_device_colors() {
        let _gray = ColorSpace::device_gray();
        let _rgb = ColorSpace::device_rgb();
        let _bgr = ColorSpace::device_bgr();
        let _cmyk = ColorSpace::device_cmyk();
    }
}

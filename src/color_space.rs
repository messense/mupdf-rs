use std::cmp::PartialEq;

use mupdf_sys::*;

use crate::context;

#[derive(Debug)]
pub struct ColorSpace {
    pub(crate) inner: *mut fz_colorspace,
}

impl ColorSpace {
    pub(crate) unsafe fn from_raw(inner: *mut fz_colorspace) -> Self {
        Self { inner }
    }

    pub fn device_gray() -> Self {
        let inner = unsafe { fz_device_gray(context()) };
        Self { inner }
    }

    pub fn device_rgb() -> Self {
        let inner = unsafe { fz_device_rgb(context()) };
        Self { inner }
    }

    pub fn device_bgr() -> Self {
        let inner = unsafe { fz_device_bgr(context()) };
        Self { inner }
    }

    pub fn device_cmyk() -> Self {
        let inner = unsafe { fz_device_cmyk(context()) };
        Self { inner }
    }

    pub fn num_of_components(&self) -> u32 {
        unsafe { fz_colorspace_n(context(), self.inner) as u32 }
    }

    pub fn is_gray(&self) -> bool {
        unsafe { fz_colorspace_is_gray(context(), self.inner) > 0 }
    }

    pub fn is_rgb(&self) -> bool {
        unsafe { fz_colorspace_is_rgb(context(), self.inner) > 0 }
    }

    pub fn is_cmyk(&self) -> bool {
        unsafe { fz_colorspace_is_cmyk(context(), self.inner) > 0 }
    }

    pub fn is_device(&self) -> bool {
        unsafe { fz_colorspace_is_device(context(), self.inner) > 0 }
    }

    pub fn is_device_gray(&self) -> bool {
        unsafe { fz_colorspace_is_device_gray(context(), self.inner) > 0 }
    }

    pub fn is_device_cmyk(&self) -> bool {
        unsafe { fz_colorspace_is_device_cmyk(context(), self.inner) > 0 }
    }

    pub fn is_indexed(&self) -> bool {
        unsafe { fz_colorspace_is_indexed(context(), self.inner) > 0 }
    }

    pub fn is_lab(&self) -> bool {
        unsafe { fz_colorspace_is_lab(context(), self.inner) > 0 }
    }

    pub fn is_lab_icc(&self) -> bool {
        unsafe { fz_colorspace_is_lab_icc(context(), self.inner) > 0 }
    }

    pub fn is_subtractive(&self) -> bool {
        unsafe { fz_colorspace_is_subtractive(context(), self.inner) > 0 }
    }
}

impl Drop for ColorSpace {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe { fz_drop_colorspace(context(), self.inner) };
        }
    }
}

impl PartialEq for ColorSpace {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

#[cfg(test)]
mod test {
    use super::ColorSpace;

    #[test]
    fn test_color_space_device_colors() {
        let gray = ColorSpace::device_gray();
        assert!(gray.is_device_gray());
        assert!(gray.is_gray());

        let rgb = ColorSpace::device_rgb();
        assert!(rgb.is_rgb());

        let _bgr = ColorSpace::device_bgr();
        let cmyk = ColorSpace::device_cmyk();
        assert!(cmyk.is_device_cmyk());
        assert!(cmyk.is_cmyk());
    }
}

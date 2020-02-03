use std::cmp::PartialEq;
use std::ffi::CStr;
use std::fmt;

use mupdf_sys::*;

use crate::context;

#[derive(Debug)]
pub struct Colorspace {
    pub(crate) inner: *mut fz_colorspace,
}

impl Colorspace {
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

    pub fn n(&self) -> u32 {
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

    pub fn name(&self) -> &str {
        let ptr = unsafe { fz_colorspace_name(context(), self.inner) };
        let name_cstr = unsafe { CStr::from_ptr(ptr) };
        name_cstr.to_str().unwrap()
    }
}

impl PartialEq for Colorspace {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl fmt::Display for Colorspace {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = self.name();
        if name.starts_with("Device") {
            name.fmt(f)
        } else {
            write!(f, "ColorSpace({})", self.n())
        }
    }
}

#[cfg(test)]
mod test {
    use super::Colorspace;

    #[test]
    fn test_color_space_device_colors() {
        let gray = Colorspace::device_gray();
        assert!(gray.is_device_gray());
        assert!(gray.is_gray());
        assert_eq!(gray.name(), "DeviceGray");

        let rgb = Colorspace::device_rgb();
        assert!(rgb.is_rgb());
        assert_eq!(rgb.name(), "DeviceRGB");

        let bgr = Colorspace::device_bgr();
        assert_eq!(bgr.name(), "DeviceBGR");

        let cmyk = Colorspace::device_cmyk();
        assert!(cmyk.is_device_cmyk());
        assert!(cmyk.is_cmyk());
        assert_eq!(cmyk.name(), "DeviceCMYK");
    }
}

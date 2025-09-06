use std::cmp::PartialEq;
use std::ffi::CStr;
use std::fmt;
use std::ptr;

use mupdf_sys::*;

use crate::{context, ColorParams, Error};

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

    /// `color` should contain a number of elements euql to [`self.n()`]. Each element within
    /// `color` should *probably* be a value between [0, 1.0] (at least, that is the case when
    /// [`Self::is_rgb()`], [`Self::is_gray()`], or [`Self::is_cmyk()`] - other colorspaces may
    /// work differently).
    ///
    /// The returned vector will have [`to.n()`] elements, and contain values that follow the same
    /// rules as the input `color` (but with regard to `to`, not `self`)
    ///
    /// [`self.n()`]: Self::n
    /// [`to.n()`]: Self::n
    pub fn convert_color(
        &self,
        color: &[f32],
        to: &Colorspace,
        via: Option<&Colorspace>,
        params: ColorParams,
    ) -> Result<Vec<f32>, Error> {
        let from_n = usize::try_from(self.n()).unwrap();
        assert!(color.len() >= from_n);
        let to_n = usize::try_from(to.n()).unwrap();

        let via = via.map_or(ptr::null_mut(), |cs| cs.inner);
        unsafe {
            let mut out = Vec::with_capacity(to_n);
            ffi_try!(mupdf_convert_color(
                context(),
                self.inner,
                color.as_ptr(),
                to.inner,
                out.as_mut_ptr(),
                via,
                params.into()
            ))?;
            out.set_len(to_n);
            Ok(out)
        }
    }

    pub fn convert_color_into(
        &self,
        color: &[f32],
        to: &Colorspace,
        out: &mut [f32],
        via: Option<&Colorspace>,
        params: ColorParams,
    ) -> Result<usize, Error> {
        let from_n = usize::try_from(self.n()).unwrap();
        assert!(color.len() >= from_n);
        let n = usize::try_from(to.n()).unwrap();
        assert!(out.len() >= n);

        let via = via.map_or(ptr::null_mut(), |cs| cs.inner);
        unsafe {
            ffi_try!(mupdf_convert_color(
                context(),
                self.inner,
                color.as_ptr(),
                to.inner,
                out.as_mut_ptr(),
                via,
                params.into()
            ))?;
            Ok(n)
        }
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
    use crate::ColorParams;

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

    #[test]
    fn test_color_conversion() {
        let red = Colorspace::device_rgb()
            .convert_color(
                &[1.0, 0.0, 0.0],
                &Colorspace::device_cmyk(),
                None,
                ColorParams::default(),
            )
            .unwrap();
        assert_eq!(red.len(), 4);
        assert_eq!(red[0], 0.0);
        assert!((0.99..=1.0).contains(&red[1]));
        assert_eq!(red[2], 1.0);
        assert_eq!(red[3], 0.0);

        let mut gray = [0.0; 4];
        let n = Colorspace::device_gray()
            .convert_color_into(
                &[0.6],
                &Colorspace::device_rgb(),
                &mut gray,
                None,
                ColorParams::default(),
            )
            .unwrap();
        assert_eq!(n, 3);
        assert!((0.59..0.61).contains(&gray[0]), "gray = {gray:?}");
        assert!((0.59..0.61).contains(&gray[1]), "gray = {gray:?}");
        assert!((0.59..0.61).contains(&gray[2]), "gray = {gray:?}");
        assert_eq!(gray[3], 0.0);
    }
}

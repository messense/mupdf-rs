use mupdf_sys::*;

use crate::Context;

#[derive(Debug)]
pub struct ColorSpace {
    inner: *mut fz_colorspace,
}

impl ColorSpace {
    pub fn device_gray(ctx: &Context) -> Self {
        let inner = unsafe { fz_device_gray(ctx.inner) };
        Self { inner }
    }

    pub fn device_rgb(ctx: &Context) -> Self {
        let inner = unsafe { fz_device_rgb(ctx.inner) };
        Self { inner }
    }

    pub fn device_bgr(ctx: &Context) -> Self {
        let inner = unsafe { fz_device_bgr(ctx.inner) };
        Self { inner }
    }

    pub fn device_cmyk(ctx: &Context) -> Self {
        let inner = unsafe { fz_device_cmyk(ctx.inner) };
        Self { inner }
    }
}

impl Drop for ColorSpace {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            // FIXME: get context
            // unsafe { fz_drop_colorspace(self.inner) };
        }
    }
}

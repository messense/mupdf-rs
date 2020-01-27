use mupdf_sys::*;

use crate::{context, Error, Pixmap};

#[derive(Debug)]
pub struct DrawDevice {
    pub(crate) inner: *mut fz_device,
}

impl DrawDevice {
    pub fn new(pixmap: &Pixmap) -> Result<Self, Error> {
        let inner = unsafe { ffi_try!(mupdf_new_draw_device(context(), pixmap.inner)) };
        Ok(Self { inner })
    }
}

impl Drop for DrawDevice {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                fz_drop_device(context(), self.inner);
            }
        }
    }
}

// TODO: impl Device for DrawDevice

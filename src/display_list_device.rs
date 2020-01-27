use mupdf_sys::*;

use crate::{context, DisplayList, Error};

#[derive(Debug)]
pub struct DisplayListDevice {
    pub(crate) inner: *mut fz_device,
}

impl DisplayListDevice {
    pub fn new(list: &DisplayList) -> Result<Self, Error> {
        let inner = unsafe { ffi_try!(mupdf_new_display_list_device(context(), list.inner)) };
        Ok(Self { inner })
    }
}

impl Drop for DisplayListDevice {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                fz_drop_device(context(), self.inner);
            }
        }
    }
}

// TODO: impl Device for DisplayListDevice

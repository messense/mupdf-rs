use mupdf_sys::*;

use crate::{context, Error};

#[derive(Debug)]
pub struct Path {
    inner: *mut fz_path,
}

impl Path {
    pub fn new() -> Result<Self, Error> {
        let inner = unsafe { ffi_try!(mupdf_new_path(context())) };
        Ok(Self { inner })
    }

    fn try_clone(&self) -> Result<Self, Error> {
        let inner = unsafe { ffi_try!(mupdf_clone_path(context(), self.inner)) };
        Ok(Self { inner })
    }
}

impl Drop for Path {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                fz_drop_path(context(), self.inner);
            }
        }
    }
}

impl Clone for Path {
    fn clone(&self) -> Self {
        self.try_clone().unwrap()
    }
}

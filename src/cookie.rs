use mupdf_sys::*;

use crate::{context, Error};

#[derive(Debug)]
pub struct Cookie {
    pub(crate) inner: *mut fz_cookie,
}

impl Cookie {
    pub fn new() -> Result<Self, Error> {
        let inner = unsafe { ffi_try!(mupdf_new_cookie(context())) };
        Ok(Self { inner })
    }

    pub fn abort(&mut self) {
        unsafe { (*self.inner).abort = 1; }
    }
}

impl Drop for Cookie {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe { fz_free(context(), self.inner); }
        }
    }
}
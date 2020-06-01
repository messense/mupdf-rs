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
        unsafe {
            (*self.inner).abort = 1;
        }
    }

    pub fn progress(&self) -> i32 {
        unsafe { (*self.inner).progress }
    }

    pub fn max_progress(&self) -> usize {
        unsafe { (*self.inner).progress_max }
    }

    pub fn errors(&self) -> i32 {
        unsafe { (*self.inner).errors }
    }

    pub fn incomplete(&self) -> bool {
        unsafe { (*self.inner).incomplete > 0 }
    }

    pub fn set_incomplete(&mut self, value: bool) {
        let val = if value { 1 } else { 0 };
        unsafe {
            (*self.inner).incomplete = val;
        }
    }
}

impl Drop for Cookie {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                fz_free(context(), self.inner as _);
            }
        }
    }
}

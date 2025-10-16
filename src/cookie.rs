use std::ffi::c_void;

use mupdf_sys::*;

use crate::{context, Error};

/// Provide two-way communication between application and library.
/// Intended for multi-threaded applications where one thread is rendering pages and
/// another thread wants to read progress feedback or abort a job that takes a long time to finish.
/// The communication is unsynchronized without locking.
#[derive(Debug)]
pub struct Cookie {
    pub(crate) inner: *mut fz_cookie,
}

impl Cookie {
    pub fn new() -> Result<Self, Error> {
        unsafe { ffi_try!(mupdf_new_cookie(context())) }.map(|inner| Self { inner })
    }

    /// Abort rendering
    pub fn abort(&mut self) {
        unsafe {
            (*self.inner).abort = 1;
        }
    }

    /// Communicates rendering progress back to the application.
    /// Increments as a page is being rendered.
    pub fn progress(&self) -> i32 {
        unsafe { (*self.inner).progress }
    }

    /// Communicates the known upper bound of rendering back to the application
    pub fn max_progress(&self) -> usize {
        unsafe { (*self.inner).progress_max }
    }

    /// count of errors during current rendering
    pub fn errors(&self) -> i32 {
        unsafe { (*self.inner).errors }
    }

    /// Initially should be set to 0.
    /// Will be set to non-zero if a TRYLATER error is thrown during rendering
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
            unsafe { fz_free(context(), self.inner.cast()) }
        }
    }
}

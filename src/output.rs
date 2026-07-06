use std::ptr::NonNull;

use mupdf_sys::{fz_drop_output, fz_output, mupdf_close_output, mupdf_new_output_with_buffer};

use crate::{context, Buffer, Error};

pub struct Output {
    pub(crate) inner: NonNull<fz_output>,
}

impl Drop for Output {
    fn drop(&mut self) {
        let ptr = self.as_ptr();

        // SAFETY: `ptr` is a valid output owned by this wrapper. MuPDF requires outputs to be
        // closed before they are dropped.
        let _ = unsafe { ffi_try!(mupdf_close_output(context(), ptr)) };

        // SAFETY: `ptr` remains owned by this wrapper after close and must be released exactly
        // once.
        unsafe { fz_drop_output(context(), ptr) };
    }
}

impl Output {
    pub(crate) fn as_ptr(&self) -> *mut fz_output {
        self.inner.as_ptr()
    }

    pub fn from_buffer(buf: &Buffer) -> Result<Self, Error> {
        unsafe { ffi_try!(mupdf_new_output_with_buffer(context(), buf.inner)) }
            .and_then(|ptr| NonNull::new(ptr).ok_or(Error::UnexpectedNullPtr))
            .map(|inner| Self { inner })
    }
}

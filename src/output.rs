use std::ptr::NonNull;

use mupdf_sys::{fz_close_output, fz_drop_output, fz_new_output_with_buffer, fz_output};

use crate::{context, Buffer};

pub struct Output {
    pub(crate) inner: NonNull<fz_output>,
}

impl Drop for Output {
    fn drop(&mut self) {
        let ptr = self.as_ptr();

        // SAFETY: `ptr` is a valid output owned by this wrapper. MuPDF requires outputs to be
        // closed before they are dropped.
        unsafe { fz_close_output(context(), ptr) };

        // SAFETY: `ptr` remains owned by this wrapper after close and must be released exactly
        // once.
        unsafe { fz_drop_output(context(), ptr) };
    }
}

impl Output {
    pub(crate) fn as_ptr(&self) -> *mut fz_output {
        self.inner.as_ptr()
    }

    pub fn from_buffer(buf: &Buffer) -> Self {
        // SAFETY: `buf.inner` is a valid MuPDF buffer owned by `buf`.
        let inner = unsafe { fz_new_output_with_buffer(context(), buf.inner) };
        // This API historically returned `Self`; panic rather than construct an invalid wrapper if
        // MuPDF unexpectedly returns null. A fallible constructor would be a future API addition.
        let inner = NonNull::new(inner).expect("fz_new_output_with_buffer returned null");

        Self { inner }
    }
}

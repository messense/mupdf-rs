use std::ptr::NonNull;

use mupdf_sys::{fz_close_output, fz_drop_output, fz_new_output_with_buffer, fz_output};

use crate::{context, Buffer};

pub struct Output {
    pub(crate) inner: NonNull<fz_output>,
}

impl Drop for Output {
    fn drop(&mut self) {
        unsafe {
            fz_close_output(context(), self.inner.as_ptr());
            fz_drop_output(context(), self.inner.as_ptr());
        }
    }
}

impl Output {
    pub fn from_buffer(buf: &Buffer) -> Self {
        let inner = unsafe { fz_new_output_with_buffer(context(), buf.inner) };

        // SAFETY: fz_new_output_with_buffer never returns NULL
        let inner = unsafe { NonNull::new_unchecked(inner) };

        Self { inner }
    }
}

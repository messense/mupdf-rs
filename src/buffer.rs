use std::ptr;

use mupdf_sys::*;

use crate::context;

#[derive(Debug)]
pub struct Buffer {
    inner: *mut fz_buffer,
}

impl Buffer {
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    pub fn with_capacity(cap: usize) -> Self {
        let inner = unsafe { fz_new_buffer(context(), cap) };
        Self { inner }
    }

    pub fn len(&self) -> usize {
        unsafe { fz_buffer_storage(context(), self.inner, ptr::null_mut()) }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                fz_drop_buffer(context(), self.inner);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::Buffer;

    #[test]
    fn test_buffer_len() {
        let buf = Buffer::new();
        assert_eq!(buf.len(), 0);
    }
}

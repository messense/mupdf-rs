use std::ptr;

use mupdf_sys::*;

#[derive(Debug)]
pub struct Context {
    inner: *mut fz_context,
}

impl Context {
    pub fn new() -> Self {
        let inner =
            unsafe { fz_new_context(ptr::null_mut(), ptr::null_mut(), FZ_STORE_DEFAULT as usize) };
        unsafe {
            fz_register_document_handlers(inner);
        }
        Self { inner }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                fz_drop_context(self.inner);
            }
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Context {
    fn clone(&self) -> Self {
        let inner = unsafe { fz_clone_context(self.inner) };
        Self { inner }
    }
}

#[cfg(test)]
mod test {
    use super::Context;

    #[test]
    fn test_context() {
        let _ctx = Context::default();
    }
}

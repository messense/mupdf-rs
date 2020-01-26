use mupdf_sys::*;

use crate::context;

#[derive(Debug)]
pub struct Shade {
    pub(crate) inner: *mut fz_shade,
}

impl Drop for Shade {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                fz_drop_shade(context(), self.inner);
            }
        }
    }
}

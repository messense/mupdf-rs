use mupdf_sys::*;

use crate::context;

#[derive(Debug)]
pub struct Separations {
    pub(crate) inner: *mut fz_separations,
}

impl Separations {
    pub(crate) unsafe fn from_raw(ptr: *mut fz_separations) -> Self {
        Self { inner: ptr }
    }

    pub fn len(&self) -> usize {
        unsafe { fz_count_separations(context(), self.inner) as usize }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn active_count(&self) -> usize {
        unsafe { fz_count_active_separations(context(), self.inner) as usize }
    }
}

impl Drop for Separations {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe { fz_drop_separations(context(), self.inner) }
        }
    }
}

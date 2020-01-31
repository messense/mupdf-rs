use mupdf_sys::*;

use bitflags::bitflags;

use crate::context;

bitflags! {
    pub struct TextPageOptions: u32 {
        const BLOCK_IMAGE = FZ_STEXT_BLOCK_IMAGE as _;
        const BLOCK_TEXT = FZ_STEXT_BLOCK_TEXT as _;
        const INHIBIT_SPACES = FZ_STEXT_INHIBIT_SPACES as _;
        const PRESERVE_IMAGES = FZ_STEXT_PRESERVE_IMAGES as _;
        const PRESERVE_LIGATURES = FZ_STEXT_PRESERVE_LIGATURES as _;
        const PRESERVE_WHITESPACE = FZ_STEXT_PRESERVE_WHITESPACE as _;
    }
}

#[derive(Debug)]
pub struct TextPage {
    pub(crate) inner: *mut fz_stext_page,
}

impl Drop for TextPage {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                fz_drop_stext_page(context(), self.inner);
            }
        }
    }
}

use std::io::Read;

use mupdf_sys::*;

use bitflags::bitflags;

use crate::{context, Buffer, Error};

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

impl TextPage {
    pub(crate) unsafe fn from_raw(ptr: *mut fz_stext_page) -> Self {
        Self { inner: ptr }
    }

    pub fn to_text(&self) -> Result<String, Error> {
        let mut buf = unsafe {
            let inner = ffi_try!(mupdf_stext_page_to_text(context(), self.inner));
            Buffer::from_raw(inner)
        };
        let mut text = String::new();
        buf.read_to_string(&mut text)?;
        Ok(text)
    }
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

use mupdf_sys::*;

use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::ptr;

use crate::pdf::PdfDocument;
use crate::{Image, Matrix, Rect};

#[derive(Clone, Copy)]
pub struct PdfFilterOptions {
    pub(crate) inner: *mut pdf_filter_options,
}

// Callback types
// TODO: I don't think this is necessary for anything.
pub type ImageFilter = fn(ctm: &Matrix, name: &str, image: &Image) -> Image;
pub type TextFilter = fn(ucsbuf: i32, ucslen: i32, trm: &Matrix, ctm: &Matrix, bbox: &Rect) -> i32;
pub type AfterTextObject = fn(doc: &PdfDocument, chain: &pdf_processor, ctm: &Matrix);
pub type EndPage = fn();

impl PdfFilterOptions {
    pub(crate) unsafe fn from_raw(ptr: *mut pdf_filter_options) -> Self {
        Self { inner: ptr }
    }

    pub fn ascii(&self) -> bool {
        unsafe { (*self.inner).ascii != 0 }
    }

    pub fn set_ascii(&mut self, value: bool) -> &mut Self {
        unsafe {
            (*self.inner).ascii = if value { 1 } else { 0 };
        }
        self
    }

    pub fn recurse(&self) -> bool {
        unsafe { (*self.inner).recurse != 0 }
    }

    pub fn set_recurse(&mut self, value: bool) -> &mut Self {
        unsafe {
            (*self.inner).recurse = if value { 1 } else { 0 };
        }
        self
    }

    pub fn sanitize(&self) -> bool {
        unsafe { (*self.inner).sanitize != 0 }
    }

    pub fn set_sanitize(&mut self, value: bool) -> &mut Self {
        unsafe {
            (*self.inner).sanitize = if value { 1 } else { 0 };
        }
        self
    }

    pub fn instance_forms(&self) -> bool {
        unsafe { (*self.inner).instance_forms != 0 }
    }

    pub fn set_instance_forms(&mut self, value: bool) -> &mut Self {
        unsafe {
            (*self.inner).instance_forms = if value { 1 } else { 0 };
        }
        self
    }

    pub fn set_image_filter(
        &mut self,
        wrapper: fn(&Matrix, &str, &Image) -> Option<Image>,
    ) -> &mut Self {
        use once_cell::sync::OnceCell;

        static WRAPPER: OnceCell<fn(&Matrix, &str, &Image) -> Option<Image>> = OnceCell::new();
        // TODO: for some reason I can't use unwrap here??
        let _ = WRAPPER.set(wrapper);

        unsafe extern "C" fn image_filter_callback(
            _ctx: *mut fz_context,
            _opaque: *mut c_void,
            ctm: fz_matrix,
            name: *const c_char,
            image: *mut fz_image,
        ) -> *mut mupdf_sys::fz_image {
            let ret = std::panic::catch_unwind(|| match WRAPPER.get() {
                Some(wrapper) => wrapper(
                    &Matrix::from(ctm),
                    CStr::from_ptr(name).to_str().unwrap(),
                    &Image::from_raw(image),
                ),
                None => None,
            });

            if let Ok(Some(ret)) = ret {
                ret.inner
            } else {
                ptr::null_mut()
            }
        }

        unsafe {
            (*self.inner).image_filter = Some(image_filter_callback);
        }
        self
    }
}

impl PdfFilterOptions {}

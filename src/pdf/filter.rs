use mupdf_sys::*;

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::ptr;

use crate::pdf::PdfDocument;
use crate::{context, Image, Matrix, Rect};

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
        wrapper: impl Fn(&Matrix, &str, &Image) -> Image,
    ) -> &mut Self {
        // TODO: not sure how to set the wrapper so that the user can pass high
        // level objects instead of C structs and pointers. I apparently can't
        // assign a closure, so not sure how that could be possible without
        // access to the wrapper.
        let image_filter_callback = move |ctx: *mut fz_context,
                                          opaque: *mut c_void,
                                          ctm: fz_matrix,
                                          name: *const c_char,
                                          image: *mut fz_image|
              -> *mut mupdf_sys::fz_image {
            // TODO: what to do with panics?
            let ret = std::panic::catch_unwind(|| unsafe {
                wrapper(
                    &Matrix::from(ctm),
                    CStr::from_ptr(name).to_str().unwrap(),
                    &Image::from_raw(image),
                )
            });

            ret.unwrap().inner
        };

        // This won't work either because I can't access `wrapper`
        /*
        unsafe extern "C" fn image_filter_callback(
            ctx: *mut fz_context,
            opaque: *mut c_void,
            ctm: fz_matrix,
            name: *const c_char,
            image: *mut fz_image,
        ) -> *mut mupdf_sys::fz_image {
            let ret = std::panic::catch_unwind(|| {
                wrapper(
                    &Matrix::from(ctm),
                    CStr::from_ptr(name),
                    &Image::from_raw(image),
                )
            });

            ret.inner
        }
        */

        unsafe {
            (*self.inner).image_filter = Some(image_filter_callback);
        }
        self
    }
}

impl PdfFilterOptions {}

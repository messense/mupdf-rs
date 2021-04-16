use mupdf_sys::*;

use std::ffi::CStr;
use std::os::raw::{c_char, c_void};

use crate::{context, Image, Matrix, Rect};
use crate::pdf::PdfDocument;

#[derive(Clone, Copy)]
pub struct PdfFilterOptions {
    pub(crate) inner: *mut pdf_filter_options,
}


// Callback types
pub type ImageFilter = fn(ctm: &Matrix, name: &str, image: &Image) -> Image;
pub type TextFilter = fn(ucsbuf: i32, ucslen: i32, trm: &Matrix, ctm: &Matrix, bbox: &Rect) -> i32;
pub type AfterTextObject = fn(doc: &PdfDocument, chain: &pdf_processor, ctm: &Matrix);
pub type EndPage = fn();

fn image_filter_callback(ctx: *mut fz_context, opaque: *mut c_void, ctm: *mut fz_matrix, name: *const c_char, image: *mut fz_image) {
    value(&Matrix::from(ctm), CStr::from_ptr(name).to_str().unwrap(), &Image::from_raw(image))
}

impl PdfFilterOptions {
    pub(crate) unsafe fn from_raw(ptr: *mut pdf_filter_options) -> Self {
        Self {
            inner: ptr,
        }
    }

    pub fn ascii(&self) -> bool {
        unsafe { (*self.inner).ascii != 0 }
    }

    pub  fn set_ascii(&mut self, value: bool) -> &mut Self {
        unsafe { (*self.inner).ascii = if value { 1 } else { 0 }; }
        self
    }

    pub fn recurse(&self) -> bool {
        unsafe { (*self.inner).recurse != 0 }
    }

    pub  fn set_recurse(&mut self, value: bool) -> &mut Self {
        unsafe { (*self.inner).recurse = if value { 1 } else { 0 }; }
        self
    }

    pub fn sanitize(&self) -> bool {
        unsafe { (*self.inner).sanitize != 0 }
    }

    pub  fn set_sanitize(&mut self, value: bool) -> &mut Self {
        unsafe { (*self.inner).sanitize = if value { 1 } else { 0 }; }
        self
    }

    pub fn instance_forms(&self) -> bool {
        unsafe { (*self.inner).instance_forms != 0 }
    }

    pub  fn set_instance_forms(&mut self, value: bool) -> &mut Self {
        unsafe { (*self.inner).instance_forms = if value { 1 } else { 0 }; }
        self
    }

    // TODO: not sure how to handle functions. These should most likely not be closures.
    // TODO: should be Option
    pub fn image_filter(&self) -> Option<ImageFilter> {
        unsafe {
            |ctm: &Matrix, name: &str, image: &Image| {
                (*self.inner).image_filter(context(), self.inner, ctm.into(), CStr::from(name), image.inner)
            }
        }
    }

    // TODO: same for the setter
    // TODO: how to pass params?
    pub  fn set_image_filter(&mut self, value: ImageFilter) -> &mut Self {
        unsafe {
            (*self.inner).image_filter = Some(image_filter_callback)
        }
        self
    }
}

impl PdfFilterOptions {
}

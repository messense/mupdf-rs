use mupdf_sys::*;

use std::marker::PhantomData;
use std::mem;

use crate::pdf::PdfDocument;
use crate::{Image, Matrix, Rect};

type InnerCallback<'a> = Box<dyn FnMut(Matrix, &str, &Image) -> Option<Image> + 'a>;

// Double indirection is required to pass trait objects trough FFI.
type BoxedCallback<'a> = Box<InnerCallback<'a>>;

pub struct PdfFilterOptions<'a> {
    pub(crate) inner: pdf_filter_options,
    // This corresponds to the BoxedCallback<'a> which is pointed to by `inner.opaque`
    phantom: Option<PhantomData<BoxedCallback<'a>>>,
}

// Callback types
pub type TextFilter = fn(ucsbuf: i32, ucslen: i32, trm: &Matrix, ctm: &Matrix, bbox: &Rect) -> i32;
pub type AfterTextObject = fn(doc: &PdfDocument, chain: &pdf_processor, ctm: &Matrix);
pub type EndPage = fn();

impl Default for PdfFilterOptions<'_> {
    fn default() -> Self {
        Self {
            inner: unsafe { mem::zeroed() },
            phantom: None,
        }
    }
}

impl Drop for PdfFilterOptions<'_> {
    fn drop(&mut self) {
        self.drop_opaque_if_necessary();
    }
}

impl<'a> PdfFilterOptions<'a> {
    fn drop_opaque_if_necessary(&mut self) -> Option<BoxedCallback<'a>> {
        self.phantom.take().map(|_| {
            let ptr = self.inner.opaque;
            self.inner.opaque = std::ptr::null_mut();
            unsafe { Box::from_raw(ptr as *mut InnerCallback<'a>) }
        })
    }

    pub fn ascii(&self) -> bool {
        self.inner.ascii != 0
    }

    pub fn set_ascii(&mut self, value: bool) -> &mut Self {
        self.inner.ascii = if value { 1 } else { 0 };
        self
    }

    pub fn recurse(&self) -> bool {
        self.inner.recurse != 0
    }

    pub fn set_recurse(&mut self, value: bool) -> &mut Self {
        self.inner.recurse = if value { 1 } else { 0 };
        self
    }

    pub fn instance_forms(&self) -> bool {
        self.inner.instance_forms != 0
    }

    pub fn set_instance_forms(&mut self, value: bool) -> &mut Self {
        self.inner.instance_forms = if value { 1 } else { 0 };
        self
    }
}

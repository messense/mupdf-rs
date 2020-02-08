use mupdf_sys::*;

use crate::{context, Error, Font, Matrix, Rect, StrokeState};

#[derive(Debug)]
pub struct Text {
    pub(crate) inner: *mut fz_text,
}

impl Text {
    pub fn new() -> Result<Self, Error> {
        let inner = unsafe { ffi_try!(mupdf_new_text(context())) };
        Ok(Self { inner })
    }

    pub fn bounds(&self, stroke: &StrokeState, ctm: &Matrix) -> Result<Rect, Error> {
        let rect = unsafe {
            ffi_try!(mupdf_bound_text(
                context(),
                self.inner,
                stroke.inner,
                ctm.into()
            ))
        };
        Ok(rect.into())
    }

    pub fn spans(&self) -> TextSpanIter {
        TextSpanIter {
            next: unsafe { (*self.inner).head },
        }
    }
}

impl Drop for Text {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                fz_drop_text(context(), self.inner);
            }
        }
    }
}

#[derive(Debug)]
pub struct TextSpan {
    inner: *mut fz_text_span,
}

impl TextSpan {
    pub fn font(&self) -> Option<Font> {
        unsafe {
            let ptr = (*self.inner).font;
            if ptr.is_null() {
                return None;
            }
            fz_keep_font(context(), ptr);
            Some(Font::from_raw(ptr))
        }
    }
}

#[derive(Debug)]
pub struct TextSpanIter {
    next: *mut fz_text_span,
}

impl Iterator for TextSpanIter {
    type Item = TextSpan;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next.is_null() {
            return None;
        }
        let node = self.next;
        self.next = unsafe { (*node).next };
        Some(TextSpan { inner: node })
    }
}

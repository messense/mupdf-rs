use std::slice;

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
    pub fn font(&self) -> Font {
        unsafe {
            let ptr = (*self.inner).font;
            fz_keep_font(context(), ptr);
            Font::from_raw(ptr)
        }
    }

    pub fn items(&self) -> TextItemIter {
        unsafe {
            let len = (*self.inner).len as usize;
            let items = slice::from_raw_parts((*self.inner).items, len);
            TextItemIter {
                items,
                index: 0,
                total: len,
            }
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

#[derive(Debug)]
pub struct TextItem {
    inner: fz_text_item,
}

impl TextItem {
    #[inline]
    pub fn x(&self) -> f32 {
        self.inner.x
    }

    #[inline]
    pub fn y(&self) -> f32 {
        self.inner.y
    }

    #[inline]
    pub fn gid(&self) -> i32 {
        self.inner.gid
    }

    #[inline]
    pub fn ucs(&self) -> i32 {
        self.inner.ucs
    }
}

#[derive(Debug)]
pub struct TextItemIter<'a> {
    items: &'a [fz_text_item],
    index: usize,
    total: usize,
}

impl<'a> Iterator for TextItemIter<'a> {
    type Item = TextItem;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.total {
            return None;
        }
        let item = self.items[self.index];
        self.index += 1;
        Some(TextItem { inner: item })
    }
}

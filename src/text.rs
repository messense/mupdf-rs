use std::convert::TryInto;
use std::slice;

use mupdf_sys::*;
use num_enum::TryFromPrimitive;

use crate::{context, Error, Font, Matrix, Rect, StrokeState, WriteMode};

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

#[derive(Debug, Clone, Copy, PartialEq, TryFromPrimitive)]
#[repr(u32)]
pub enum BidiDirection {
    Ltr = fz_bidi_direction_FZ_BIDI_LTR as u32,
    Neutral = fz_bidi_direction_FZ_BIDI_NEUTRAL as u32,
    Rtl = fz_bidi_direction_FZ_BIDI_RTL as u32,
}

#[derive(Debug, Clone, Copy, PartialEq, TryFromPrimitive)]
#[repr(u32)]
pub enum Language {
    Unset = fz_text_language_FZ_LANG_UNSET as u32,
    Ja = fz_text_language_FZ_LANG_ja as u32,
    Ko = fz_text_language_FZ_LANG_ko as u32,
    Ur = fz_text_language_FZ_LANG_ur as u32,
    Urd = fz_text_language_FZ_LANG_urd as u32,
    Zh = fz_text_language_FZ_LANG_zh as u32,
    ZhHans = fz_text_language_FZ_LANG_zh_Hans as u32,
    ZhHant = fz_text_language_FZ_LANG_zh_Hant as u32,
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

    pub fn trm(&self) -> Matrix {
        unsafe { (*self.inner).trm.into() }
    }

    pub fn wmode(&self) -> WriteMode {
        unsafe { (*self.inner).wmode().try_into().unwrap() }
    }

    pub fn set_wmode(&mut self, wmode: WriteMode) {
        unsafe {
            (*self.inner).set_wmode(wmode as _);
        }
    }

    pub fn bidi_level(&self) -> u32 {
        unsafe { (*self.inner).bidi_level() }
    }

    pub fn set_bidi_level(&mut self, bidi_level: u32) {
        unsafe { (*self.inner).set_bidi_level(bidi_level) }
    }

    pub fn markup_dir(&self) -> BidiDirection {
        unsafe { (*self.inner).markup_dir().try_into().unwrap() }
    }

    pub fn set_markup_dir(&mut self, dir: BidiDirection) {
        unsafe { (*self.inner).set_markup_dir(dir as _) }
    }

    pub fn language(&self) -> Language {
        unsafe { (*self.inner).language().try_into().unwrap() }
    }

    pub fn set_language(&mut self, language: Language) {
        unsafe { (*self.inner).set_language(language as _) }
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

/// Text buffer
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

impl Iterator for TextItemIter<'_> {
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

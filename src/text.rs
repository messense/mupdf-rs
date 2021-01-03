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
#[cfg_attr(target_env = "msvc", repr(i32))]
#[cfg_attr(not(target_env = "msvc"), repr(u32))]
pub enum BidiDirection {
    Ltr = fz_bidi_direction_FZ_BIDI_LTR,
    Neutral = fz_bidi_direction_FZ_BIDI_NEUTRAL,
    Rtl = fz_bidi_direction_FZ_BIDI_RTL,
}

#[derive(Debug, Clone, Copy, PartialEq, TryFromPrimitive)]
#[cfg_attr(target_env = "msvc", repr(i32))]
#[cfg_attr(not(target_env = "msvc"), repr(u32))]
pub enum Language {
    Unset = fz_text_language_FZ_LANG_UNSET,
    Ja = fz_text_language_FZ_LANG_ja,
    Ko = fz_text_language_FZ_LANG_ko,
    Ur = fz_text_language_FZ_LANG_ur,
    Urd = fz_text_language_FZ_LANG_urd,
    Zh = fz_text_language_FZ_LANG_zh,
    ZhHans = fz_text_language_FZ_LANG_zh_Hans,
    ZhHant = fz_text_language_FZ_LANG_zh_Hant,
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

    #[cfg(target_env = "msvc")]
    pub fn markup_dir(&self) -> BidiDirection {
        unsafe { ((*self.inner).markup_dir() as i32).try_into().unwrap() }
    }

    #[cfg(not(target_env = "msvc"))]
    pub fn markup_dir(&self) -> BidiDirection {
        unsafe { (*self.inner).markup_dir().try_into().unwrap() }
    }

    pub fn set_markup_dir(&mut self, dir: BidiDirection) {
        unsafe { (*self.inner).set_markup_dir(dir as _) }
    }

    #[cfg(target_env = "msvc")]
    pub fn language(&self) -> Language {
        unsafe { ((*self.inner).language() as i32).try_into().unwrap() }
    }

    #[cfg(not(target_env = "msvc"))]
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

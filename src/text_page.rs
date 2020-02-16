use std::convert::TryInto;
use std::ffi::CString;
use std::io::Read;
use std::ptr;
use std::slice;

use bitflags::bitflags;
use mupdf_sys::*;
use num_enum::TryFromPrimitive;

use crate::{context, Buffer, Error, Image, Matrix, Point, Quad, Rect, WriteMode};

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

    pub fn blocks(&self) -> TextBlockIter {
        TextBlockIter {
            next: unsafe { (*self.inner).first_block },
        }
    }

    pub fn search(&self, needle: &str, hit_max: u32) -> Result<Vec<Quad>, Error> {
        struct Quads(*mut fz_quad);

        impl Drop for Quads {
            fn drop(&mut self) {
                if !self.0.is_null() {
                    unsafe { fz_free(context(), self.0 as _) };
                }
            }
        }

        let c_needle = CString::new(needle)?;
        let hit_max = if hit_max < 1 { 16 } else { hit_max };
        let mut hit_count = 0;
        unsafe {
            let quads = Quads(ffi_try!(mupdf_search_stext_page(
                context(),
                self.inner,
                c_needle.as_ptr(),
                hit_max as _,
                &mut hit_count
            )));
            if hit_count == 0 {
                return Ok(Vec::new());
            }
            let items = slice::from_raw_parts(quads.0, hit_count as usize);
            Ok(items.iter().map(|quad| (*quad).into()).collect())
        }
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

#[derive(Debug, Clone, Copy, PartialEq, TryFromPrimitive)]
#[repr(u32)]
pub enum TextBlockType {
    Text = FZ_STEXT_BLOCK_TEXT,
    Image = FZ_STEXT_BLOCK_IMAGE,
}

#[derive(Debug)]
pub struct TextBlock {
    inner: *mut fz_stext_block,
}

impl TextBlock {
    pub fn r#type(&self) -> TextBlockType {
        unsafe { ((*self.inner).type_ as u32).try_into().unwrap() }
    }

    pub fn bounds(&self) -> Rect {
        unsafe { (*self.inner).bbox.into() }
    }

    pub fn lines(&self) -> TextLineIter {
        unsafe {
            if (*self.inner).type_ as u32 == FZ_STEXT_BLOCK_TEXT {
                return TextLineIter {
                    next: (*self.inner).u.t.first_line,
                };
            }
        }
        TextLineIter {
            next: ptr::null_mut(),
        }
    }

    pub fn ctm(&self) -> Option<Matrix> {
        unsafe {
            if (*self.inner).type_ as u32 == FZ_STEXT_BLOCK_IMAGE {
                return Some((*self.inner).u.i.transform.into());
            }
        }
        None
    }

    pub fn image(&self) -> Option<Image> {
        unsafe {
            if (*self.inner).type_ as u32 == FZ_STEXT_BLOCK_IMAGE {
                let inner = (*self.inner).u.i.image;
                fz_keep_image(context(), inner);
                return Some(Image::from_raw(inner));
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct TextBlockIter {
    next: *mut fz_stext_block,
}

impl Iterator for TextBlockIter {
    type Item = TextBlock;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next.is_null() {
            return None;
        }
        let node = self.next;
        self.next = unsafe { (*node).next };
        Some(TextBlock { inner: node })
    }
}

#[derive(Debug)]
pub struct TextLine {
    inner: *mut fz_stext_line,
}

impl TextLine {
    pub fn bounds(&self) -> Rect {
        unsafe { (*self.inner).bbox.into() }
    }

    pub fn wmode(&self) -> WriteMode {
        unsafe { ((*self.inner).wmode as u32).try_into().unwrap() }
    }

    pub fn chars(&self) -> TextCharIter {
        TextCharIter {
            next: unsafe { (*self.inner).first_char },
        }
    }
}

#[derive(Debug)]
pub struct TextLineIter {
    next: *mut fz_stext_line,
}

impl Iterator for TextLineIter {
    type Item = TextLine;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next.is_null() {
            return None;
        }
        let node = self.next;
        self.next = unsafe { (*node).next };
        Some(TextLine { inner: node })
    }
}

#[derive(Debug)]
pub struct TextChar {
    inner: *mut fz_stext_char,
}

impl TextChar {
    pub fn char(&self) -> Option<char> {
        std::char::from_u32(unsafe { (*self.inner).c as u32 })
    }

    pub fn origin(&self) -> Point {
        unsafe { (*self.inner).origin.into() }
    }

    pub fn size(&self) -> f32 {
        unsafe { (*self.inner).size }
    }
}

#[derive(Debug)]
pub struct TextCharIter {
    next: *mut fz_stext_char,
}

impl Iterator for TextCharIter {
    type Item = TextChar;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next.is_null() {
            return None;
        }
        let node = self.next;
        self.next = unsafe { (*node).next };
        Some(TextChar { inner: node })
    }
}

#[cfg(test)]
mod test {
    use crate::{Document, TextPageOptions};

    #[test]
    fn test_text_page_search() {
        use crate::{Point, Quad};

        let doc = Document::open("tests/files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let text_page = page0.to_text_page(TextPageOptions::BLOCK_IMAGE).unwrap();
        let hits = text_page.search("Dummy", 1).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(
            hits,
            [Quad {
                ul: Point {
                    x: 56.8,
                    y: 69.32512
                },
                ur: Point {
                    x: 115.85405,
                    y: 69.32512
                },
                ll: Point {
                    x: 56.8,
                    y: 87.311844
                },
                lr: Point {
                    x: 115.85405,
                    y: 87.311844
                }
            }]
        );

        let hits = text_page.search("Not Found", 1).unwrap();
        assert_eq!(hits.len(), 0);
    }
}

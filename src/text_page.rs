use std::convert::TryInto;
use std::ffi::CString;
use std::io::Read;
use std::marker::PhantomData;
use std::ptr;
use std::slice;

use bitflags::bitflags;
use mupdf_sys::*;
use num_enum::TryFromPrimitive;

use crate::{context, Buffer, Error, Image, Matrix, Point, Quad, Rect, WriteMode};

bitflags! {
    /// Options for creating a pixmap and draw device.
    pub struct TextPageOptions: u32 {
        const BLOCK_IMAGE = FZ_STEXT_BLOCK_IMAGE as _;
        const BLOCK_TEXT = FZ_STEXT_BLOCK_TEXT as _;
        const INHIBIT_SPACES = FZ_STEXT_INHIBIT_SPACES as _;
        const PRESERVE_IMAGES = FZ_STEXT_PRESERVE_IMAGES as _;
        const PRESERVE_LIGATURES = FZ_STEXT_PRESERVE_LIGATURES as _;
        const PRESERVE_WHITESPACE = FZ_STEXT_PRESERVE_WHITESPACE as _;
    }
}

/// A text page is a list of blocks, together with an overall bounding box
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
            _marker: PhantomData,
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
    Text = FZ_STEXT_BLOCK_TEXT as u32,
    Image = FZ_STEXT_BLOCK_IMAGE as u32,
}

/// A text block is a list of lines of text (typically a paragraph), or an image.
pub struct TextBlock<'a> {
    inner: &'a fz_stext_block,
}

impl TextBlock<'_> {
    pub fn r#type(&self) -> TextBlockType {
        (self.inner.type_ as u32).try_into().unwrap()
    }

    pub fn bounds(&self) -> Rect {
        self.inner.bbox.into()
    }

    pub fn lines(&self) -> TextLineIter {
        unsafe {
            if self.inner.type_ == FZ_STEXT_BLOCK_TEXT as i32 {
                return TextLineIter {
                    next: self.inner.u.t.first_line,
                    _marker: PhantomData,
                };
            }
        }
        TextLineIter {
            next: ptr::null_mut(),
            _marker: PhantomData,
        }
    }

    pub fn ctm(&self) -> Option<Matrix> {
        unsafe {
            if self.inner.type_ == FZ_STEXT_BLOCK_IMAGE as i32 {
                return Some(self.inner.u.i.transform.into());
            }
        }
        None
    }

    pub fn image(&self) -> Option<Image> {
        unsafe {
            if self.inner.type_ == FZ_STEXT_BLOCK_IMAGE as i32 {
                let inner = self.inner.u.i.image;
                fz_keep_image(context(), inner);
                return Some(Image::from_raw(inner));
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct TextBlockIter<'a> {
    next: *mut fz_stext_block,
    _marker: PhantomData<TextBlock<'a>>,
}

impl<'a> Iterator for TextBlockIter<'a> {
    type Item = TextBlock<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next.is_null() {
            return None;
        }
        let node = unsafe { &*self.next };
        self.next = node.next;
        Some(TextBlock { inner: node })
    }
}

/// A text line is a list of characters that share a common baseline.
#[derive(Debug)]
pub struct TextLine<'a> {
    inner: &'a fz_stext_line,
}

impl TextLine<'_> {
    pub fn bounds(&self) -> Rect {
        self.inner.bbox.into()
    }

    pub fn wmode(&self) -> WriteMode {
        (self.inner.wmode as u32).try_into().unwrap()
    }

    pub fn chars(&self) -> TextCharIter {
        TextCharIter {
            next: self.inner.first_char,
            _marker: PhantomData,
        }
    }
}

#[derive(Debug)]
pub struct TextLineIter<'a> {
    next: *mut fz_stext_line,
    _marker: PhantomData<TextLine<'a>>,
}

impl<'a> Iterator for TextLineIter<'a> {
    type Item = TextLine<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next.is_null() {
            return None;
        }
        let node = unsafe { &*self.next };
        self.next = node.next;
        Some(TextLine { inner: node })
    }
}

/// A text char is a unicode character, the style in which is appears,
/// and the point at which it is positioned.
#[derive(Debug)]
pub struct TextChar<'a> {
    inner: &'a fz_stext_char,
}

impl TextChar<'_> {
    pub fn char(&self) -> Option<char> {
        std::char::from_u32(self.inner.c as u32)
    }

    pub fn origin(&self) -> Point {
        self.inner.origin.into()
    }

    pub fn size(&self) -> f32 {
        self.inner.size
    }

    pub fn quad(&self) -> Quad {
        self.inner.quad.into()
    }
}

#[derive(Debug)]
pub struct TextCharIter<'a> {
    next: *mut fz_stext_char,
    _marker: PhantomData<TextChar<'a>>,
}

impl<'a> Iterator for TextCharIter<'a> {
    type Item = TextChar<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next.is_null() {
            return None;
        }
        let node = unsafe { &*self.next };
        self.next = node.next;
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

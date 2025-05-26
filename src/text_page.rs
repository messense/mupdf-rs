use std::{
    convert::TryInto,
    ffi::{c_int, c_void, CString},
    io::Read,
    marker::PhantomData,
    ptr::{self, NonNull},
    slice,
};

use bitflags::bitflags;
use mupdf_sys::*;
use num_enum::TryFromPrimitive;

use crate::{
    context, rust_slice_to_ffi_ptr, unsafe_impl_ffi_wrapper, Buffer, Error, FFIWrapper, Image,
    Matrix, Point, Quad, Rect, WriteMode,
};
use crate::{output::Output, FFIAnalogue};

bitflags! {
    /// Options for creating a pixmap and draw device.
    pub struct TextPageFlags: u32 {
        const PRESERVE_LIGATURES = FZ_STEXT_PRESERVE_LIGATURES as _;
        const PRESERVE_WHITESPACE = FZ_STEXT_PRESERVE_WHITESPACE as _;
        const PRESERVE_IMAGES = FZ_STEXT_PRESERVE_IMAGES as _;
        const INHIBIT_SPACES = FZ_STEXT_INHIBIT_SPACES as _;
        const DEHYPHENATE = FZ_STEXT_DEHYPHENATE as _;
        const PRESERVE_SPANS = FZ_STEXT_PRESERVE_SPANS as _;
        const CLIP = FZ_STEXT_CLIP as _;
        const USE_CID_FOR_UNKNOWN_UNICODE = FZ_STEXT_USE_CID_FOR_UNKNOWN_UNICODE as _;
        const COLLECT_STRUCTURE = FZ_STEXT_COLLECT_STRUCTURE as _;
        const ACCURATE_BBOXES = FZ_STEXT_ACCURATE_BBOXES as _;
        const COLLECT_VECTORS = FZ_STEXT_COLLECT_VECTORS as _;
        const IGNORE_ACTUALTEXT = FZ_STEXT_IGNORE_ACTUALTEXT as _;
        const SEGMENT = FZ_STEXT_SEGMENT as _;
        const PARAGRAPH_BREAK = FZ_STEXT_PARAGRAPH_BREAK as _;
        const TABLE_HUNT = FZ_STEXT_TABLE_HUNT as _;
        const COLLECT_STYLES = FZ_STEXT_COLLECT_STYLES as _;
        const USE_GID_FOR_UNKNOWN_UNICODE = FZ_STEXT_USE_GID_FOR_UNKNOWN_UNICODE as _;
        const ACCURATE_ASCENDERS = FZ_STEXT_ACCURATE_ASCENDERS as _;
        const ACCURATE_SIDE_BEARINGS = FZ_STEXT_ACCURATE_SIDE_BEARINGS as _;
    }
}

/// A text page is a list of blocks, together with an overall bounding box
#[derive(Debug)]
pub struct TextPage {
    pub(crate) inner: NonNull<fz_stext_page>,
}

unsafe_impl_ffi_wrapper!(TextPage, fz_stext_page, fz_drop_stext_page);

impl TextPage {
    pub fn to_html(&self, id: i32) -> Result<String, Error> {
        let mut buf = Buffer::with_capacity(8192);

        let out = Output::from_buffer(&buf);
        unsafe {
            ffi_try!(mupdf_print_stext_page_as_html(
                context(),
                out.inner.as_ptr(),
                self.inner.as_ptr(),
                id
            ))?
        };
        drop(out);

        let mut res = String::new();
        buf.read_to_string(&mut res)?;
        Ok(res)
    }

    pub fn to_xhtml(&self, id: i32) -> Result<String, Error> {
        let mut buf = Buffer::with_capacity(8192);

        let out = Output::from_buffer(&buf);
        unsafe {
            ffi_try!(mupdf_print_stext_page_as_xhtml(
                context(),
                out.inner.as_ptr(),
                self.inner.as_ptr(),
                id
            ))?
        };
        drop(out);

        let mut res = String::new();
        buf.read_to_string(&mut res)?;
        Ok(res)
    }

    pub fn to_xml(&self, id: i32) -> Result<String, Error> {
        let mut buf = Buffer::with_capacity(8192);

        let out = Output::from_buffer(&buf);
        unsafe {
            ffi_try!(mupdf_print_stext_page_as_xml(
                context(),
                out.inner.as_ptr(),
                self.inner.as_ptr(),
                id
            ))?
        };
        drop(out);

        let mut res = String::new();
        buf.read_to_string(&mut res)?;
        Ok(res)
    }

    pub fn to_text(&self) -> Result<String, Error> {
        let mut buf = Buffer::with_capacity(8192);

        let out = Output::from_buffer(&buf);
        unsafe {
            ffi_try!(mupdf_print_stext_page_as_text(
                context(),
                out.inner.as_ptr(),
                self.inner.as_ptr()
            ))?
        };
        drop(out);

        let mut res = String::new();
        buf.read_to_string(&mut res)?;
        Ok(res)
    }

    pub fn to_json(&self, scale: f32) -> Result<String, Error> {
        let mut buf = Buffer::with_capacity(8192);

        let out = Output::from_buffer(&buf);
        unsafe {
            ffi_try!(mupdf_print_stext_page_as_json(
                context(),
                out.inner.as_ptr(),
                self.inner.as_ptr(),
                scale
            ))?
        };
        drop(out);

        let mut res = String::new();
        buf.read_to_string(&mut res)?;
        Ok(res)
    }

    pub fn blocks(&self) -> TextBlockIter {
        TextBlockIter {
            next: unsafe { (*self.as_ptr().cast_mut()).first_block },
            _marker: PhantomData,
        }
    }

    pub fn search(&self, needle: &str) -> Result<Vec<Quad>, Error> {
        let mut vec = Vec::new();
        self.search_cb(needle, &mut vec, |v, quads| {
            v.extend(quads.iter().cloned());
            SearchHitResponse::ContinueSearch
        })?;
        Ok(vec)
    }

    /// Search through the page, finding all instances of `needle` and processing them through
    /// `cb`.
    /// Note that the `&[Quad]` given to `cb` in its invocation lives only during the time that
    /// `cb` is being evaluated. That means the following won't work or compile:
    ///
    /// ```compile_fail
    /// # use mupdf::{TextPage, Quad, text_page::SearchHitResponse};
    /// # let text_page: TextPage = todo!();
    /// let mut quads: Vec<&Quad> = Vec::new();
    /// text_page.search_cb("search term", &mut quads, |v, quads: &[Quad]| {
    ///     v.extend(quads);
    ///     SearchHitResponse::ContinueSearch
    /// }).unwrap();
    /// ```
    ///
    /// But the following will:
    /// ```no_run
    /// # use mupdf::{TextPage, Quad, text_page::SearchHitResponse};
    /// # let text_page: TextPage = todo!();
    /// let mut quads: Vec<Quad> = Vec::new();
    /// text_page.search_cb("search term", &mut quads, |v, quads: &[Quad]| {
    ///     v.extend(quads.iter().cloned());
    ///     SearchHitResponse::ContinueSearch
    /// }).unwrap();
    /// ```
    pub fn search_cb<T, F>(&self, needle: &str, data: &mut T, cb: F) -> Result<u32, Error>
    where
        T: ?Sized,
        F: Fn(&mut T, &[Quad]) -> SearchHitResponse,
    {
        // This struct allows us to wrap both the callback that the user gave us and the data so
        // that we can pass it into the ffi callback nicely
        struct FnWithData<'parent, T: ?Sized, F>
        where
            F: Fn(&mut T, &[Quad]) -> SearchHitResponse,
        {
            data: &'parent mut T,
            f: F,
        }

        let mut opaque = FnWithData { data, f: cb };

        // And then here's the `fn` that we'll pass in - it has to be an fn, not capturing context,
        // because it needs to be unsafe extern "C". to be used with FFI.
        unsafe extern "C" fn ffi_cb<T, F>(
            _ctx: *mut fz_context,
            data: *mut c_void,
            num_quads: c_int,
            hit_bbox: *mut fz_quad,
        ) -> c_int
        where
            T: ?Sized,
            F: Fn(&mut T, &[Quad]) -> SearchHitResponse,
            Quad: FFIAnalogue<FFIType = fz_quad>,
        {
            // This is upheld by our `FFIAnalogue` bound above
            let quad_ptr = hit_bbox.cast::<Quad>();
            let Some(nn) = NonNull::new(quad_ptr) else {
                return SearchHitResponse::ContinueSearch as c_int;
            };

            // This guarantee is upheld by mupdf - they're giving us a pointer to the same type we
            // gave them.
            let data = data.cast::<FnWithData<'_, T, F>>();

            // But if they like gave us a -1 for number of results or whatever, give up on
            // decoding.
            let Ok(len) = usize::try_from(num_quads) else {
                return SearchHitResponse::ContinueSearch as c_int;
            };

            // SAFETY: We've ensure nn is not null, and we're trusting the FFI layer for the other
            // invariants (about actually holding the data, etc)
            let slice = unsafe { slice::from_raw_parts_mut(nn.as_ptr(), len) };

            // Get the function and the data
            // SAFETY: Trusting that the FFI layer actually gave us this ptr
            let f = unsafe { &(*data).f };
            // SAFETY: Trusting that the FFI layer actually gave us this ptr
            let data = unsafe { &mut (*data).data };

            // And call the function with the data
            f(data, slice) as c_int
        }

        let c_needle = CString::new(needle)?;
        unsafe {
            ffi_try!(mupdf_search_stext_page_cb(
                context(),
                self.as_ptr().cast_mut(),
                c_needle.as_ptr(),
                Some(ffi_cb::<T, F>),
                &raw mut opaque as *mut c_void
            ))
        }
        .map(|count| count as u32)
    }

    pub fn highlight_selection(
        &mut self,
        a: Point,
        b: Point,
        quads: &[Quad],
    ) -> Result<i32, Error> {
        let (ptr, len): (*const fz_quad, _) = rust_slice_to_ffi_ptr(quads)?;

        unsafe {
            ffi_try!(mupdf_highlight_selection(
                context(),
                self.as_mut_ptr(),
                a.into(),
                b.into(),
                ptr as *mut fz_quad,
                len
            ))
        }
    }
}

#[repr(i32)]
pub enum SearchHitResponse {
    ContinueSearch = 0,
    AbortSearch = 1,
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
    use crate::{document::test_document, text_page::SearchHitResponse, Document, TextPageFlags};

    #[test]
    fn test_page_to_html() {
        let doc = test_document!("..", "files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let text_page = page0.to_text_page(TextPageFlags::empty()).unwrap();
        let html = text_page.to_html(0).unwrap();
        assert!(html.contains("Dummy PDF file"));
    }

    #[test]
    fn test_page_to_xhtml() {
        let doc = test_document!("..", "files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let text_page = page0.to_text_page(TextPageFlags::empty()).unwrap();
        let xhtml = text_page.to_xhtml(0).unwrap();
        assert!(xhtml.contains("Dummy PDF file"));
    }

    #[test]
    fn test_page_to_xml() {
        let doc = test_document!("..", "files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let text_page = page0.to_text_page(TextPageFlags::empty()).unwrap();
        let xml = text_page.to_xml(0).unwrap();
        assert!(xml.contains("Dummy PDF file"));
    }

    #[test]
    fn test_page_to_text() {
        let doc = test_document!("..", "files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let text_page = page0.to_text_page(TextPageFlags::empty()).unwrap();
        let text = text_page.to_text().unwrap();
        assert_eq!(text, "Dummy PDF file\n\n");
    }

    #[test]
    fn test_text_page_search() {
        use crate::{Point, Quad};

        let doc = test_document!("..", "files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let text_page = page0.to_text_page(TextPageFlags::empty()).unwrap();
        let hits = text_page.search("Dummy").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(
            &*hits,
            [Quad {
                ul: Point {
                    x: 56.8,
                    y: 69.32953
                },
                ur: Point {
                    x: 115.85159,
                    y: 69.32953
                },
                ll: Point {
                    x: 56.8,
                    y: 87.29713
                },
                lr: Point {
                    x: 115.85159,
                    y: 87.29713
                }
            }]
        );

        let hits = text_page.search("Not Found").unwrap();
        assert_eq!(hits.len(), 0);
    }

    #[test]
    fn test_text_page_cb_search() {
        let doc = test_document!("..", "files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let text_page = page0.to_text_page(TextPageFlags::empty()).unwrap();
        let mut sum_x = 0.0;
        let num_hits = text_page
            .search_cb("Dummy", &mut sum_x, |acc, hits| {
                for q in hits {
                    *acc += q.ul.x + q.ur.x + q.ll.x + q.lr.x;
                }
                SearchHitResponse::ContinueSearch
            })
            .unwrap();
        assert_eq!(num_hits, 1);
        assert_eq!(sum_x, 56.8 + 115.85159 + 56.8 + 115.85159);

        let hits = text_page.search("Not Found").unwrap();
        assert_eq!(hits.len(), 0);
    }
}

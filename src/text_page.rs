use std::{
    convert::TryInto,
    ffi::{c_char, c_int, c_void, CStr, CString},
    fmt,
    io::Read,
    marker::PhantomData,
    ptr::{self, NonNull},
    slice,
};

use bitflags::bitflags;
use mupdf_sys::*;

use crate::{
    context, from_enum, rust_slice_to_ffi_ptr, unsafe_impl_ffi_wrapper, Buffer, Error, FFIWrapper,
    Font, Image, Matrix, Point, Quad, Rect, WriteMode,
};
use crate::{output::Output, FFIAnalogue};

bitflags! {
    /// Per-char flags reported by the structured-text extractor.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct TextCharFlags: u16 {
        const STRIKEOUT = FZ_STEXT_STRIKEOUT as _;
        const UNDERLINE = FZ_STEXT_UNDERLINE as _;
        const SYNTHETIC = FZ_STEXT_SYNTHETIC as _;
        /// Real or synthesised ("fake") bold.
        const BOLD = FZ_STEXT_BOLD as _;
        const FILLED = FZ_STEXT_FILLED as _;
        const STROKED = FZ_STEXT_STROKED as _;
        const CLIPPED = FZ_STEXT_CLIPPED as _;
        const UNICODE_IS_CID = FZ_STEXT_UNICODE_IS_CID as _;
        const UNICODE_IS_GID = FZ_STEXT_UNICODE_IS_GID as _;
        const SYNTHETIC_LARGE = FZ_STEXT_SYNTHETIC_LARGE as _;
    }
}

bitflags! {
    /// Options for creating a pixmap and draw device.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextExtractOptions {
    pub flags: TextPageFlags,
}

impl Default for TextExtractOptions {
    fn default() -> Self {
        Self {
            flags: TextPageFlags::empty(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextWord {
    pub text: String,
    pub bounds: Rect,
    pub block: usize,
    pub line: usize,
    pub word: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructuredTextChar {
    pub ch: char,
    pub origin: Point,
    pub quad: Quad,
    pub size: f32,
    pub argb: u32,
    pub flags: TextCharFlags,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructuredTextLine {
    pub bounds: Rect,
    pub wmode: WriteMode,
    pub text: String,
    pub chars: Vec<StructuredTextChar>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextBlockContent {
    Text { lines: Vec<StructuredTextLine> },
    Image { transform: Option<Matrix> },
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructuredTextBlock {
    pub bounds: Rect,
    pub content: TextBlockContent,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructuredText {
    pub blocks: Vec<StructuredTextBlock>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawStructuredText {
    pub blocks: Vec<StructuredTextBlock>,
}

/// A text page is a list of blocks, together with an overall bounding box
#[derive(Debug)]
pub struct TextPage {
    pub(crate) inner: NonNull<fz_stext_page>,
}

unsafe_impl_ffi_wrapper!(TextPage, fz_stext_page, fz_drop_stext_page);

impl TextPage {
    pub fn to_html(&self, id: i32, full: bool) -> Result<String, Error> {
        let mut buf = Buffer::with_capacity(8192)?;

        let out = Output::from_buffer(&buf)?;
        if full {
            unsafe {
                ffi_try!(mupdf_print_stext_header_as_html(
                    context(),
                    out.inner.as_ptr()
                ))?
            };
        }
        unsafe {
            ffi_try!(mupdf_print_stext_page_as_html(
                context(),
                out.inner.as_ptr(),
                self.inner.as_ptr(),
                id
            ))?
        };
        if full {
            unsafe {
                ffi_try!(mupdf_print_stext_trailer_as_html(
                    context(),
                    out.inner.as_ptr()
                ))?
            };
        }
        drop(out);

        let mut res = String::new();
        buf.read_to_string(&mut res)?;
        Ok(res)
    }

    pub fn to_xhtml(&self, id: i32) -> Result<String, Error> {
        let mut buf = Buffer::with_capacity(8192)?;

        let out = Output::from_buffer(&buf)?;
        unsafe {
            ffi_try!(mupdf_print_stext_header_as_xhtml(
                context(),
                out.inner.as_ptr()
            ))?;
            ffi_try!(mupdf_print_stext_page_as_xhtml(
                context(),
                out.inner.as_ptr(),
                self.inner.as_ptr(),
                id
            ))?;
            ffi_try!(mupdf_print_stext_trailer_as_html(
                context(),
                out.inner.as_ptr()
            ))?;
        }
        drop(out);

        let mut res = String::new();
        buf.read_to_string(&mut res)?;
        Ok(res)
    }

    pub fn to_xml(&self, id: i32) -> Result<String, Error> {
        let mut buf = Buffer::with_capacity(8192)?;

        let out = Output::from_buffer(&buf)?;
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
        let mut buf = Buffer::with_capacity(8192)?;

        let out = Output::from_buffer(&buf)?;
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
        let mut buf = Buffer::with_capacity(8192)?;

        let out = Output::from_buffer(&buf)?;
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

    /// The top-level blocks of the page.
    ///
    /// With [`TextPageFlags::COLLECT_STRUCTURE`] MuPDF emits [`TextBlockType::Struct`] blocks and
    /// nests the text blocks that belong to the structure under them; those nested blocks are not
    /// yielded here, reach them through [`TextBlock::structure`] and [`TextStruct::blocks`].
    /// MuPDF's own segmentation and classification can also produce Struct blocks without that
    /// flag, so do not assume their absence.
    pub fn blocks(&self) -> TextBlockIter<'_> {
        TextBlockIter {
            next: unsafe { (*self.as_ptr().cast_mut()).first_block },
            _marker: PhantomData,
        }
    }

    pub fn words(&self) -> Vec<TextWord> {
        let mut words = Vec::new();

        for (block_index, block) in self.blocks().enumerate() {
            if block.r#type() != TextBlockType::Text {
                continue;
            }
            for (line_index, line) in block.lines().enumerate() {
                let mut current = String::new();
                let mut bounds: Option<Rect> = None;
                let mut word_index = 0;

                for ch in line.chars() {
                    let Some(value) = ch.char() else {
                        continue;
                    };
                    if value.is_whitespace() {
                        if !current.is_empty() {
                            words.push(TextWord {
                                text: std::mem::take(&mut current),
                                bounds: bounds.take().unwrap(),
                                block: block_index,
                                line: line_index,
                                word: word_index,
                            });
                            word_index += 1;
                        }
                        continue;
                    }

                    let char_bounds = Rect::from(ch.quad());
                    bounds = Some(match bounds {
                        Some(bounds) => bounds.r#union(&char_bounds),
                        None => char_bounds,
                    });
                    current.push(value);
                }

                if !current.is_empty() {
                    words.push(TextWord {
                        text: current,
                        bounds: bounds.unwrap(),
                        block: block_index,
                        line: line_index,
                        word: word_index,
                    });
                }
            }
        }

        words
    }

    pub fn structured(&self) -> StructuredText {
        let mut blocks = Vec::new();

        for block in self.blocks() {
            let content = match block.r#type() {
                TextBlockType::Text => {
                    let lines = block
                        .lines()
                        .map(|line| {
                            let mut text = String::new();
                            let chars = line
                                .chars()
                                .filter_map(|ch| {
                                    let value = ch.char()?;
                                    text.push(value);
                                    Some(StructuredTextChar {
                                        ch: value,
                                        origin: ch.origin(),
                                        quad: ch.quad(),
                                        size: ch.size(),
                                        argb: ch.argb(),
                                        flags: ch.flags(),
                                    })
                                })
                                .collect();
                            StructuredTextLine {
                                bounds: line.bounds(),
                                wmode: line.wmode(),
                                text,
                                chars,
                            }
                        })
                        .collect();
                    TextBlockContent::Text { lines }
                }
                TextBlockType::Image => TextBlockContent::Image {
                    transform: block.ctm(),
                },
                _ => TextBlockContent::Other,
            };

            blocks.push(StructuredTextBlock {
                bounds: block.bounds(),
                content,
            });
        }

        StructuredText { blocks }
    }

    pub fn raw_structured(&self) -> RawStructuredText {
        RawStructuredText {
            blocks: self.structured().blocks,
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
            crate::guard_ffi_callback(|| f(data, slice)) as c_int
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

    /// The text MuPDF selects between two points of the page, in reading order: the same
    /// characters [`Self::highlight_selection`] highlights for those points. Lines end with `\n`,
    /// or with `\r\n` when `crlf` is set.
    pub fn copy_selection(&self, a: Point, b: Point, crlf: bool) -> Result<String, Error> {
        let ptr = unsafe {
            ffi_try!(mupdf_copy_selection(
                context(),
                self.as_ptr().cast_mut(),
                a.into(),
                b.into(),
                c_int::from(crlf)
            ))
        }?;
        Ok(take_c_string(ptr))
    }

    /// The text of the characters inside `area`, in reading order. Lines end with `\n`, or with
    /// `\r\n` when `crlf` is set.
    pub fn copy_rectangle(&self, area: Rect, crlf: bool) -> Result<String, Error> {
        let ptr = unsafe {
            ffi_try!(mupdf_copy_rectangle(
                context(),
                self.as_ptr().cast_mut(),
                area.into(),
                c_int::from(crlf)
            ))
        }?;
        Ok(take_c_string(ptr))
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

/// Copies a string MuPDF allocated into a `String` and frees it. A null pointer is the empty string.
fn take_c_string(ptr: *mut c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let text = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    unsafe { fz_free(context(), ptr.cast()) };
    text
}

#[repr(i32)]
pub enum SearchHitResponse {
    ContinueSearch = 0,
    AbortSearch = 1,
}

from_enum! { c_int => c_int,
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum TextBlockType {
        Text = FZ_STEXT_BLOCK_TEXT,
        Image = FZ_STEXT_BLOCK_IMAGE,
        Struct = FZ_STEXT_BLOCK_STRUCT,
        Vector = FZ_STEXT_BLOCK_VECTOR,
        Grid = FZ_STEXT_BLOCK_GRID,
    }
}

from_enum! { c_int => c_int,
    /// Standard PDF structure types (`fz_structure`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum StructureType {
        /// Not a valid structure type (`FZ_STRUCTURE_INVALID`).
        Invalid = FZ_STRUCTURE_INVALID,
        Document = FZ_STRUCTURE_DOCUMENT,
        Part = FZ_STRUCTURE_PART,
        Art = FZ_STRUCTURE_ART,
        Sect = FZ_STRUCTURE_SECT,
        Div = FZ_STRUCTURE_DIV,
        BlockQuote = FZ_STRUCTURE_BLOCKQUOTE,
        Caption = FZ_STRUCTURE_CAPTION,
        TOC = FZ_STRUCTURE_TOC,
        TOCI = FZ_STRUCTURE_TOCI,
        Index = FZ_STRUCTURE_INDEX,
        NonStruct = FZ_STRUCTURE_NONSTRUCT,
        Private = FZ_STRUCTURE_PRIVATE,
        DocumentFragment = FZ_STRUCTURE_DOCUMENTFRAGMENT,
        Aside = FZ_STRUCTURE_ASIDE,
        Title = FZ_STRUCTURE_TITLE,
        FENote = FZ_STRUCTURE_FENOTE,
        Sub = FZ_STRUCTURE_SUB,
        P = FZ_STRUCTURE_P,
        H = FZ_STRUCTURE_H,
        H1 = FZ_STRUCTURE_H1,
        H2 = FZ_STRUCTURE_H2,
        H3 = FZ_STRUCTURE_H3,
        H4 = FZ_STRUCTURE_H4,
        H5 = FZ_STRUCTURE_H5,
        H6 = FZ_STRUCTURE_H6,
        List = FZ_STRUCTURE_LIST,
        ListItem = FZ_STRUCTURE_LISTITEM,
        Label = FZ_STRUCTURE_LABEL,
        ListBody = FZ_STRUCTURE_LISTBODY,
        Table = FZ_STRUCTURE_TABLE,
        TR = FZ_STRUCTURE_TR,
        TH = FZ_STRUCTURE_TH,
        TD = FZ_STRUCTURE_TD,
        THead = FZ_STRUCTURE_THEAD,
        TBody = FZ_STRUCTURE_TBODY,
        TFoot = FZ_STRUCTURE_TFOOT,
        Span = FZ_STRUCTURE_SPAN,
        Quote = FZ_STRUCTURE_QUOTE,
        Note = FZ_STRUCTURE_NOTE,
        Reference = FZ_STRUCTURE_REFERENCE,
        BibEntry = FZ_STRUCTURE_BIBENTRY,
        Code = FZ_STRUCTURE_CODE,
        Link = FZ_STRUCTURE_LINK,
        Annot = FZ_STRUCTURE_ANNOT,
        Em = FZ_STRUCTURE_EM,
        Strong = FZ_STRUCTURE_STRONG,
        Ruby = FZ_STRUCTURE_RUBY,
        RB = FZ_STRUCTURE_RB,
        RT = FZ_STRUCTURE_RT,
        RP = FZ_STRUCTURE_RP,
        Warichu = FZ_STRUCTURE_WARICHU,
        WT = FZ_STRUCTURE_WT,
        WP = FZ_STRUCTURE_WP,
        Figure = FZ_STRUCTURE_FIGURE,
        Formula = FZ_STRUCTURE_FORMULA,
        Form = FZ_STRUCTURE_FORM,
        Artifact = FZ_STRUCTURE_ARTIFACT,
    }
}

impl StructureType {
    /// The standard structure name as reported by MuPDF (for example `"P"` or `"H1"`).
    /// Names come from MuPDF's `fz_structure_to_string`, except `NonStruct`, which MuPDF
    /// misspells.
    #[must_use]
    pub fn name(self) -> &'static str {
        // MuPDF returns "NonDtruct" for this one (typo in `fz_structure_to_string`).
        if self == Self::NonStruct {
            return "NonStruct";
        }
        // SAFETY: `fz_structure_to_string` is a pure function returning a pointer to a static,
        // NUL-terminated string for every input value.
        unsafe { CStr::from_ptr(fz_structure_to_string(self.into())) }
            .to_str()
            .unwrap_or("Invalid")
    }
}

/// A text block is a list of lines of text (typically a paragraph), or an image.
pub struct TextBlock<'a> {
    inner: &'a fz_stext_block,
}

impl<'a> TextBlock<'a> {
    pub fn r#type(&self) -> TextBlockType {
        self.inner.type_.try_into().unwrap()
    }

    pub fn bounds(&self) -> Rect {
        self.inner.bbox.into()
    }

    pub fn lines(&self) -> TextLineIter<'a> {
        unsafe {
            if self.inner.type_ == FZ_STEXT_BLOCK_TEXT as c_int {
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

    /// For [`TextBlockType::Struct`] blocks, the structure element this block stands for.
    /// `None` for other block types or when MuPDF has no structure node for the block.
    pub fn structure(&self) -> Option<TextStruct<'a>> {
        if self.inner.type_ != FZ_STEXT_BLOCK_STRUCT as c_int {
            return None;
        }
        // SAFETY: the block type is `FZ_STEXT_BLOCK_STRUCT`, so `u.s` is the active union member.
        let down = unsafe { self.inner.u.s.down };
        // A non-null `down` points to an `fz_stext_struct` owned by the text page, which outlives
        // `'a`; the struct is never mutated after the page is built.
        TextStruct::new(down)
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
                if inner.is_null() {
                    return None;
                }
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

/// A node of the structure tree collected with [`TextPageFlags::COLLECT_STRUCTURE`].
#[derive(Clone, Copy)]
pub struct TextStruct<'a> {
    // Kept as a raw pointer (not a reference) because `raw` is a variable-length tail that a
    // reference to the fixed-size `fz_stext_struct` would not cover.
    inner: NonNull<fz_stext_struct>,
    _marker: PhantomData<&'a fz_stext_struct>,
}

impl fmt::Debug for TextStruct<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextStruct")
            .field("standard", &self.standard())
            .field("raw", &self.raw_bytes().map(String::from_utf8_lossy))
            .finish()
    }
}

impl<'a> TextStruct<'a> {
    fn new(ptr: *mut fz_stext_struct) -> Option<Self> {
        NonNull::new(ptr).map(|inner| Self {
            inner,
            _marker: PhantomData,
        })
    }

    /// The standard structure type of this element.
    pub fn standard(&self) -> StructureType {
        // SAFETY: `inner` points to a struct owned by the text page, valid for `'a`; only fixed
        // fields are read, through the raw pointer.
        unsafe { (*self.inner.as_ptr()).standard }
            .try_into()
            .unwrap_or(StructureType::Invalid)
    }

    /// The raw role name from the document as bytes (PDF names may hold arbitrary bytes), or
    /// `None` if the element has no name.
    pub fn raw_bytes(&self) -> Option<&'a [u8]> {
        // SAFETY: `raw` is a flexible array holding a NUL-terminated string allocated together
        // with the struct (MuPDF always writes the terminator). The pointer is derived from the
        // raw struct pointer, so it keeps provenance over the whole allocation, and stays valid
        // for `'a`.
        let bytes = unsafe {
            let ptr = ptr::addr_of!((*self.inner.as_ptr()).raw).cast::<c_char>();
            CStr::from_ptr(ptr).to_bytes()
        };
        (!bytes.is_empty()).then_some(bytes)
    }

    /// The raw (possibly non-standard) role name from the document. `None` both when the
    /// element has no name and when the name is not valid UTF-8; use [`Self::raw_bytes`] to
    /// tell them apart.
    pub fn raw(&self) -> Option<&'a str> {
        std::str::from_utf8(self.raw_bytes()?).ok()
    }

    /// The position of this element in its parent's `/K` array (as computed by MuPDF), or
    /// `None` if the struct has no owning block. MuPDF reports `-1` when it cannot locate the
    /// element in its parent's kids. For struct blocks that MuPDF synthesizes or renumbers
    /// during segmentation or classification, this is MuPDF's own sibling ordering key, not
    /// necessarily a `/K` position.
    pub fn index(&self) -> Option<i32> {
        // SAFETY: `up` is null or a block owned by the text page, which outlives `self`.
        let up = unsafe { (*self.inner.as_ptr()).up.as_ref() }?;
        if up.type_ != FZ_STEXT_BLOCK_STRUCT as c_int {
            return None;
        }
        // SAFETY: the block type is `FZ_STEXT_BLOCK_STRUCT`, so `u.s` is the active union member.
        Some(unsafe { up.u.s.index })
    }

    /// The enclosing structure element, if any.
    pub fn parent(&self) -> Option<TextStruct<'a>> {
        // SAFETY: `parent` is null or points to another struct owned by the same text page.
        Self::new(unsafe { (*self.inner.as_ptr()).parent })
    }

    /// The child blocks of this structure element.
    pub fn blocks(&self) -> TextBlockIter<'a> {
        TextBlockIter {
            // SAFETY: fixed field read through the raw pointer of a live struct.
            next: unsafe { (*self.inner.as_ptr()).first_block },
            _marker: PhantomData,
        }
    }
}

/// A text line is a list of characters that share a common baseline.
#[derive(Debug)]
pub struct TextLine<'a> {
    inner: &'a fz_stext_line,
}

impl<'a> TextLine<'a> {
    pub fn bounds(&self) -> Rect {
        self.inner.bbox.into()
    }

    pub fn wmode(&self) -> WriteMode {
        (self.inner.wmode as u32).try_into().unwrap()
    }

    /// The normalized direction of the baseline.
    pub fn dir(&self) -> Point {
        self.inner.dir.into()
    }

    pub fn chars(&self) -> TextCharIter<'a> {
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

    /// Fill color, packed as `0xAARRGGBB` (sRGB).
    pub fn argb(&self) -> u32 {
        self.inner.argb
    }

    /// MuPDF's bidi marker for this char. Bit 0 is set for right-to-left. MuPDF keeps only
    /// `level & 1` (not the Unicode embedding level), and uses 3 for right-to-left chars it
    /// detected as laid out in visual order. If any char of a line has 3, MuPDF then reverses
    /// every right-to-left run of that line.
    /// Prefer [`Self::is_rtl`].
    ///
    /// For PDF pages the value is a per-char guess from the Unicode class of the char (strong
    /// R/AL and Arabic numbers are right-to-left; strong L and European numbers
    /// left-to-right; other weak and neutral chars inherit the previous level), so it
    /// depends on the font providing correct Unicode (e.g. a `ToUnicode` CMap). For HTML/EPUB
    /// it comes from the layout engine's bidi algorithm.
    pub fn bidi(&self) -> u16 {
        self.inner.bidi
    }

    /// Whether the char is right-to-left (bit 0 of [`Self::bidi`]).
    pub fn is_rtl(&self) -> bool {
        self.inner.bidi & 1 == 1
    }

    pub fn flags(&self) -> TextCharFlags {
        TextCharFlags::from_bits_truncate(self.inner.flags)
    }

    /// Returns `None` if mupdf did not associate a font to the TextChar.
    /// Bumps the font refcount so the returned [`Font`] can outlive this [`TextChar`].
    pub fn font(&self) -> Option<Font> {
        let ptr = self.inner.font;
        if ptr.is_null() {
            return None;
        }
        unsafe {
            fz_keep_font(context(), ptr);
            Some(Font::from_raw(ptr))
        }
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

        let html = text_page.to_html(0, false).unwrap();
        assert!(!html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("Dummy PDF file"));

        let html = text_page.to_html(0, true).unwrap();
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("Dummy PDF file"));
    }

    #[test]
    fn test_page_to_xhtml() {
        let doc = test_document!("..", "files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let text_page = page0.to_text_page(TextPageFlags::empty()).unwrap();

        let xhtml = text_page.to_xhtml(0).unwrap();
        assert!(xhtml.starts_with("<?xml "));
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
    fn test_text_page_words_and_structured_extractors() {
        use crate::TextBlockContent;

        let doc = test_document!("..", "files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let text_page = page0.to_text_page(TextPageFlags::empty()).unwrap();

        let words = text_page.words();
        assert_eq!(
            words
                .iter()
                .map(|word| word.text.as_str())
                .collect::<Vec<_>>(),
            vec!["Dummy", "PDF", "file"]
        );
        assert_eq!(words[0].block, 0);
        assert_eq!(words[0].line, 0);
        assert_eq!(words[0].word, 0);
        assert!(!words[0].bounds.is_empty());

        let structured = text_page.structured();
        assert_eq!(structured.blocks.len(), 1);
        let TextBlockContent::Text { lines } = &structured.blocks[0].content else {
            panic!("expected text block");
        };
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "Dummy PDF file");
        assert_eq!(lines[0].chars.len(), "Dummy PDF file".chars().count());
        assert_eq!(lines[0].chars[0].ch, 'D');

        let raw = text_page.raw_structured();
        assert_eq!(raw.blocks, structured.blocks);

        let json = text_page.to_json(1.0).unwrap();
        assert!(json.contains("Dummy PDF file"));
    }

    #[test]
    fn test_page_text_convenience_extractors() {
        let doc = test_document!("..", "files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let options = crate::TextExtractOptions::default();

        assert_eq!(page0.text(options).unwrap(), "Dummy PDF file\n\n");
        let words = page0.words(options).unwrap();
        assert_eq!(words.len(), 3);
        assert_eq!(words[1].text, "PDF");
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
    fn test_text_page_copy_selection_and_rectangle() {
        use crate::{Point, Rect};

        let doc = test_document!("..", "files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let text_page = page0.to_text_page(TextPageFlags::empty()).unwrap();
        // "Dummy PDF file" is one line; its "Dummy" spans x 56.8..115.9, y 69.3..87.3.
        let all = text_page
            .copy_selection(Point::new(0.0, 78.0), Point::new(600.0, 78.0), false)
            .unwrap();
        assert_eq!(all.trim_end(), "Dummy PDF file");
        let word = text_page
            .copy_selection(Point::new(56.0, 78.0), Point::new(116.0, 78.0), false)
            .unwrap();
        assert_eq!(word.trim_end(), "Dummy");
        let boxed = text_page
            .copy_rectangle(Rect::new(50.0, 65.0, 120.0, 90.0), false)
            .unwrap();
        assert_eq!(boxed.trim_end(), "Dummy");
        // Nothing between two points of an empty area of the page.
        let none = text_page
            .copy_selection(Point::new(0.0, 700.0), Point::new(1.0, 700.0), false)
            .unwrap();
        assert!(!none.contains("Dummy"), "{none:?}");
    }

    #[test]
    fn test_text_char_font_and_flags() {
        use crate::TextCharFlags;

        let doc = test_document!("..", "files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let text_page = page0.to_text_page(TextPageFlags::empty()).unwrap();

        let block = text_page.blocks().next().expect("at least one block");
        let line = block.lines().next().expect("at least one line");
        let first_char = line.chars().next().expect("at least one char");

        assert_eq!(first_char.char(), Some('D'));
        assert_eq!(first_char.argb(), 0xff000000);
        assert!(first_char.flags().contains(TextCharFlags::FILLED));

        let font = first_char.font().expect("char should have a font");
        assert!(!font.name().is_empty());
    }

    #[test]
    fn test_text_char_font_outlives_text_page() {
        let doc = test_document!("..", "files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let text_page = page0.to_text_page(TextPageFlags::empty()).unwrap();

        let (font, expected_name) = {
            let block = text_page.blocks().next().expect("at least one block");
            let line = block.lines().next().expect("at least one line");
            let text_char = line.chars().next().expect("at least one char");
            let font = text_char.font().expect("char should have a font");
            let expected_name = font.name().to_owned();
            (font, expected_name)
        };

        drop(text_page);
        drop(page0);
        drop(doc);

        assert_eq!(font.name(), expected_name);
        assert!(!font.name().is_empty());
        let _ = font.is_bold();
        let _ = font.ascender();
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

    /// Builds a minimal single-page PDF (Helvetica) from raw page content and extra catalog
    /// entries; `extra_objs` are numbered from 6 on.
    fn synthetic_pdf(
        content: &str,
        catalog_extra: &str,
        page_extra: &str,
        extra_objs: &[&str],
    ) -> Vec<u8> {
        synthetic_pdf_with_font(
            content,
            catalog_extra,
            page_extra,
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
            extra_objs,
        )
    }

    /// Like [`synthetic_pdf`] with a custom font dictionary for `/F1` (object 5).
    fn synthetic_pdf_with_font(
        content: &str,
        catalog_extra: &str,
        page_extra: &str,
        font: &str,
        extra_objs: &[&str],
    ) -> Vec<u8> {
        let mut objs: Vec<String> = vec![
            format!("<< /Type /Catalog /Pages 2 0 R {catalog_extra} >>"),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Contents 4 0 R \
                 /Resources << /Font << /F1 5 0 R >> >> {page_extra} >>"
            ),
            format!(
                "<< /Length {} >>\nstream\n{content}\nendstream",
                content.len()
            ),
            font.to_owned(),
        ];
        objs.extend(extra_objs.iter().map(|o| (*o).to_owned()));
        let mut out = b"%PDF-1.7\n".to_vec();
        let mut offsets = Vec::new();
        for (i, o) in objs.iter().enumerate() {
            offsets.push(out.len());
            out.extend(format!("{} 0 obj\n{o}\nendobj\n", i + 1).bytes());
        }
        let xref = out.len();
        out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).bytes());
        for off in offsets {
            out.extend(format!("{off:010} 00000 n \n").bytes());
        }
        out.extend(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
                objs.len() + 1
            )
            .bytes(),
        );
        out
    }

    fn text_page_of(pdf: &[u8], flags: TextPageFlags) -> crate::TextPage {
        let doc = Document::from_copied_bytes(pdf, "synthetic.pdf").unwrap();
        doc.load_page(0).unwrap().to_text_page(flags).unwrap()
    }

    #[test]
    fn test_line_dir_horizontal_and_rotated() {
        let pdf = synthetic_pdf(
            "BT /F1 12 Tf 1 0 0 1 50 250 Tm (Horizontal) Tj ET\n\
             BT /F1 12 Tf 0 1 -1 0 200 50 Tm (Rotated) Tj ET",
            "",
            "",
            &[],
        );
        let tp = text_page_of(&pdf, TextPageFlags::empty());
        let dirs: Vec<(String, crate::Point)> = tp
            .blocks()
            .flat_map(|b| b.lines().collect::<Vec<_>>())
            .map(|l| {
                let t: String = l.chars().filter_map(|c| c.char()).collect();
                (t, l.dir())
            })
            .collect();
        let horiz = dirs.iter().find(|(t, _)| t == "Horizontal").unwrap().1;
        assert!(
            (horiz.x - 1.0).abs() < 1e-3 && horiz.y.abs() < 1e-3,
            "{horiz:?}"
        );
        let rot = dirs.iter().find(|(t, _)| t == "Rotated").unwrap().1;
        // A 90 degree counter-clockwise text matrix in PDF space points up on the page, which is
        // y = -1 in MuPDF's y-down page space.
        assert!(rot.x.abs() < 1e-3 && (rot.y + 1.0).abs() < 1e-3, "{rot:?}");
    }

    #[test]
    fn test_char_bidi_even_for_latin() {
        let pdf = synthetic_pdf("BT /F1 12 Tf 1 0 0 1 50 250 Tm (Latin) Tj ET", "", "", &[]);
        let tp = text_page_of(&pdf, TextPageFlags::empty());
        let levels: Vec<u16> = tp
            .blocks()
            .flat_map(|b| b.lines().collect::<Vec<_>>())
            .flat_map(|l| l.chars().map(|c| c.bidi()).collect::<Vec<_>>())
            .collect();
        assert_eq!(levels.len(), 5);
        assert!(levels.iter().all(|l| l % 2 == 0), "{levels:?}");
        assert!(tp
            .blocks()
            .flat_map(|b| b.lines())
            .flat_map(|l| l.chars())
            .all(|c| !c.is_rtl()));
    }

    fn line_bidi(tp: &crate::TextPage) -> Vec<Vec<(char, u16)>> {
        tp.blocks()
            .flat_map(|b| b.lines().collect::<Vec<_>>())
            .map(|l| {
                l.chars()
                    .map(|c| {
                        assert_eq!(c.is_rtl(), c.bidi() & 1 == 1);
                        (c.char().unwrap(), c.bidi())
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn test_char_bidi_rtl_in_pdf() {
        // Helvetica glyphs with a ToUnicode CMap mapping 'a', 'b', 'c' to Hebrew alef, bet and
        // gimel: MuPDF guesses the bidi level of PDF text from the Unicode value of each char.
        let body = "/CIDInit /ProcSet findresource begin 12 dict begin begincmap \
            /CMapName /Adobe-Identity-UCS def /CMapType 2 def \
            1 begincodespacerange <00> <FF> endcodespacerange \
            3 beginbfchar <61> <05D0> <62> <05D1> <63> <05D2> endbfchar endcmap end end";
        let tounicode = format!("<< /Length {} >>\nstream\n{body}\nendstream", body.len());
        let pdf = synthetic_pdf_with_font(
            "BT /F1 12 Tf 1 0 0 1 50 250 Tm (abc) Tj ET\n\
             BT /F1 12 Tf 1 0 0 1 50 200 Tm (xy abc xy) Tj ET",
            "",
            "",
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /ToUnicode 6 0 R >>",
            &[tounicode.as_str()],
        );
        let lines = line_bidi(&text_page_of(&pdf, TextPageFlags::empty()));
        // The Hebrew run is drawn with an advancing pen (visual order), so MuPDF flags it with 3
        // (the first char of the run keeps 1) and reverses it into logical order.
        let (gimel, bet, alef) = ('\u{5d2}', '\u{5d1}', '\u{5d0}');
        assert_eq!(lines[0], [(gimel, 3), (bet, 3), (alef, 1)]);
        // In the mixed line the Latin text keeps level 0 and the Hebrew run is odd.
        let rtl: Vec<_> = lines[1]
            .iter()
            .filter(|(c, _)| "\u{5d0}\u{5d1}\u{5d2}".contains(*c))
            .collect();
        assert_eq!(rtl.len(), 3);
        assert!(rtl.iter().all(|(_, b)| b % 2 == 1), "{lines:?}");
        let latin: Vec<_> = lines[1]
            .iter()
            .filter(|(c, _)| matches!(c, 'x' | 'y'))
            .collect();
        assert_eq!(latin.len(), 4);
        assert!(latin.iter().all(|(_, b)| *b == 0), "{lines:?}");
    }

    #[cfg(feature = "html")]
    #[test]
    fn test_char_bidi_rtl_in_html() {
        // The HTML layout engine assigns real bidi levels (whether or not a font has glyphs for the text).
        let html = "<html><body>\
            <p dir=\"rtl\">\u{5e9}\u{5dc}\u{5d5}\u{5dd} abc \u{5e2}\u{5d5}\u{5dc}\u{5dd}</p>\
            <p>abc \u{5e9}\u{5dc}\u{5d5}\u{5dd} def</p>\
            <p>\u{627}\u{644}\u{633}\u{644}\u{627}\u{645}</p></body></html>";
        let doc = Document::from_copied_bytes(html.as_bytes(), "a.html").unwrap();
        let tp = doc
            .load_page(0)
            .unwrap()
            .to_text_page(TextPageFlags::empty())
            .unwrap();
        let lines = line_bidi(&tp);
        assert_eq!(lines.len(), 3, "{lines:?}");
        let is_hebrew_or_arabic = |c: char| ('\u{590}'..='\u{6ff}').contains(&c);
        for line in &lines {
            for (c, b) in line {
                if is_hebrew_or_arabic(*c) {
                    assert_eq!(*b, 1, "{c:?} in {line:?}");
                } else if c.is_ascii_alphabetic() {
                    assert_eq!(*b, 0, "{c:?} in {line:?}");
                }
            }
        }
        // In an rtl paragraph the spaces around the embedded Latin word take the paragraph level.
        assert_eq!(lines[0][4], (' ', 1));
        assert_eq!(lines[0][8], (' ', 1));
        // In an ltr paragraph they are level 0.
        assert_eq!(lines[1][3], (' ', 0));
        // Line direction is geometric, not bidi: rtl lines still run along +x.
        let dirs: Vec<_> = tp
            .blocks()
            .flat_map(|b| b.lines().collect::<Vec<_>>())
            .map(|l| l.dir())
            .collect();
        assert!(
            dirs.iter()
                .all(|d| (d.x - 1.0).abs() < 1e-3 && d.y.abs() < 1e-3),
            "{dirs:?}"
        );
    }

    fn tagged_pdf() -> Vec<u8> {
        tagged_pdf_with("H1", "")
    }

    fn tagged_pdf_with(heading_role: &str, root_extra: &str) -> Vec<u8> {
        synthetic_pdf(
            "/H1 <</MCID 0>> BDC BT /F1 18 Tf 1 0 0 1 50 250 Tm (Title) Tj ET EMC\n\
             /P <</MCID 1>> BDC BT /F1 12 Tf 1 0 0 1 50 200 Tm (Body text) Tj ET EMC",
            "/MarkInfo << /Marked true >> /StructTreeRoot 6 0 R",
            "/StructParents 0",
            &[
                format!("<< /Type /StructTreeRoot /K 7 0 R /ParentTree 10 0 R {root_extra} >>")
                    .as_str(),
                "<< /Type /StructElem /S /Document /P 6 0 R /K [8 0 R 9 0 R] >>",
                format!("<< /Type /StructElem /S /{heading_role} /P 7 0 R /Pg 3 0 R /K 0 >>")
                    .as_str(),
                "<< /Type /StructElem /S /P /P 7 0 R /Pg 3 0 R /K 1 >>",
                "<< /Nums [0 [8 0 R 9 0 R]] >>",
            ],
        )
    }

    fn line_texts(blocks: crate::text_page::TextBlockIter<'_>) -> Vec<String> {
        blocks
            .flat_map(|b| b.lines().collect::<Vec<_>>())
            .map(|l| l.chars().filter_map(|c| c.char()).collect())
            .collect()
    }

    #[test]
    fn test_structure_blocks_with_collect_structure() {
        use crate::text_page::{StructureType, TextBlockType};

        let pdf = tagged_pdf();
        let tp = text_page_of(&pdf, TextPageFlags::COLLECT_STRUCTURE);
        let mut found = Vec::new();
        fn walk(
            blocks: crate::text_page::TextBlockIter<'_>,
            depth: usize,
            found: &mut Vec<(usize, StructureType, Option<String>, Vec<String>)>,
        ) {
            for b in blocks {
                if b.r#type() == TextBlockType::Struct {
                    let s = b.structure().expect("struct block has a structure");
                    found.push((
                        depth,
                        s.standard(),
                        s.raw().map(str::to_owned),
                        line_texts(s.blocks()),
                    ));
                    walk(s.blocks(), depth + 1, found);
                }
            }
        }
        walk(tp.blocks(), 0, &mut found);
        let h1 = found
            .iter()
            .find(|f| f.1 == StructureType::H1)
            .unwrap_or_else(|| panic!("H1 missing: {found:?}"));
        assert_eq!(h1.2.as_deref(), Some("H1"));
        assert!(h1.3.iter().any(|t| t == "Title"), "{found:?}");
        let p = found
            .iter()
            .find(|f| f.1 == StructureType::P)
            .unwrap_or_else(|| panic!("P missing: {found:?}"));
        assert_eq!(p.2.as_deref(), Some("P"));
        assert_eq!(p.1.name(), "P");
        assert!(p.3.iter().any(|t| t == "Body text"), "{found:?}");
        // Document wraps both, so the heading and the paragraph are nested one level down.
        assert_eq!((h1.0, p.0), (1, 1), "{found:?}");
        assert_eq!(found[0].1, StructureType::Document);
        // The nested text is not repeated among the top-level blocks.
        assert!(line_texts(tp.blocks()).is_empty());
    }

    #[test]
    fn test_structure_parent_and_index() {
        use crate::text_page::TextBlockType;

        let pdf = tagged_pdf();
        let tp = text_page_of(&pdf, TextPageFlags::COLLECT_STRUCTURE);
        let top: Vec<_> = tp
            .blocks()
            .filter(|b| b.r#type() == TextBlockType::Struct)
            .collect();
        assert!(!top.is_empty());
        let first = top[0].structure().unwrap();
        assert_eq!(first.standard(), crate::text_page::StructureType::Document);
        assert!(first.parent().is_none());
        let h1 = first
            .blocks()
            .filter_map(|b| b.structure())
            .find(|s| s.raw() == Some("H1"))
            .expect("H1 under Document");
        assert_eq!(h1.parent().unwrap().standard(), first.standard());
        assert_eq!(h1.index(), Some(0));
        let p = first
            .blocks()
            .filter_map(|b| b.structure())
            .find(|s| s.raw() == Some("P"))
            .expect("P under Document");
        assert_eq!(p.index(), Some(1));
        assert_eq!(p.raw_bytes(), Some(&b"P"[..]));
        // every nested struct's parent chain ends at the root
        fn check(blocks: crate::text_page::TextBlockIter<'_>) {
            for b in blocks {
                if let Some(s) = b.structure() {
                    let _ = s.index();
                    for child in s.blocks() {
                        if let Some(cs) = child.structure() {
                            let p = cs.parent().expect("nested struct has a parent");
                            assert_eq!(p.standard(), s.standard());
                        }
                    }
                    check(s.blocks());
                }
            }
        }
        check(tp.blocks());
    }

    #[test]
    fn test_structure_type_names() {
        use crate::text_page::StructureType;
        // MuPDF spells this one "NonDtruct".
        assert_eq!(StructureType::NonStruct.name(), "NonStruct");
        assert_eq!(StructureType::Document.name(), "Document");
    }

    #[test]
    fn test_structure_role_map_custom_role() {
        use crate::text_page::StructureType;

        let pdf = tagged_pdf_with("MyHead", "/RoleMap << /MyHead /H1 >>");
        let tp = text_page_of(&pdf, TextPageFlags::COLLECT_STRUCTURE);
        let doc_struct = tp
            .blocks()
            .find_map(|b| b.structure())
            .expect("Document struct");
        let head = doc_struct
            .blocks()
            .filter_map(|b| b.structure())
            .find(|s| s.raw() == Some("MyHead"))
            .expect("custom role kept as raw name");
        assert_eq!(head.standard(), StructureType::H1);
        assert_eq!(head.standard().name(), "H1");
        assert_ne!(head.raw(), Some(head.standard().name()));
    }

    #[test]
    fn test_no_struct_blocks_without_collect_structure() {
        use crate::text_page::TextBlockType;

        let pdf = tagged_pdf();
        let tp = text_page_of(&pdf, TextPageFlags::empty());
        assert!(tp.blocks().all(|b| b.r#type() != TextBlockType::Struct));
        assert!(tp.blocks().all(|b| b.structure().is_none()));
        let texts = line_texts(tp.blocks());
        assert!(texts.iter().any(|t| t == "Title"));
        assert!(texts.iter().any(|t| t == "Body text"));
    }
}

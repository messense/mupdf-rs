use std::ffi::{c_int, CStr, CString};
use std::io::Read;
use std::ptr::{self, NonNull};

use mupdf_sys::*;

use crate::array::FzArray;
use crate::{
    context, rust_vec_from_ffi_ptr, unsafe_impl_ffi_wrapper, Buffer, Colorspace, Cookie, Device,
    DisplayList, Error, FFIWrapper, Link, Matrix, Pixmap, Quad, Rect, Separations, TextPage,
    TextPageOptions,
};

#[derive(Debug)]
pub struct Page {
    /// Ideally we'd use `Unique` here to signify ownership but that's not stable
    pub(crate) inner: NonNull<fz_page>,
    pub(crate) doc: *mut fz_document,
}

unsafe_impl_ffi_wrapper!(Page, fz_page, fz_drop_page);

impl Page {
    /// # Safety
    ///
    /// * `raw` may be null, that will cause this function to return an error. If it is non-null,
    ///   however, it must point to a valid, well-aligned instance of [`fz_page`].
    pub(crate) unsafe fn from_raw(raw: *mut fz_page) -> Result<Self, Error> {
        NonNull::new(raw)
            // SAFETY: Upheld by caller
            .map(|nn| unsafe { Self::from_non_null(nn) })
            .ok_or(Error::UnexpectedNullPtr)
    }

    /// # Safety
    ///
    /// * `nonnull` must point to a valid, well-aligned instance of [`fz_page`]
    pub(crate) unsafe fn from_non_null(nonnull: NonNull<fz_page>) -> Self {
        Self {
            inner: nonnull,
            doc: (*nonnull.as_ptr()).doc,
        }
    }

    pub fn bounds(&self) -> Result<Rect, Error> {
        unsafe { ffi_try!(mupdf_bound_page(context(), self.as_ptr() as *mut _)) }.map(Into::into)
    }

    pub fn to_pixmap(
        &self,
        ctm: &Matrix,
        cs: &Colorspace,
        alpha: f32,
        show_extras: bool,
    ) -> Result<Pixmap, Error> {
        unsafe {
            ffi_try!(mupdf_page_to_pixmap(
                context(),
                self.as_ptr() as *mut _,
                ctm.into(),
                cs.inner,
                alpha,
                show_extras
            ))
        }
        .map(|inner| unsafe { Pixmap::from_raw(inner) })
    }

    pub fn to_svg(&self, ctm: &Matrix) -> Result<String, Error> {
        let inner = unsafe {
            ffi_try!(mupdf_page_to_svg(
                context(),
                self.as_ptr() as *mut _,
                ctm.into(),
                ptr::null_mut()
            ))
        }?;
        let mut buf = unsafe { Buffer::from_raw(inner) };
        let mut svg = String::new();
        buf.read_to_string(&mut svg)?;
        Ok(svg)
    }

    pub fn to_svg_with_cookie(&self, ctm: &Matrix, cookie: &Cookie) -> Result<String, Error> {
        let inner = unsafe {
            ffi_try!(mupdf_page_to_svg(
                context(),
                self.as_ptr() as *mut _,
                ctm.into(),
                cookie.inner
            ))
        }?;
        let mut buf = unsafe { Buffer::from_raw(inner) };
        let mut svg = String::new();
        buf.read_to_string(&mut svg)?;
        Ok(svg)
    }

    pub fn to_text_page(&self, opts: TextPageOptions) -> Result<TextPage, Error> {
        unsafe {
            ffi_try!(mupdf_page_to_text_page(
                context(),
                self.as_ptr() as *mut _,
                opts.bits() as _
            ))
        }
        .map(|inner| unsafe { TextPage::from_raw(inner) })
    }

    pub fn to_display_list(&self, annotations: bool) -> Result<DisplayList, Error> {
        unsafe {
            ffi_try!(mupdf_page_to_display_list(
                context(),
                self.as_ptr() as *mut _,
                annotations
            ))
        }
        .map(|inner| unsafe { DisplayList::from_raw(inner) })
    }

    pub fn run(&self, device: &Device, ctm: &Matrix) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_run_page(
                context(),
                self.as_ptr() as *mut _,
                device.dev,
                ctm.into(),
                ptr::null_mut()
            ))
        }
    }

    pub fn run_with_cookie(
        &self,
        device: &Device,
        ctm: &Matrix,
        cookie: &Cookie,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_run_page(
                context(),
                self.as_ptr() as *mut _,
                device.dev,
                ctm.into(),
                cookie.inner
            ))
        }
    }

    pub fn run_contents(&self, device: &Device, ctm: &Matrix) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_run_page_contents(
                context(),
                self.as_ptr() as *mut _,
                device.dev,
                ctm.into(),
                ptr::null_mut()
            ))
        }
    }

    pub fn run_contents_with_cookie(
        &self,
        device: &Device,
        ctm: &Matrix,
        cookie: &Cookie,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_run_page_contents(
                context(),
                self.as_ptr() as *mut _,
                device.dev,
                ctm.into(),
                cookie.inner
            ))
        }
    }

    pub fn run_annotations(&self, device: &Device, ctm: &Matrix) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_run_page_annots(
                context(),
                self.as_ptr() as *mut _,
                device.dev,
                ctm.into(),
                ptr::null_mut()
            ))
        }
    }

    pub fn run_annotations_with_cookie(
        &self,
        device: &Device,
        ctm: &Matrix,
        cookie: &Cookie,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_run_page_annots(
                context(),
                self.as_ptr() as *mut _,
                device.dev,
                ctm.into(),
                cookie.inner
            ))
        }
    }

    pub fn run_widgets(&self, device: &Device, ctm: &Matrix) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_run_page_widgets(
                context(),
                self.as_ptr() as *mut _,
                device.dev,
                ctm.into(),
                ptr::null_mut()
            ))
        }
    }

    pub fn run_widgets_with_cookie(
        &self,
        device: &Device,
        ctm: &Matrix,
        cookie: &Cookie,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_run_page_widgets(
                context(),
                self.as_ptr() as *mut _,
                device.dev,
                ctm.into(),
                cookie.inner
            ))
        }
    }

    pub fn to_html(&self) -> Result<String, Error> {
        let inner = unsafe { ffi_try!(mupdf_page_to_html(context(), self.as_ptr() as *mut _)) }?;
        let mut buf = unsafe { Buffer::from_raw(inner) };
        let mut out = String::new();
        buf.read_to_string(&mut out)?;
        Ok(out)
    }

    pub fn stext_page_as_json_from_page(&self, scale: f32) -> Result<String, Error> {
        let inner = unsafe {
            ffi_try!(mupdf_stext_page_as_json_from_page(
                context(),
                self.as_ptr() as *mut _,
                scale
            ))
        }?;
        let mut buf = unsafe { Buffer::from_raw(inner) };
        let mut res = String::new();
        buf.read_to_string(&mut res)?;
        Ok(res)
    }

    pub fn to_xhtml(&self) -> Result<String, Error> {
        let inner = unsafe { ffi_try!(mupdf_page_to_xhtml(context(), self.as_ptr() as *mut _)) }?;
        let mut buf = unsafe { Buffer::from_raw(inner) };
        let mut out = String::new();
        buf.read_to_string(&mut out)?;
        Ok(out)
    }

    pub fn to_xml(&self) -> Result<String, Error> {
        let inner = unsafe { ffi_try!(mupdf_page_to_xml(context(), self.as_ptr() as *mut _)) }?;
        let mut buf = unsafe { Buffer::from_raw(inner) };
        let mut out = String::new();
        buf.read_to_string(&mut out)?;
        Ok(out)
    }

    pub fn to_text(&self) -> Result<String, Error> {
        let inner = unsafe { ffi_try!(mupdf_page_to_text(context(), self.as_ptr() as *mut _)) }?;
        let mut buf = unsafe { Buffer::from_raw(inner) };
        let mut out = String::new();
        buf.read_to_string(&mut out)?;
        Ok(out)
    }

    pub fn links(&self) -> Result<LinkIter, Error> {
        unsafe { ffi_try!(mupdf_load_links(context(), self.as_ptr() as *mut _)) }.map(|next| {
            LinkIter {
                next,
                doc: self.doc,
            }
        })
    }

    pub fn separations(&self) -> Result<Separations, Error> {
        unsafe { ffi_try!(mupdf_page_separations(context(), self.as_ptr() as *mut _)) }
            .map(|inner| unsafe { Separations::from_raw(inner) })
    }

    pub fn search(&self, needle: &str, hit_max: u32) -> Result<FzArray<Quad>, Error> {
        let c_needle = CString::new(needle)?;
        let hit_max = if hit_max < 1 { 16 } else { hit_max };
        let mut hit_count = 0;
        unsafe {
            ffi_try!(mupdf_search_page(
                context(),
                self.as_ptr() as *mut fz_page,
                c_needle.as_ptr(),
                hit_max as c_int,
                &mut hit_count
            ))
        }
        .and_then(|quads| unsafe { rust_vec_from_ffi_ptr(quads, hit_count) })
    }
}

impl Clone for Page {
    fn clone(&self) -> Self {
        let inner = self.inner;
        unsafe {
            fz_keep_page(context(), inner.as_ptr() as *mut _);
        }
        // SAFETY: If it was safe to construct `self`, it's safe to construct another instance of
        // `Self`.
        unsafe { Page::from_non_null(inner) }
    }
}

#[derive(Debug)]
pub struct LinkIter {
    next: *mut fz_link,
    doc: *mut fz_document,
}

impl Iterator for LinkIter {
    type Item = Link;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next.is_null() {
            return None;
        }
        let node = self.next;
        unsafe {
            self.next = (*node).next;
            let bounds = (*node).rect.into();
            let uri = CStr::from_ptr((*node).uri).to_string_lossy().into_owned();
            let mut page = 0;
            if fz_is_external_link(context(), (*node).uri) == 0 {
                page = fz_resolve_link(
                    context(),
                    self.doc,
                    (*node).uri,
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
                .page;
            }
            Some(Link {
                bounds,
                page: page as u32,
                uri,
            })
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Font {
    pub name: String,
    pub family: String,
    pub weight: String,
    pub style: String,
    pub size: u32,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct BBox {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Line {
    pub wmode: u32,
    pub bbox: BBox,
    pub font: Font,
    pub x: i32,
    pub y: i32,
    pub text: String,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Block {
    pub r#type: String,
    pub bbox: BBox,
    pub lines: Vec<Line>,
}

// StructuredText
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct StextPage {
    pub blocks: Vec<Block>,
}

#[cfg(test)]
mod test {
    use crate::{Document, Matrix};

    #[test]
    #[cfg(feature = "serde")]
    fn test_get_stext_page_as_json() {
        let path_to_doc = std::env::current_dir()
            .unwrap()
            .join("tests")
            .join("files")
            .join("dummy.pdf");
        let doc = Document::open(path_to_doc.to_str().unwrap()).unwrap();
        let page = doc.load_page(0).unwrap();
        match page.stext_page_as_json_from_page(1.0) {
            Ok(stext_json) => {
                let stext_page: serde_json::Result<crate::page::StextPage> =
                    serde_json::from_str(stext_json.as_str());
                match stext_page {
                    Ok(res) => {
                        for block in res.blocks {
                            if block.r#type.eq("text") {
                                for line in block.lines {
                                    assert_eq!(&line.text, &"Dummy PDF file".to_string());
                                }
                            }
                        }
                    }
                    Err(err) => {
                        println!("stext_page parsing error: {:?}", &err);
                    }
                }
            }
            Err(_err) => {}
        }
    }

    #[test]
    fn test_page_to_svg() {
        let doc = Document::open("tests/files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let svg = page0.to_svg(&Matrix::IDENTITY).unwrap();
        assert!(!svg.is_empty());
    }

    #[test]
    fn test_page_to_html() {
        let doc = Document::open("tests/files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let html = page0.to_html().unwrap();
        assert!(!html.is_empty());
    }

    #[test]
    fn test_page_to_xhtml() {
        let doc = Document::open("tests/files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let xhtml = page0.to_xhtml().unwrap();
        assert!(!xhtml.is_empty());
    }

    #[test]
    fn test_page_to_xml() {
        let doc = Document::open("tests/files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let xml = page0.to_xml().unwrap();
        assert!(!xml.is_empty());
    }

    #[test]
    fn test_page_to_text() {
        let doc = Document::open("tests/files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let text = page0.to_text().unwrap();
        assert!(!text.is_empty());
    }

    #[test]
    fn test_page_to_display_list() {
        let doc = Document::open("tests/files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let _dl = page0.to_display_list(true).unwrap();
        let _dl = page0.to_display_list(false).unwrap();
    }

    #[test]
    fn test_page_to_text_page() {
        use crate::TextPageOptions;

        let doc = Document::open("tests/files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let _tp = page0
            .to_text_page(TextPageOptions::PRESERVE_IMAGES)
            .unwrap();
    }

    #[test]
    fn test_page_links() {
        use crate::Link;

        let doc = Document::open("tests/files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let links_iter = page0.links().unwrap();
        let links: Vec<Link> = links_iter.collect();
        assert_eq!(links.len(), 0);
    }

    #[test]
    fn test_page_separations() {
        let doc = Document::open("tests/files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let seps = page0.separations().unwrap();
        assert_eq!(seps.len(), 0);
    }

    #[test]
    fn test_page_search() {
        use crate::{Point, Quad};

        let doc = Document::open("tests/files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let hits = page0.search("Dummy", 1).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(
            &*hits,
            [Quad {
                ul: Point {
                    x: 56.8,
                    y: 69.32512,
                },
                ur: Point {
                    x: 115.85405,
                    y: 69.32512,
                },
                ll: Point {
                    x: 56.8,
                    y: 87.311844,
                },
                lr: Point {
                    x: 115.85405,
                    y: 87.311844,
                },
            }]
        );

        let hits = page0.search("Not Found", 1).unwrap();
        assert_eq!(hits.len(), 0);
    }
}

use std::ffi::{c_int, CStr, CString};
use std::io::Read;
use std::ptr::NonNull;

use mupdf_sys::*;

use crate::array::FzArray;
use crate::document::Location;
use crate::{
    context, rust_vec_from_ffi_ptr, unsafe_impl_ffi_wrapper, Buffer, Colorspace, Cookie, Device,
    DisplayList, Document, Error, FFIWrapper, Link, Matrix, Pixmap, Quad, Rect, Separations,
    TextPage, TextPageFlags,
};

#[derive(Debug)]
pub struct Page {
    /// Ideally we'd use `Unique` here to signify ownership but that's not stable
    pub(crate) inner: NonNull<fz_page>,
    pub(crate) doc: Document,
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
            doc: Document::from_raw((*nonnull.as_ptr()).doc),
        }
    }

    pub fn bounds(&self) -> Result<Rect, Error> {
        unsafe { ffi_try!(mupdf_bound_page(context(), self.as_ptr() as *mut _)) }.map(Into::into)
    }

    pub fn to_pixmap(
        &self,
        ctm: &Matrix,
        cs: &Colorspace,
        alpha: bool,
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

    pub fn to_text_page(&self, flags: TextPageFlags) -> Result<TextPage, Error> {
        let opts = fz_stext_options {
            flags: flags.bits() as _,
            scale: 0.0,
            clip: fz_rect {
                x0: 0.0,
                y0: 0.0,
                x1: 0.0,
                y1: 0.0,
            },
        };

        let inner = unsafe {
            ffi_try!(mupdf_new_stext_page_from_page(
                context(),
                self.as_ptr().cast_mut(),
                &opts
            ))?
        };

        let inner = unsafe { NonNull::new_unchecked(inner) };

        Ok(TextPage { inner })
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

    pub fn links(&self) -> Result<LinkIter, Error> {
        unsafe { ffi_try!(mupdf_load_links(context(), self.inner.as_ptr())) }.map(|next| LinkIter {
            next,
            doc: self.doc.clone(),
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
    doc: Document,
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
            let uri = CStr::from_ptr((*node).uri);
            let location = Location::from_uri(&self.doc, uri).unwrap();

            Some(Link {
                bounds,
                location,
                uri: uri.to_string_lossy().into_owned(),
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
    use crate::{document::test_document, Document, Matrix};

    #[test]
    #[cfg(feature = "serde")]
    fn test_get_stext_page_as_json() {
        let doc = test_document!("..", "files/dummy.pdf").unwrap();
        let page = doc.load_page(0).unwrap();
        let text_page = page.to_text_page(crate::TextPageFlags::empty()).unwrap();

        let json = text_page.to_json(1.0).unwrap();
        let stext_page: crate::page::StextPage = serde_json::from_str(json.as_str()).unwrap();

        for block in stext_page.blocks {
            if block.r#type == "text" {
                for line in block.lines {
                    assert_eq!(&line.text, &"Dummy PDF file".to_string());
                }
            }
        }
    }

    #[test]
    fn test_page_to_svg() {
        let doc = test_document!("..", "files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let svg = page0.to_svg(&Matrix::IDENTITY).unwrap();
        assert!(!svg.is_empty());
    }

    #[test]
    fn test_page_to_display_list() {
        let doc = test_document!("..", "files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let _dl = page0.to_display_list(true).unwrap();
        let _dl = page0.to_display_list(false).unwrap();
    }

    #[test]
    fn test_page_to_text_page() {
        use crate::TextPageFlags;

        let doc = test_document!("..", "files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let _tp = page0.to_text_page(TextPageFlags::PRESERVE_IMAGES).unwrap();
    }

    #[test]
    fn test_page_links() {
        use crate::Link;

        let doc = test_document!("..", "files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let links_iter = page0.links().unwrap();
        let links: Vec<Link> = links_iter.collect();
        assert_eq!(links.len(), 0);
    }

    #[test]
    fn test_page_separations() {
        let doc = test_document!("..", "files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let seps = page0.separations().unwrap();
        assert_eq!(seps.len(), 0);
    }

    #[test]
    fn test_page_search() {
        use crate::{Point, Quad};

        let doc = test_document!("..", "files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let hits = page0.search("Dummy", 1).unwrap();
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

        let hits = page0.search("Not Found", 1).unwrap();
        assert_eq!(hits.len(), 0);
    }
}

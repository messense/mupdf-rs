use std::ffi::{CStr, CString};
use std::io::Write;
use std::ptr;

use mupdf_sys::*;

use crate::pdf::PdfDocument;
use crate::{context, Buffer, Colorspace, Cookie, Error, Outline, Page};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetadataName {
    Format,
    Encryption,
    Author,
    Title,
    Producer,
    Creator,
    CreationDate,
    ModDate,
    Subject,
    Keywords,
}

impl MetadataName {
    pub fn to_str(&self) -> &'static str {
        use MetadataName::*;

        match *self {
            Format => "format",
            Encryption => "encryption",
            Author => "info:Author",
            Title => "info:Title",
            Producer => "info:Producer",
            Creator => "info:Creator",
            CreationDate => "info:CreationDate",
            ModDate => "info:ModDate",
            Subject => "info:Subject",
            Keywords => "info:Keywords",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Location {
    pub chapter: i32,
    pub page: i32,
}

#[derive(Debug)]
pub struct Document {
    pub(crate) inner: *mut fz_document,
}

impl Document {
    pub(crate) unsafe fn from_raw(ptr: *mut fz_document) -> Self {
        Self { inner: ptr }
    }

    pub fn open(filename: &str) -> Result<Self, Error> {
        let c_name = CString::new(filename)?;
        let inner = unsafe { ffi_try!(mupdf_open_document(context(), c_name.as_ptr())) };
        Ok(Self { inner })
    }

    pub fn from_bytes(bytes: &[u8], magic: &str) -> Result<Self, Error> {
        let c_magic = CString::new(magic)?;
        let len = bytes.len();
        let mut buf = Buffer::with_capacity(len);
        buf.write_all(bytes)?;
        let inner = unsafe {
            ffi_try!(mupdf_open_document_from_bytes(
                context(),
                buf.inner,
                c_magic.as_ptr()
            ))
        };
        Ok(Self { inner })
    }

    pub fn recognize(magic: &str) -> Result<bool, Error> {
        let c_magic = CString::new(magic)?;
        let ret = unsafe { ffi_try!(mupdf_recognize_document(context(), c_magic.as_ptr())) };
        Ok(ret)
    }

    pub fn needs_password(&self) -> Result<bool, Error> {
        let ret = unsafe { ffi_try!(mupdf_needs_password(context(), self.inner)) };
        Ok(ret)
    }

    pub fn authenticate(&mut self, password: &str) -> Result<bool, Error> {
        let c_pass = CString::new(password)?;
        let ret = unsafe {
            ffi_try!(mupdf_authenticate_password(
                context(),
                self.inner,
                c_pass.as_ptr()
            ))
        };
        Ok(ret)
    }

    pub fn page_count(&self) -> Result<i32, Error> {
        let count = unsafe { ffi_try!(mupdf_document_page_count(context(), self.inner)) };
        Ok(count)
    }

    pub fn metadata(&self, name: MetadataName) -> Result<String, Error> {
        let c_key = CString::new(name.to_str())?;
        let info_ptr =
            unsafe { ffi_try!(mupdf_lookup_metadata(context(), self.inner, c_key.as_ptr())) };
        if info_ptr.is_null() {
            return Ok(String::new());
        }
        let c_info = unsafe { CStr::from_ptr(info_ptr) };
        let info = c_info.to_string_lossy().into_owned();
        unsafe {
            mupdf_drop_str(info_ptr);
        }
        Ok(info)
    }

    pub fn resolve_link(&self, uri: &str) -> Result<Option<Location>, Error> {
        let c_uri = CString::new(uri)?;
        let loc = unsafe { ffi_try!(mupdf_resolve_link(context(), self.inner, c_uri.as_ptr())) };
        if loc.page >= 0 {
            return Ok(Some(Location {
                chapter: loc.chapter,
                page: loc.page,
            }));
        }
        Ok(None)
    }

    pub fn is_reflowable(&self) -> Result<bool, Error> {
        let ret = unsafe { ffi_try!(mupdf_is_document_reflowable(context(), self.inner)) };
        Ok(ret)
    }

    pub fn is_pdf(&self) -> bool {
        let pdf = unsafe { pdf_specifics(context(), self.inner) };
        if !pdf.is_null() {
            return true;
        }
        false
    }

    pub fn convert_to_pdf(
        &self,
        start_page: i32,
        end_page: i32,
        rotate: u32,
    ) -> Result<PdfDocument, Error> {
        self.convert_to_pdf_internal(start_page, end_page, rotate, None)
    }

    pub fn convert_to_pdf_with_cookie(
        &self,
        start_page: i32,
        end_page: i32,
        rotate: u32,
        cookie: &Cookie,
    ) -> Result<PdfDocument, Error> {
        self.convert_to_pdf_internal(start_page, end_page, rotate, Some(cookie))
    }

    fn convert_to_pdf_internal(
        &self,
        start_page: i32,
        end_page: i32,
        rotate: u32,
        cookie: Option<&Cookie>,
    ) -> Result<PdfDocument, Error> {
        let page_count = self.page_count()? as i32;
        let start_page = if start_page > page_count - 1 {
            page_count - 1
        } else {
            start_page
        };
        let end_page = if end_page > page_count - 1 || end_page < 0 {
            page_count - 1
        } else {
            end_page
        };
        unsafe {
            let cookie_ptr = if let Some(ck) = cookie {
                ck.inner
            } else {
                ptr::null_mut()
            };
            let inner = ffi_try!(mupdf_convert_to_pdf(
                context(),
                self.inner,
                start_page as _,
                end_page as _,
                rotate as _,
                cookie_ptr
            ));
            Ok(PdfDocument::from_raw(inner))
        }
    }

    pub fn layout(&mut self, width: f32, height: f32, em: f32) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_layout_document(
                context(),
                self.inner,
                width,
                height,
                em
            ));
        }
        Ok(())
    }

    pub fn load_page(&self, page_no: i32) -> Result<Page, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_load_page(context(), self.inner, page_no));
            Ok(Page::from_raw(inner))
        }
    }

    pub fn pages(&self) -> Result<PageIter, Error> {
        Ok(PageIter {
            index: 0,
            total: self.page_count()?,
            doc: self,
        })
    }

    pub fn output_intent(&self) -> Result<Option<Colorspace>, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_document_output_intent(context(), self.inner));
            if inner.is_null() {
                return Ok(None);
            }
            Ok(Some(Colorspace::from_raw(inner)))
        }
    }

    unsafe fn walk_outlines(&self, outline: *mut fz_outline) -> Vec<Outline> {
        let mut outlines = Vec::new();
        let mut next = outline;
        while !next.is_null() {
            let mut x = 0.0;
            let mut y = 0.0;
            let mut page = None;
            let title = CStr::from_ptr((*next).title).to_string_lossy().into_owned();
            let uri = if !(*next).uri.is_null() {
                if fz_is_external_link(context(), (*next).uri) > 0 {
                    Some(CStr::from_ptr((*next).uri).to_string_lossy().into_owned())
                } else {
                    page = Some(
                        fz_resolve_link(context(), self.inner, (*next).uri, &mut x, &mut y).page
                            as u32,
                    );
                    None
                }
            } else {
                None
            };
            let down = if !(*next).down.is_null() {
                self.walk_outlines((*next).down)
            } else {
                Vec::new()
            };
            outlines.push(Outline {
                title,
                uri,
                page,
                down,
                x,
                y,
            });
            next = (*next).next;
        }
        outlines
    }

    pub fn outlines(&self) -> Result<Vec<Outline>, Error> {
        let outline = unsafe { ffi_try!(mupdf_load_outline(context(), self.inner)) };
        if outline.is_null() {
            return Ok(Vec::new());
        }
        unsafe {
            let toc = self.walk_outlines(outline);
            fz_drop_outline(context(), outline);
            Ok(toc)
        }
    }
}

impl Drop for Document {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                fz_drop_document(context(), self.inner);
            }
        }
    }
}

impl Clone for Document {
    fn clone(&self) -> Self {
        unsafe { Document::from_raw(fz_keep_document(context(), self.inner)) }
    }
}

#[derive(Debug)]
pub struct PageIter<'a> {
    index: i32,
    total: i32,
    doc: &'a Document,
}

impl Iterator for PageIter<'_> {
    type Item = Result<Page, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.total {
            return None;
        }
        let page = self.doc.load_page(self.index);
        self.index += 1;
        Some(page)
    }
}

impl<'a> IntoIterator for &'a Document {
    type Item = Result<Page, Error>;
    type IntoIter = PageIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.pages().unwrap()
    }
}

impl<'a> IntoIterator for &'a mut Document {
    type Item = Result<Page, Error>;
    type IntoIter = PageIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.pages().unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::{Document, MetadataName, Page};

    #[test]
    fn test_recognize_document() {
        assert!(Document::recognize("test.pdf").unwrap());
        assert!(Document::recognize("application/pdf").unwrap());
        assert!(Document::recognize("text/html").unwrap());

        assert!(!Document::recognize("test.doc").unwrap());
    }

    #[test]
    fn test_document_open_html() {
        let doc = Document::open("tests/files/dummy.html").unwrap();
        assert!(!doc.is_pdf());
    }

    #[test]
    fn test_document_load_page() {
        let doc = Document::open("tests/files/dummy.pdf").unwrap();
        assert!(doc.is_pdf());
        assert_eq!(doc.page_count().unwrap(), 1);

        let page0 = doc.load_page(0).unwrap();
        let bounds = page0.bounds().unwrap();
        assert_eq!(bounds.x0, 0.0);
        assert_eq!(bounds.y0, 0.0);
        assert_eq!(bounds.x1, 595.0);
        assert_eq!(bounds.y1, 842.0);

        let cs = doc.output_intent().unwrap();
        assert!(cs.is_none());
    }

    #[test]
    fn test_encrypted_document_load_page() {
        let mut doc = Document::open("tests/files/dummy-encrypted.pdf").unwrap();
        assert!(doc.is_pdf());
        assert!(doc.needs_password().unwrap());
        // Before authentication, no outlines
        let outlines = doc.outlines().unwrap();
        assert_eq!(outlines.len(), 0);
        doc.authenticate("123456").unwrap();
        // After authentication, can read outlines
        let outlines = doc.outlines().unwrap();
        assert_eq!(outlines.len(), 0);

        assert_eq!(doc.page_count().unwrap(), 1);
        let page0 = doc.load_page(0).unwrap();
        let bounds = page0.bounds().unwrap();
        assert_eq!(bounds.x0, 0.0);
        assert_eq!(bounds.y0, 0.0);
        assert_eq!(bounds.x1, 595.0);
        assert_eq!(bounds.y1, 842.0);
    }

    #[test]
    fn test_document_page_iterator() {
        let doc = Document::open("tests/files/dummy.pdf").unwrap();
        let pages: Result<Vec<Page>, _> = doc.into_iter().collect();
        let pages = pages.unwrap();
        assert_eq!(pages.len(), 1);
        let page0 = &pages[0];
        let bounds = page0.bounds().unwrap();
        assert_eq!(bounds.x0, 0.0);
        assert_eq!(bounds.y0, 0.0);
        assert_eq!(bounds.x1, 595.0);
        assert_eq!(bounds.y1, 842.0);
    }

    #[test]
    fn test_document_metadata() {
        let doc = Document::open("tests/files/dummy.pdf").unwrap();

        let format = doc.metadata(MetadataName::Format).unwrap();
        assert_eq!(format, "PDF 1.4");
        let encryption = doc.metadata(MetadataName::Encryption).unwrap();
        assert_eq!(encryption, "None");
        let author = doc.metadata(MetadataName::Author).unwrap();
        assert_eq!(author, "Evangelos Vlachogiannis");
        let title = doc.metadata(MetadataName::Title).unwrap();
        assert!(title.is_empty());
        let producer = doc.metadata(MetadataName::Producer).unwrap();
        assert_eq!(producer, "OpenOffice.org 2.1");
        let creator = doc.metadata(MetadataName::Creator).unwrap();
        assert_eq!(creator, "Writer");
        let creation_date = doc.metadata(MetadataName::CreationDate).unwrap();
        // FIXME: parse Date format
        assert_eq!(creation_date, "D:20070223175637+02'00'");
        let mod_date = doc.metadata(MetadataName::ModDate).unwrap();
        assert!(mod_date.is_empty());
        let subject = doc.metadata(MetadataName::Subject).unwrap();
        assert!(subject.is_empty());
        let keywords = doc.metadata(MetadataName::Keywords).unwrap();
        assert!(keywords.is_empty());
    }

    #[test]
    fn test_document_outlines() {
        let doc = Document::open("tests/files/dummy.pdf").unwrap();
        let outlines = doc.outlines().unwrap();
        assert_eq!(outlines.len(), 1);

        let out1 = &outlines[0];
        assert_eq!(out1.page, Some(0));
        assert_eq!(out1.title, "Dummy PDF file");
        assert!(out1.uri.is_none());
        assert_eq!(out1.x, 56.7);
        assert_eq!(out1.y, 68.70001);
    }
}

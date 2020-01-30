use std::ffi::{CStr, CString};
use std::io::Write;
use std::os::raw::{c_char, c_int};

use mupdf_sys::*;

use crate::{context, Buffer, Error, Page};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetaDataType {
    Format,
    Encryption,
    Author,
    Title,
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
        buf.write(bytes)?;
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

    pub fn authenticate_password(&mut self, password: &str) -> Result<bool, Error> {
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

    pub fn page_count(&self) -> Result<usize, Error> {
        let count = unsafe { ffi_try!(mupdf_document_page_count(context(), self.inner)) };
        Ok(count as usize)
    }

    pub fn metadata(&self, typ: MetaDataType) -> Result<String, Error> {
        let key = match typ {
            MetaDataType::Format => "format",
            MetaDataType::Encryption => "encryption",
            MetaDataType::Author => "info:Author",
            MetaDataType::Title => "info::Title",
        };
        let c_key = CString::new(key)?;
        const INFO_LEN: usize = 256;
        let mut info: [c_char; INFO_LEN] = [0; INFO_LEN];
        unsafe {
            ffi_try!(mupdf_lookup_metadata(
                context(),
                self.inner,
                c_key.as_ptr(),
                info.as_mut_ptr(),
                INFO_LEN as c_int
            ));
        }
        let c_info = unsafe { CStr::from_ptr(info.as_ptr()) };
        Ok(c_info.to_string_lossy().into_owned())
    }

    pub fn is_reflowable(&self) -> Result<bool, Error> {
        let ret = unsafe { ffi_try!(mupdf_is_document_reflowable(context(), self.inner)) };
        Ok(ret)
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

#[cfg(test)]
mod test {
    use super::Document;

    #[test]
    fn test_recognize_document() {
        assert!(Document::recognize("test.pdf").unwrap());
        assert!(Document::recognize("application/pdf").unwrap());

        assert!(!Document::recognize("test.doc").unwrap());
        assert!(!Document::recognize("text/html").unwrap());
    }

    #[test]
    fn test_document_load_page() {
        let doc = Document::open("tests/files/dummy.pdf").unwrap();
        assert_eq!(doc.page_count().unwrap(), 1);
        let page0 = doc.load_page(0).unwrap();
        let bounds = page0.bounds().unwrap();
        assert_eq!(bounds.x0, 0.0);
        assert_eq!(bounds.y0, 0.0);
        assert_eq!(bounds.x1, 595.0);
        assert_eq!(bounds.y1, 842.0);
    }
}

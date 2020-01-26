use std::ffi::{CStr, CString};
use std::io::Write;
use std::os::raw::{c_char, c_int};

use mupdf_sys::*;

use crate::{context, Buffer, Error};

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
    pub fn open(filename: &str) -> Result<Self, Error> {
        let c_name = CString::new(filename).unwrap();
        let inner = unsafe { ffi_try!(mupdf_open_document(context(), c_name.as_ptr())) };
        Ok(Self { inner })
    }

    pub fn from_bytes(bytes: &[u8], magic: &str) -> Result<Self, Error> {
        let c_magic = CString::new(magic).unwrap();
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
        let c_magic = CString::new(magic).unwrap();
        let ret = unsafe { ffi_try!(mupdf_recognize_document(context(), c_magic.as_ptr())) };
        Ok(ret)
    }

    pub fn needs_password(&self) -> Result<bool, Error> {
        let ret = unsafe { ffi_try!(mupdf_needs_password(context(), self.inner)) };
        Ok(ret)
    }

    pub fn authenticate_password(&mut self, password: &str) -> Result<bool, Error> {
        let c_pass = CString::new(password).unwrap();
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
        let c_key = CString::new(key).unwrap();
        const info_len: usize = 256;
        let mut info: [c_char; info_len] = [0; info_len];
        unsafe {
            ffi_try!(mupdf_lookup_metadata(
                context(),
                self.inner,
                c_key.as_ptr(),
                info.as_mut_ptr(),
                info_len as c_int
            ));
        }
        let c_info = unsafe { CStr::from_ptr(info.as_ptr()) };
        Ok(c_info.to_string_lossy().into_owned())
    }
}

impl Drop for Document {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                fz_drop_document(context(), self.inner);
                // This is a reasonable place to call Memento.
                Memento_fin();
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
}

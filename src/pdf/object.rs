use std::convert::TryFrom;
use std::ffi::{CStr, CString};
use std::fmt;
use std::io::{self, BufReader, Read, Write};
use std::slice;

use mupdf_sys::*;

use crate::{context, Buffer, Error};

pub trait IntoPdfDictKey {
    fn into_pdf_dict_key(self) -> Result<PdfObject, Error>;
}

impl IntoPdfDictKey for &str {
    fn into_pdf_dict_key(self) -> Result<PdfObject, Error> {
        PdfObject::new_name(self)
    }
}

impl IntoPdfDictKey for String {
    fn into_pdf_dict_key(self) -> Result<PdfObject, Error> {
        PdfObject::new_name(&self)
    }
}

impl IntoPdfDictKey for PdfObject {
    fn into_pdf_dict_key(self) -> Result<PdfObject, Error> {
        Ok(self)
    }
}

#[derive(Debug)]
pub struct PdfObject {
    pub(crate) inner: *mut pdf_obj,
    owned: bool,
}

impl PdfObject {
    pub(crate) unsafe fn from_raw(ptr: *mut pdf_obj, owned: bool) -> Self {
        Self { inner: ptr, owned }
    }

    pub fn try_clone(&self) -> Result<Self, Error> {
        let inner = unsafe { ffi_try!(mupdf_pdf_clone_obj(context(), self.inner)) };
        Ok(Self { inner, owned: true })
    }

    pub fn new_null() -> PdfObject {
        unsafe {
            let inner = mupdf_pdf_new_null();
            PdfObject::from_raw(inner, true)
        }
    }

    pub fn new_bool(b: bool) -> PdfObject {
        unsafe {
            let inner = mupdf_pdf_new_bool(b);
            PdfObject::from_raw(inner, true)
        }
    }

    pub fn new_int(i: i32) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_new_int(context(), i));
            Ok(PdfObject::from_raw(inner, true))
        }
    }

    pub fn new_real(f: f32) -> Result<PdfObject, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_pdf_new_real(context(), f));
            Ok(PdfObject::from_raw(inner, true))
        }
    }

    pub fn new_string(s: &str) -> Result<PdfObject, Error> {
        let c_str = CString::new(s)?;
        unsafe {
            let inner = ffi_try!(mupdf_pdf_new_string(context(), c_str.as_ptr()));
            Ok(PdfObject::from_raw(inner, true))
        }
    }

    pub fn new_name(name: &str) -> Result<PdfObject, Error> {
        let c_name = CString::new(name)?;
        unsafe {
            let inner = ffi_try!(mupdf_pdf_new_name(context(), c_name.as_ptr()));
            Ok(PdfObject::from_raw(inner, true))
        }
    }

    pub fn is_indirect(&self) -> Result<bool, Error> {
        let ret = unsafe { ffi_try!(mupdf_pdf_is_indirect(context(), self.inner)) };
        Ok(ret)
    }

    pub fn is_null(&self) -> Result<bool, Error> {
        let ret = unsafe { ffi_try!(mupdf_pdf_is_null(context(), self.inner)) };
        Ok(ret)
    }

    pub fn is_bool(&self) -> Result<bool, Error> {
        let ret = unsafe { ffi_try!(mupdf_pdf_is_bool(context(), self.inner)) };
        Ok(ret)
    }

    pub fn is_int(&self) -> Result<bool, Error> {
        let ret = unsafe { ffi_try!(mupdf_pdf_is_int(context(), self.inner)) };
        Ok(ret)
    }

    pub fn is_real(&self) -> Result<bool, Error> {
        let ret = unsafe { ffi_try!(mupdf_pdf_is_real(context(), self.inner)) };
        Ok(ret)
    }

    pub fn is_number(&self) -> Result<bool, Error> {
        let ret = unsafe { ffi_try!(mupdf_pdf_is_number(context(), self.inner)) };
        Ok(ret)
    }

    pub fn is_string(&self) -> Result<bool, Error> {
        let ret = unsafe { ffi_try!(mupdf_pdf_is_string(context(), self.inner)) };
        Ok(ret)
    }

    pub fn is_name(&self) -> Result<bool, Error> {
        let ret = unsafe { ffi_try!(mupdf_pdf_is_name(context(), self.inner)) };
        Ok(ret)
    }

    pub fn is_array(&self) -> Result<bool, Error> {
        let ret = unsafe { ffi_try!(mupdf_pdf_is_array(context(), self.inner)) };
        Ok(ret)
    }

    pub fn is_dict(&self) -> Result<bool, Error> {
        let ret = unsafe { ffi_try!(mupdf_pdf_is_dict(context(), self.inner)) };
        Ok(ret)
    }

    pub fn is_stream(&self) -> Result<bool, Error> {
        let ret = unsafe { ffi_try!(mupdf_pdf_is_stream(context(), self.inner)) };
        Ok(ret)
    }

    pub fn as_bool(&self) -> Result<bool, Error> {
        let ret = unsafe { ffi_try!(mupdf_pdf_to_bool(context(), self.inner)) };
        Ok(ret)
    }

    pub fn as_int(&self) -> Result<i32, Error> {
        let ret = unsafe { ffi_try!(mupdf_pdf_to_int(context(), self.inner)) };
        Ok(ret)
    }

    pub fn as_float(&self) -> Result<f32, Error> {
        let ret = unsafe { ffi_try!(mupdf_pdf_to_float(context(), self.inner)) };
        Ok(ret)
    }

    pub fn as_indirect(&self) -> Result<i32, Error> {
        let ret = unsafe { ffi_try!(mupdf_pdf_to_indirect(context(), self.inner)) };
        Ok(ret)
    }

    pub fn as_name(&self) -> Result<&str, Error> {
        unsafe {
            let name_ptr = ffi_try!(mupdf_pdf_to_name(context(), self.inner));
            let c_name = CStr::from_ptr(name_ptr);
            let name = c_name.to_str().unwrap();
            Ok(name)
        }
    }

    pub fn as_string(&self) -> Result<&str, Error> {
        unsafe {
            let str_ptr = ffi_try!(mupdf_pdf_to_string(context(), self.inner));
            let c_str = CStr::from_ptr(str_ptr);
            let string = c_str.to_str().unwrap();
            Ok(string)
        }
    }

    pub fn as_bytes(&self) -> Result<&[u8], Error> {
        let mut len = 0;
        unsafe {
            let ptr = ffi_try!(mupdf_pdf_to_bytes(context(), self.inner, &mut len));
            let byte_slice = slice::from_raw_parts(ptr, len);
            Ok(byte_slice)
        }
    }

    pub fn resolve(&self) -> Result<Option<Self>, Error> {
        let inner = unsafe { ffi_try!(mupdf_pdf_resolve_indirect(context(), self.inner)) };
        if inner.is_null() {
            return Ok(None);
        }
        Ok(Some(Self { inner, owned: true }))
    }

    pub fn read_stream(&self) -> Result<Vec<u8>, Error> {
        let inner = unsafe { ffi_try!(mupdf_pdf_read_stream(context(), self.inner)) };
        let buf = unsafe { Buffer::from_raw(inner) };
        let buf_len = buf.len();
        let mut reader = BufReader::new(buf);
        let mut output = Vec::with_capacity(buf_len);
        reader.read_to_end(&mut output)?;
        Ok(output)
    }

    pub fn read_raw_stream(&self) -> Result<Vec<u8>, Error> {
        let inner = unsafe { ffi_try!(mupdf_pdf_read_raw_stream(context(), self.inner)) };
        let buf = unsafe { Buffer::from_raw(inner) };
        let buf_len = buf.len();
        let mut reader = BufReader::new(buf);
        let mut output = Vec::with_capacity(buf_len);
        reader.read_to_end(&mut output)?;
        Ok(output)
    }

    pub fn write_object(&mut self, obj: &PdfObject) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_write_object(context(), self.inner, obj.inner));
        }
        Ok(())
    }

    pub fn write_stream_buffer(&mut self, buf: &Buffer) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_write_stream_buffer(
                context(),
                self.inner,
                buf.inner,
                0
            ));
        }
        Ok(())
    }

    pub fn write_stream_string(&mut self, string: &str) -> Result<(), Error> {
        let buf = Buffer::from_str(string)?;
        self.write_stream_buffer(&buf)
    }

    pub fn write_raw_stream_buffer(&mut self, buf: &Buffer) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_write_stream_buffer(
                context(),
                self.inner,
                buf.inner,
                1
            ));
        }
        Ok(())
    }

    pub fn write_raw_stream_string(&mut self, string: &str) -> Result<(), Error> {
        let buf = Buffer::from_str(string)?;
        self.write_raw_stream_buffer(&buf)
    }

    pub fn get_array(&self, index: i32) -> Result<Option<Self>, Error> {
        let inner = unsafe { ffi_try!(mupdf_pdf_array_get(context(), self.inner, index)) };
        if inner.is_null() {
            return Ok(None);
        }
        Ok(Some(Self { inner, owned: true }))
    }

    pub fn get_dict<K: IntoPdfDictKey>(&self, key: K) -> Result<Option<Self>, Error> {
        let key = key.into_pdf_dict_key()?;
        let inner = unsafe { ffi_try!(mupdf_pdf_dict_get(context(), self.inner, key.inner)) };
        if inner.is_null() {
            return Ok(None);
        }
        Ok(Some(Self { inner, owned: true }))
    }

    pub fn get_dict_inheritable<K: IntoPdfDictKey>(&self, key: K) -> Result<Option<Self>, Error> {
        let key = key.into_pdf_dict_key()?;
        let inner = unsafe {
            ffi_try!(mupdf_pdf_dict_get_inheritable(
                context(),
                self.inner,
                key.inner
            ))
        };
        if inner.is_null() {
            return Ok(None);
        }
        Ok(Some(Self { inner, owned: true }))
    }

    pub fn len(&self) -> Result<usize, Error> {
        let size = unsafe { ffi_try!(mupdf_pdf_array_len(context(), self.inner)) };
        Ok(size as usize)
    }

    pub fn array_put(&mut self, index: i32, value: Self) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_array_put(
                context(),
                self.inner,
                index,
                value.inner
            ));
        }
        Ok(())
    }

    pub fn array_push(&mut self, value: Self) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_array_push(context(), self.inner, value.inner));
        }
        Ok(())
    }

    pub fn array_delete(&mut self, index: i32) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_array_delete(context(), self.inner, index));
        }
        Ok(())
    }

    pub fn dict_put<K: IntoPdfDictKey>(&mut self, key: K, value: Self) -> Result<(), Error> {
        let key_obj = key.into_pdf_dict_key()?;
        unsafe {
            ffi_try!(mupdf_pdf_dict_put(
                context(),
                self.inner,
                key_obj.inner,
                value.inner
            ));
        }
        Ok(())
    }

    pub fn dict_delete<K: IntoPdfDictKey>(&mut self, key: K) -> Result<(), Error> {
        let key_obj = key.into_pdf_dict_key()?;
        unsafe {
            ffi_try!(mupdf_pdf_dict_delete(context(), self.inner, key_obj.inner));
        }
        Ok(())
    }

    fn print(&self, tight: bool, ascii: bool) -> Result<String, Error> {
        unsafe {
            let ptr = ffi_try!(mupdf_pdf_obj_to_string(context(), self.inner, tight, ascii));
            let c_str = CStr::from_ptr(ptr);
            let s = c_str.to_string_lossy().into_owned();
            fz_free(context(), ptr as _);
            Ok(s)
        }
    }
}

impl Write for PdfObject {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = buf.len();
        let mut fz_buf = Buffer::with_capacity(len);
        fz_buf.write(buf)?;
        self.write_stream_buffer(&fz_buf)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for PdfObject {
    fn drop(&mut self) {
        if self.owned && !self.inner.is_null() {
            unsafe {
                pdf_drop_obj(context(), self.inner);
            }
        }
    }
}

impl Clone for PdfObject {
    fn clone(&self) -> PdfObject {
        self.try_clone().unwrap()
    }
}

impl fmt::Display for PdfObject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = self.print(true, false).unwrap();
        write!(f, "{}", s)
    }
}

impl From<bool> for PdfObject {
    fn from(b: bool) -> PdfObject {
        PdfObject::new_bool(b)
    }
}

impl TryFrom<i32> for PdfObject {
    type Error = Error;

    fn try_from(i: i32) -> Result<PdfObject, Self::Error> {
        PdfObject::new_int(i)
    }
}

impl TryFrom<f32> for PdfObject {
    type Error = Error;

    fn try_from(f: f32) -> Result<PdfObject, Self::Error> {
        PdfObject::new_real(f)
    }
}

impl TryFrom<&str> for PdfObject {
    type Error = Error;

    fn try_from(s: &str) -> Result<PdfObject, Self::Error> {
        PdfObject::new_string(s)
    }
}

impl TryFrom<String> for PdfObject {
    type Error = Error;

    fn try_from(s: String) -> Result<PdfObject, Self::Error> {
        PdfObject::new_string(&s)
    }
}

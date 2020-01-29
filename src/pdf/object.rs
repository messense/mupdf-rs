use std::ffi::{CStr, CString};
use std::io::{self, BufReader, Read, Write};

use mupdf_sys::*;

use crate::{context, Buffer, Error};

#[derive(Debug)]
pub struct PdfObject {
    pub(crate) inner: *mut pdf_obj,
}

impl PdfObject {
    pub(crate) unsafe fn from_raw(ptr: *mut pdf_obj) -> Self {
        Self { inner: ptr }
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

    pub fn as_name(&self) -> Result<String, Error> {
        unsafe {
            let name_ptr = ffi_try!(mupdf_pdf_to_name(context(), self.inner));
            let c_name = CStr::from_ptr(name_ptr);
            let name = c_name.to_string_lossy().into_owned();
            mupdf_drop_str(name_ptr);
            Ok(name)
        }
    }

    pub fn as_string(&self) -> Result<String, Error> {
        unsafe {
            let str_ptr = ffi_try!(mupdf_pdf_to_string(context(), self.inner));
            let c_str = CStr::from_ptr(str_ptr);
            let string = c_str.to_string_lossy().into_owned();
            mupdf_drop_str(str_ptr);
            Ok(string)
        }
    }

    pub fn as_bytes(&self) -> Result<Vec<u8>, Error> {
        todo!()
    }

    pub fn resolve(&self) -> Result<Option<Self>, Error> {
        let inner = unsafe { ffi_try!(mupdf_pdf_resolve_indirect(context(), self.inner)) };
        if inner.is_null() {
            return Ok(None);
        }
        Ok(Some(Self { inner }))
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
        Ok(Some(Self { inner }))
    }

    pub fn get_dict(&self, key: &str) -> Result<Option<Self>, Error> {
        let c_key = CString::new(key)?;
        let inner = unsafe { ffi_try!(mupdf_pdf_dict_get(context(), self.inner, c_key.as_ptr())) };
        if inner.is_null() {
            return Ok(None);
        }
        Ok(Some(Self { inner }))
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
        if !self.inner.is_null() {
            unsafe {
                pdf_drop_obj(context(), self.inner);
            }
        }
    }
}

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
        todo!()
    }

    pub fn as_int(&self) -> Result<i32, Error> {
        todo!()
    }

    pub fn as_float(&self) -> Result<f32, Error> {
        todo!()
    }

    pub fn as_indirect(&self) -> Result<i32, Error> {
        todo!()
    }

    pub fn as_name(&self) -> Result<String, Error> {
        todo!()
    }

    pub fn as_string(&self) -> Result<String, Error> {
        todo!()
    }

    pub fn as_bytes(&self) -> Result<Vec<u8>, Error> {
        todo!()
    }

    pub fn to_string(&self, tight: bool, ascii: bool) -> Result<String, Error> {
        todo!()
    }

    pub fn resolve(&self) -> Result<Self, Error> {
        todo!()
    }

    pub fn read_stream(&self) -> Result<Vec<u8>, Error> {
        todo!()
    }

    pub fn read_raw_stream(&self) -> Result<Vec<u8>, Error> {
        todo!()
    }

    pub fn write_object(&mut self, obj: &PdfObject) -> Result<(), Error> {
        todo!()
    }

    pub fn write_stream_buffer(&mut self, buf: &Buffer) -> Result<(), Error> {
        todo!()
    }

    pub fn write_stream_string(&mut self, string: &str) -> Result<(), Error> {
        todo!()
    }

    pub fn write_raw_stream_buffer(&mut self, buf: &Buffer) -> Result<(), Error> {
        todo!()
    }

    pub fn write_raw_stream_string(&mut self, string: &str) -> Result<(), Error> {
        todo!()
    }

    pub fn get_array(&self, index: i32) -> Result<Option<Self>, Error> {
        todo!()
    }

    pub fn get_dictionary(&self, name: &str) -> Result<Option<Self>, Error> {
        todo!()
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

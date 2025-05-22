use std::convert::TryFrom;
use std::ffi::CString;
use std::io;
use std::ptr;
use std::str::FromStr;

use mupdf_sys::*;

use crate::{context, Error};

/// A wrapper around a dynamically allocated array of bytes.
/// Buffers have a capacity (the number of bytes storage immediately available) and a current size.
#[derive(Debug)]
pub struct Buffer {
    pub(crate) inner: *mut fz_buffer,
    offset: usize,
}

impl FromStr for Buffer {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let c_str = CString::new(s)?;
        unsafe { ffi_try!(mupdf_buffer_from_str(context(), c_str.as_ptr())) }
            .map(|inner| Self { inner, offset: 0 })
    }
}

impl Buffer {
    pub(crate) unsafe fn from_raw(ptr: *mut fz_buffer) -> Self {
        Self {
            inner: ptr,
            offset: 0,
        }
    }

    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    pub fn from_base64(str: &str) -> Result<Self, Error> {
        let c_str = CString::new(str)?;
        unsafe { ffi_try!(mupdf_buffer_from_base64(context(), c_str.as_ptr())) }
            .map(|inner| Self { inner, offset: 0 })
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let mut buf = Buffer::with_capacity(bytes.len());
        buf.write_bytes(bytes)?;
        Ok(buf)
    }

    pub fn with_capacity(cap: usize) -> Self {
        let inner = unsafe { fz_new_buffer(context(), cap) };
        Self { inner, offset: 0 }
    }

    pub fn len(&self) -> usize {
        unsafe { fz_buffer_storage(context(), self.inner, ptr::null_mut()) }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn into_inner(mut self) -> *mut fz_buffer {
        let inner = self.inner;
        self.inner = ptr::null_mut();
        inner
    }

    fn read_bytes(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        let len = buf.len();
        let read_len = unsafe {
            ffi_try!(mupdf_buffer_read_bytes(
                context(),
                self.inner,
                self.offset,
                buf.as_mut_ptr(),
                len
            ))
        }?;
        self.offset += read_len as usize;
        Ok(read_len)
    }

    fn write_bytes(&mut self, buf: &[u8]) -> Result<usize, Error> {
        let len = buf.len();
        unsafe {
            ffi_try!(mupdf_buffer_write_bytes(
                context(),
                self.inner,
                buf.as_ptr(),
                len
            ))
        }?;
        Ok(len)
    }
}

impl io::Read for Buffer {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.read_bytes(buf).map_err(io::Error::other)
    }
}

impl io::Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_bytes(buf).map_err(io::Error::other)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                fz_drop_buffer(context(), self.inner);
            }
        }
    }
}

impl TryFrom<&str> for Buffer {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Buffer::from_str(s)
    }
}

impl TryFrom<String> for Buffer {
    type Error = Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Buffer::from_str(&s)
    }
}

impl TryFrom<&[u8]> for Buffer {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        Buffer::from_bytes(bytes)
    }
}

impl TryFrom<Vec<u8>> for Buffer {
    type Error = Error;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        Buffer::from_bytes(&bytes)
    }
}

#[cfg(test)]
mod test {
    use super::Buffer;
    use std::{
        io::{Read, Write},
        str::FromStr,
    };

    #[test]
    fn test_buffer_len() {
        let buf = Buffer::new();
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn test_buffer_read_write() {
        let mut buf = Buffer::new();
        let n = buf.write("abc".as_bytes()).unwrap();
        assert_eq!(n, 3);

        let mut output = [0; 3];
        buf.read_exact(&mut output).unwrap();
        assert_eq!(output, [97, 98, 99]);
    }

    #[test]
    fn test_buffer_read_to_string() {
        let mut buf = Buffer::new();
        let n = buf.write("abc".as_bytes()).unwrap();
        assert_eq!(n, 3);

        let mut output = String::new();
        buf.read_to_string(&mut output).unwrap();
        assert_eq!(output, "abc");
    }

    #[test]
    fn test_buffer_read_to_end() {
        let mut buf = Buffer::new();
        let n = buf.write("abc".as_bytes()).unwrap();
        assert_eq!(n, 3);

        let mut output = Vec::new();
        buf.read_to_end(&mut output).unwrap();
        assert_eq!(output, [97, 98, 99]);
    }

    #[test]
    fn test_buffer_read_exact() {
        let mut buf = Buffer::new();
        let n = buf.write("abc".as_bytes()).unwrap();
        assert_eq!(n, 3);

        let mut output = [0; 2];
        buf.read_exact(&mut output).unwrap();
        assert_eq!(output, [97, 98]);

        let mut output = [0; 1];
        buf.read_exact(&mut output).unwrap();
        assert_eq!(output, [99]);

        let mut output = [0; 1];
        assert_eq!(buf.read(&mut output).unwrap(), 0);
    }

    #[test]
    #[allow(clippy::unbuffered_bytes)]
    fn test_buffer_as_bytes() {
        let mut buf = Buffer::new();
        let n = buf.write("abc".as_bytes()).unwrap();
        assert_eq!(n, 3);

        let bytes = buf.bytes().collect::<Result<Vec<u8>, _>>();
        assert_eq!(bytes.unwrap(), [97, 98, 99]);
    }

    #[test]
    fn test_buffer_from_str() {
        let mut buf = Buffer::from_str("abc").unwrap();
        let mut output = [0; 3];
        buf.read_exact(&mut output).unwrap();
        assert_eq!(output, [97, 98, 99]);
    }

    #[test]
    fn test_buffer_from_base64() {
        let mut buf = Buffer::from_base64("YWJj").unwrap();
        let mut output = [0; 3];
        buf.read_exact(&mut output).unwrap();
        assert_eq!(output, [97, 98, 99]);
    }
}

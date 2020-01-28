use std::io;
use std::ptr;

use mupdf_sys::*;

use crate::{context, Error};

#[derive(Debug)]
pub struct Buffer {
    pub(crate) inner: *mut fz_buffer,
    offset: usize,
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

    pub fn with_capacity(cap: usize) -> Self {
        let inner = unsafe { fz_new_buffer(context(), cap) };
        Self { inner, offset: 0 }
    }

    pub fn len(&self) -> usize {
        unsafe { fz_buffer_storage(context(), self.inner, ptr::null_mut()) }
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
        };
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
        };
        Ok(len)
    }
}

impl io::Read for Buffer {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.read_bytes(buf)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }
}

impl io::Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_bytes(buf)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
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

#[cfg(test)]
mod test {
    use super::Buffer;
    use std::io::{Read, Write};

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
        buf.read(&mut output).unwrap();
        assert_eq!(output, [97, 98, 99]);
    }
}

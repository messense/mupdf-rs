use std::ffi::CString;
use std::ptr;

use mupdf_sys::*;

use crate::{context, Device, Error, FilePath, Rect};

#[derive(Debug)]
pub struct DocumentWriter {
    inner: *mut fz_document_writer,
}

impl DocumentWriter {
    pub fn new<P: AsRef<FilePath> + ?Sized>(
        filename: &P,
        format: &str,
        options: &str,
    ) -> Result<Self, Error> {
        let c_filename = CString::new(filename.as_ref().as_bytes())?;
        let c_format = CString::new(format)?;
        let c_options = CString::new(options)?;
        unsafe {
            ffi_try!(mupdf_new_document_writer(
                context(),
                c_filename.as_ptr(),
                c_format.as_ptr(),
                c_options.as_ptr()
            ))
        }
        .map(|inner| Self { inner })
    }

    #[cfg(feature = "tesseract")]
    pub fn new_pdfocr_writer<P: AsRef<FilePath> + ?Sized>(
        path: &P,
        options: &str,
    ) -> Result<Self, Error> {
        let c_path = CString::new(path.as_ref().as_bytes())?;
        let c_options = CString::new(options)?;

        unsafe {
            ffi_try!(mupdf_new_pdfocr_writer(
                context(),
                c_path.as_ptr(),
                c_options.as_ptr()
            ))
        }
        .map(|inner| Self { inner })
    }

    pub fn begin_page(&mut self, media_box: Rect) -> Result<Device, Error> {
        unsafe {
            ffi_try!(mupdf_document_writer_begin_page(
                context(),
                self.inner,
                media_box.into()
            ))
        }
        .map(|dev| unsafe { Device::from_raw(dev, ptr::null_mut()) })
    }

    pub fn end_page(&mut self, mut device: Device) -> Result<(), Error> {
        // End page closes and drops the device. Prevent dropping it twice
        // by setting the inner device to null here. Order is important here,
        // because the ffi_try! on the end page can return early while already
        // having dropped the device, causing a double-free anyway.
        device.dev = ptr::null_mut();
        unsafe { ffi_try!(mupdf_document_writer_end_page(context(), self.inner)) }
    }
}

impl Drop for DocumentWriter {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                fz_close_document_writer(context(), self.inner);
                fz_drop_document_writer(context(), self.inner);
            }
        }
    }
}

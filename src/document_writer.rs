use std::ffi::CString;
use std::ptr;

use mupdf_sys::*;

use crate::{context, Device, Error, Rect};

#[derive(Debug)]
pub struct DocumentWriter {
    inner: *mut fz_document_writer,
}

impl DocumentWriter {
    pub fn new(filename: &str, format: &str, options: &str) -> Result<Self, Error> {
        let c_filename = CString::new(filename)?;
        let c_format = CString::new(format)?;
        let c_options = CString::new(options)?;
        let inner = unsafe {
            ffi_try!(mupdf_new_document_writer(
                context(),
                c_filename.as_ptr(),
                c_format.as_ptr(),
                c_options.as_ptr()
            ))
        };
        Ok(Self { inner })
    }

    pub fn begin_page(&mut self, media_box: Rect) -> Result<Device, Error> {
        unsafe {
            let dev = ffi_try!(mupdf_document_writer_begin_page(
                context(),
                self.inner,
                media_box.into()
            ));
            Ok(Device::from_raw(dev, ptr::null_mut()))
        }
    }

    pub fn end_page(&mut self, mut device: Device) -> Result<(), Error> {
        unsafe {
            // End page closes and drops the device. Prevent dropping it twice
            // by setting the inner device to null here. Order is important here,
            // because the ffi_try! on the end page can return early while already
            // having dropped the device, causing a double-free anyway.
            device.dev = ptr::null_mut();
            ffi_try!(mupdf_document_writer_end_page(context(), self.inner));
        }
        Ok(())
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

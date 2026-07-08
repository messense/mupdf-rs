use std::ffi::CString;
use std::ops::Deref;
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
    pub fn with_ocr<P: AsRef<FilePath> + ?Sized>(path: &P, options: &str) -> Result<Self, Error> {
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

    pub fn begin_page(&mut self, media_box: Rect) -> Result<WriterPage<'_>, Error> {
        unsafe {
            ffi_try!(mupdf_document_writer_begin_page(
                context(),
                self.inner,
                media_box.into()
            ))
        }
        .map(|dev| WriterPage {
            device: unsafe { Device::from_raw(dev, ptr::null_mut()) },
            writer: self,
            ended: false,
        })
    }
}

/// An in-progress page on a [`DocumentWriter`]. End the page with [`WriterPage::end`].
///
/// Dropping the page without calling [`end`](WriterPage::end) still finishes it (so the
/// writer stays usable), but any error from finishing is silently discarded.
#[derive(Debug)]
pub struct WriterPage<'a> {
    device: Device,
    writer: &'a mut DocumentWriter,
    ended: bool,
}

impl Deref for WriterPage<'_> {
    type Target = Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl WriterPage<'_> {
    /// Finishes this page and returns control to the writer.
    pub fn end(mut self) -> Result<(), Error> {
        self.finish_page()
    }

    fn finish_page(&mut self) -> Result<(), Error> {
        // The writer owns the page device (fz_begin_page keeps no extra reference) and
        // MuPDF closes it inside fz_end_page; do not drop it again.
        self.device.dev = ptr::null_mut();
        self.ended = true;
        unsafe { ffi_try!(mupdf_document_writer_end_page(context(), self.writer.inner)) }
    }
}

impl Drop for WriterPage<'_> {
    fn drop(&mut self) {
        if !self.ended {
            let _ = self.finish_page();
        }
    }
}

impl Drop for DocumentWriter {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            // Closing performs the final save, so it can fail on I/O errors; route it
            // through the fz_try-protected wrapper so the error cannot longjmp out of
            // Rust (an uncaught fz_throw exits the process).
            let _ = unsafe { ffi_try!(mupdf_close_document_writer(context(), self.inner)) };
            unsafe {
                fz_drop_document_writer(context(), self.inner);
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod page_lifecycle_test {
    use super::DocumentWriter;
    use crate::Rect;

    #[test]
    fn dropping_page_without_end_keeps_writer_usable() {
        let output = "tests/output/writer_page_drop.pdf";
        let mut writer = DocumentWriter::new(output, "pdf", "").unwrap();
        let mediabox = Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 100.0,
        };

        drop(writer.begin_page(mediabox).unwrap());

        let page = writer.begin_page(mediabox).unwrap();
        page.end().unwrap();
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "tesseract")]
#[cfg(test)]
mod test {
    use crate::{pdf::PdfDocument, ColorParams, Image, Matrix, Rect};

    use super::DocumentWriter;

    #[test]
    fn test_writer_ocr() {
        let output = "tests/output/ocr.pdf";

        {
            let mut writer = DocumentWriter::with_ocr(output, "").unwrap();

            let image = Image::from_file("tests/files/ocr.png").unwrap();
            let width = image.width() as f32;
            let height = image.height() as f32;

            let page = writer
                .begin_page(Rect {
                    x0: 0.0,
                    y0: 0.0,
                    x1: width,
                    y1: height,
                })
                .unwrap();
            page.fill_image(
                &image,
                &Matrix::new_scale(width, height),
                1.0,
                ColorParams::default(),
            )
            .unwrap();
            page.end().unwrap();
        }

        let doc = PdfDocument::open(output).unwrap();
        let page = doc.load_page(0).unwrap();
        let res = page.search("A short OCR test", 0).unwrap();
        assert_eq!(res.len(), 1);
    }
}

use mupdf_sys::*;

use num_enum::TryFromPrimitive;
use std::convert::TryFrom;

use crate::{context, Error};

#[derive(Debug, Clone, Copy, PartialEq, TryFromPrimitive)]
#[repr(i32)]
pub enum PdfAnnotationType {
    Text = 0,
    Link = 1,
    FreeText = 2,
    Line = 3,
    Square = 4,
    Circle = 5,
    Polygon = 6,
    PloyLine = 7,
    Highlight = 8,
    Underline = 9,
    Squiggly = 10,
    StrikeOut = 11,
    Redact = 12,
    Stamp = 13,
    Caret = 14,
    Ink = 15,
    Popup = 16,
    FileAttachment = 17,
    Sound = 18,
    Movie = 19,
    Widget = 20,
    Screen = 21,
    PrinterMark = 22,
    TrapNet = 23,
    Watermark = 24,
    ThreeD = 25,
    Unknown = -1,
}

#[derive(Debug, Clone, Copy, PartialEq, TryFromPrimitive)]
#[repr(i32)]
pub enum LineEndingStyle {
    None = 0,
    Square = 1,
    Circle = 2,
    Diamond = 3,
    OpenArrow = 4,
    ClosedArrow = 5,
    Butt = 6,
    ROpenArrow = 7,
    RClosedArrow = 8,
    Slash = 9,
}

#[derive(Debug)]
pub struct PdfAnnotation {
    pub(crate) inner: *mut pdf_annot,
}

impl PdfAnnotation {
    pub(crate) unsafe fn from_raw(ptr: *mut pdf_annot) -> Self {
        Self { inner: ptr }
    }

    pub fn r#type(&self) -> Result<PdfAnnotationType, Error> {
        let subtype = unsafe { ffi_try!(mupdf_pdf_annot_type(context(), self.inner)) };
        let typ = PdfAnnotationType::try_from(subtype).unwrap_or(PdfAnnotationType::Unknown);
        Ok(typ)
    }

    pub fn is_hot(&self) -> bool {
        unsafe { (*self.inner).is_hot > 0 }
    }

    pub fn is_active(&self) -> bool {
        unsafe { (*self.inner).is_active > 0 }
    }
}

impl Drop for PdfAnnotation {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                pdf_drop_annot(context(), self.inner);
            }
        }
    }
}

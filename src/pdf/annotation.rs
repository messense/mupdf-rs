use std::convert::TryFrom;
use std::ffi::{CStr, CString};

use mupdf_sys::*;

use crate::pdf::PdfFilterOptions;
use crate::{context, from_enum, Error};

from_enum! { pdf_annot_type,
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum PdfAnnotationType {
        Text = PDF_ANNOT_TEXT,
        Link = PDF_ANNOT_LINK,
        FreeText = PDF_ANNOT_FREE_TEXT,
        Line = PDF_ANNOT_LINE,
        Square = PDF_ANNOT_SQUARE,
        Circle = PDF_ANNOT_CIRCLE,
        Polygon = PDF_ANNOT_POLYGON,
        PloyLine = PDF_ANNOT_POLY_LINE,
        Highlight = PDF_ANNOT_HIGHLIGHT,
        Underline = PDF_ANNOT_UNDERLINE,
        Squiggly = PDF_ANNOT_SQUIGGLY,
        StrikeOut = PDF_ANNOT_STRIKE_OUT,
        Redact = PDF_ANNOT_REDACT,
        Stamp = PDF_ANNOT_STAMP,
        Caret = PDF_ANNOT_CARET,
        Ink = PDF_ANNOT_INK,
        Popup = PDF_ANNOT_POPUP,
        FileAttachment = PDF_ANNOT_FILE_ATTACHMENT,
        Sound = PDF_ANNOT_SOUND,
        Movie = PDF_ANNOT_MOVIE,
        RichMedia = PDF_ANNOT_RICH_MEDIA,
        Widget = PDF_ANNOT_WIDGET,
        Screen = PDF_ANNOT_SCREEN,
        PrinterMark = PDF_ANNOT_PRINTER_MARK,
        TrapNet = PDF_ANNOT_TRAP_NET,
        Watermark = PDF_ANNOT_WATERMARK,
        ThreeD = PDF_ANNOT_3D,
        Projection = PDF_ANNOT_PROJECTION,
        Unknown = PDF_ANNOT_UNKNOWN,
    }
}

from_enum! { pdf_line_ending,
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum LineEndingStyle {
        None = PDF_ANNOT_LE_NONE,
        Square = PDF_ANNOT_LE_SQUARE,
        Circle = PDF_ANNOT_LE_CIRCLE,
        Diamond = PDF_ANNOT_LE_DIAMOND,
        OpenArrow = PDF_ANNOT_LE_OPEN_ARROW,
        ClosedArrow = PDF_ANNOT_LE_CLOSED_ARROW,
        Butt = PDF_ANNOT_LE_BUTT,
        ROpenArrow = PDF_ANNOT_LE_R_OPEN_ARROW,
        RClosedArrow = PDF_ANNOT_LE_R_CLOSED_ARROW,
        Slash = PDF_ANNOT_LE_SLASH,
    }
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
        unsafe { ffi_try!(mupdf_pdf_annot_type(context(), self.inner)) }.map(|subtype| {
            PdfAnnotationType::try_from(subtype).unwrap_or(PdfAnnotationType::Unknown)
        })
    }

    pub fn is_hot(&self) -> bool {
        unsafe { pdf_annot_hot(context(), self.inner) != 0 }
    }

    pub fn is_active(&self) -> bool {
        unsafe { pdf_annot_active(context(), self.inner) != 0 }
    }

    pub fn author(&self) -> Result<Option<&str>, Error> {
        let ptr = unsafe { ffi_try!(mupdf_pdf_annot_author(context(), self.inner)) }?;
        if ptr.is_null() {
            return Ok(None);
        }
        let c_str = unsafe { CStr::from_ptr(ptr) };
        Ok(Some(c_str.to_str().unwrap()))
    }

    pub fn set_author(&mut self, author: &str) -> Result<(), Error> {
        let c_author = CString::new(author)?;
        unsafe {
            ffi_try!(mupdf_pdf_set_annot_author(
                context(),
                self.inner,
                c_author.as_ptr()
            ))
        }
    }

    pub fn filter(&mut self, mut opt: PdfFilterOptions) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pdf_filter_annot_contents(
                context(),
                self.inner,
                &mut opt.inner as *mut _
            ))
        }
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

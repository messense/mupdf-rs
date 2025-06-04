use std::convert::TryFrom;
use std::ffi::{CStr, CString};

use mupdf_sys::*;

use crate::pdf::PdfFilterOptions;
use crate::{context, from_enum, Error};

from_enum! { pdf_annot_type,
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum PdfAnnotationType {
        Text = pdf_annot_type_PDF_ANNOT_TEXT,
        Link = pdf_annot_type_PDF_ANNOT_LINK,
        FreeText = pdf_annot_type_PDF_ANNOT_FREE_TEXT,
        Line = pdf_annot_type_PDF_ANNOT_LINE,
        Square = pdf_annot_type_PDF_ANNOT_SQUARE,
        Circle = pdf_annot_type_PDF_ANNOT_CIRCLE,
        Polygon = pdf_annot_type_PDF_ANNOT_POLYGON,
        PloyLine = pdf_annot_type_PDF_ANNOT_POLY_LINE,
        Highlight = pdf_annot_type_PDF_ANNOT_HIGHLIGHT,
        Underline = pdf_annot_type_PDF_ANNOT_UNDERLINE,
        Squiggly = pdf_annot_type_PDF_ANNOT_SQUIGGLY,
        StrikeOut = pdf_annot_type_PDF_ANNOT_STRIKE_OUT,
        Redact = pdf_annot_type_PDF_ANNOT_REDACT,
        Stamp = pdf_annot_type_PDF_ANNOT_STAMP,
        Caret = pdf_annot_type_PDF_ANNOT_CARET,
        Ink = pdf_annot_type_PDF_ANNOT_INK,
        Popup = pdf_annot_type_PDF_ANNOT_POPUP,
        FileAttachment = pdf_annot_type_PDF_ANNOT_FILE_ATTACHMENT,
        Sound = pdf_annot_type_PDF_ANNOT_SOUND,
        Movie = pdf_annot_type_PDF_ANNOT_MOVIE,
        RichMedia = pdf_annot_type_PDF_ANNOT_RICH_MEDIA,
        Widget = pdf_annot_type_PDF_ANNOT_WIDGET,
        Screen = pdf_annot_type_PDF_ANNOT_SCREEN,
        PrinterMark = pdf_annot_type_PDF_ANNOT_PRINTER_MARK,
        TrapNet = pdf_annot_type_PDF_ANNOT_TRAP_NET,
        Watermark = pdf_annot_type_PDF_ANNOT_WATERMARK,
        ThreeD = pdf_annot_type_PDF_ANNOT_3D,
        Projection = pdf_annot_type_PDF_ANNOT_PROJECTION,
        Unknown = pdf_annot_type_PDF_ANNOT_UNKNOWN,
    }
}

from_enum! { pdf_line_ending,
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum LineEndingStyle {
        None = pdf_line_ending_PDF_ANNOT_LE_NONE,
        Square = pdf_line_ending_PDF_ANNOT_LE_SQUARE,
        Circle = pdf_line_ending_PDF_ANNOT_LE_CIRCLE,
        Diamond = pdf_line_ending_PDF_ANNOT_LE_DIAMOND,
        OpenArrow = pdf_line_ending_PDF_ANNOT_LE_OPEN_ARROW,
        ClosedArrow = pdf_line_ending_PDF_ANNOT_LE_CLOSED_ARROW,
        Butt = pdf_line_ending_PDF_ANNOT_LE_BUTT,
        ROpenArrow = pdf_line_ending_PDF_ANNOT_LE_R_OPEN_ARROW,
        RClosedArrow = pdf_line_ending_PDF_ANNOT_LE_R_CLOSED_ARROW,
        Slash = pdf_line_ending_PDF_ANNOT_LE_SLASH,
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

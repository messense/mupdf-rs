use std::ffi::{c_uint, CStr, CString};
use std::fmt;
use std::str::FromStr;

use mupdf_sys::*;

use crate::{context, from_enum, Buffer, Error, Matrix, Path};

from_enum! { c_uint,
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum SimpleFontEncoding {
        Latin = PDF_SIMPLE_ENCODING_LATIN,
        Greek = PDF_SIMPLE_ENCODING_GREEK,
        Cyrillic = PDF_SIMPLE_ENCODING_CYRILLIC,
    }
}

from_enum! { u32,
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum WriteMode {
        Horizontal = 0,
        Vertical = 1,
    }
}

from_enum! { c_uint,
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum CjkFontOrdering {
        AdobeCns = FZ_ADOBE_CNS,
        AdobeGb = FZ_ADOBE_GB,
        AdobeJapan = FZ_ADOBE_JAPAN,
        AdobeKorea = FZ_ADOBE_KOREA,
    }
}

impl FromStr for CjkFontOrdering {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ordering = match s {
            "zh-Hant" | "zh-TW" | "zh-HK" | "zh-Hans" => Self::AdobeCns,
            "zh-CN" => Self::AdobeGb,
            "ja" => Self::AdobeJapan,
            "ko" => Self::AdobeKorea,
            _ => {
                return Err(Error::InvalidLanguage(s.to_string()));
            }
        };
        Ok(ordering)
    }
}

#[derive(Debug)]
pub struct Font {
    pub(crate) inner: *mut fz_font,
}

impl Font {
    pub(crate) unsafe fn from_raw(ptr: *mut fz_font) -> Self {
        Self { inner: ptr }
    }

    pub fn new(name: &str) -> Result<Self, Error> {
        Self::new_with_index(name, 0)
    }

    pub fn new_with_index(name: &str, index: i32) -> Result<Self, Error> {
        let c_name = CString::new(name)?;
        unsafe { ffi_try!(mupdf_new_font(context(), c_name.as_ptr(), index)) }
            .map(|inner| Self { inner })
    }

    pub fn from_bytes(name: &str, font_data: &[u8]) -> Result<Self, Error> {
        Self::from_bytes_with_index(name, 0, font_data)
    }

    pub fn from_bytes_with_index(name: &str, index: i32, font_data: &[u8]) -> Result<Self, Error> {
        let c_name = CString::new(name)?;
        let buffer = Buffer::from_bytes(font_data)?;
        unsafe {
            ffi_try!(mupdf_new_font_from_buffer(
                context(),
                c_name.as_ptr(),
                index,
                buffer.inner
            ))
        }
        .map(|inner| Self { inner })
    }

    pub fn name(&self) -> &str {
        let f_name = unsafe { fz_font_name(context(), self.inner) };
        let c_name = unsafe { CStr::from_ptr(f_name) };
        c_name.to_str().unwrap()
    }

    pub fn is_bold(&self) -> bool {
        unsafe { fz_font_is_bold(context(), self.inner) > 0 }
    }

    pub fn is_italic(&self) -> bool {
        unsafe { fz_font_is_italic(context(), self.inner) > 0 }
    }

    pub fn is_monospaced(&self) -> bool {
        unsafe { fz_font_is_monospaced(context(), self.inner) > 0 }
    }

    pub fn is_serif(&self) -> bool {
        unsafe { fz_font_is_serif(context(), self.inner) > 0 }
    }

    pub fn ascender(&self) -> f32 {
        unsafe { fz_font_ascender(context(), self.inner) }
    }

    pub fn descender(&self) -> f32 {
        unsafe { fz_font_descender(context(), self.inner) }
    }

    pub fn encode_character(&self, unicode: i32) -> Result<i32, Error> {
        unsafe { ffi_try!(mupdf_encode_character(context(), self.inner, unicode)) }
    }

    pub fn advance_glyph_with_wmode(&self, glyph: i32, wmode: bool) -> Result<f32, Error> {
        unsafe { ffi_try!(mupdf_advance_glyph(context(), self.inner, glyph, wmode)) }
    }

    pub fn advance_glyph(&self, glyph: i32) -> Result<f32, Error> {
        self.advance_glyph_with_wmode(glyph, false)
    }

    pub fn outline_glyph_with_ctm(&self, glyph: i32, ctm: &Matrix) -> Result<Option<Path>, Error> {
        let inner = unsafe {
            ffi_try!(mupdf_outline_glyph(
                context(),
                self.inner,
                glyph,
                ctm.into()
            ))
        }?;
        if inner.is_null() {
            return Ok(None);
        }
        Ok(Some(unsafe { Path::from_raw(inner) }))
    }

    pub fn outline_glyph(&self, glyph: i32) -> Result<Option<Path>, Error> {
        self.outline_glyph_with_ctm(glyph, &Matrix::IDENTITY)
    }
}

impl Drop for Font {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                fz_drop_font(context(), self.inner);
            }
        }
    }
}

impl fmt::Display for Font {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Font({})", self.name())
    }
}

#[cfg(test)]
mod test {
    use super::Font;

    #[test]
    fn test_font_name() {
        let font = Font::new("Courier").expect("new font failed");
        assert_eq!(font.name(), "Courier");
    }

    #[test]
    fn test_encode_character() {
        let font = Font::new("Courier").expect("new font failed");
        let glyph = font.encode_character(97).unwrap();
        assert_eq!(glyph, 66);
    }

    #[test]
    fn test_advance_glyph() {
        let font = Font::new("Courier").expect("new font failed");
        let glyph = font.encode_character(97).unwrap();
        let advance = font.advance_glyph(glyph).unwrap();
        assert_eq!(advance, 0.6);
    }

    #[test]
    fn test_outline_glyph() {
        let font = Font::new("Courier").expect("new font failed");
        let glyph = font.encode_character(97).unwrap();
        let _path = font.outline_glyph(glyph).unwrap();
    }
}

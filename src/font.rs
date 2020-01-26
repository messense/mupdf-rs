use std::ffi::{CStr, CString};
use std::os::raw::c_int;

use mupdf_sys::*;

use crate::{context, Error};

#[derive(Debug)]
pub struct Font {
    inner: *mut fz_font,
}

impl Font {
    pub fn new(name: &str) -> Result<Self, Error> {
        Self::new_with_index(name, 0)
    }

    pub fn new_with_index(name: &str, index: i32) -> Result<Self, Error> {
        let c_name = CString::new(name).unwrap();
        let inner = unsafe { ffi_try!(mupdf_new_font(context(), c_name.as_ptr(), index)) };
        Ok(Self { inner })
    }

    pub fn from_bytes(name: &str, font_data: &[u8]) -> Result<Self, Error> {
        Self::from_bytes_with_index(name, 0, font_data)
    }

    pub fn from_bytes_with_index(name: &str, index: i32, font_data: &[u8]) -> Result<Self, Error> {
        let c_name = CString::new(name).unwrap();
        let data_len = font_data.len() as c_int;
        let inner = unsafe {
            ffi_try!(mupdf_new_font_from_memory(
                context(),
                c_name.as_ptr(),
                index,
                font_data.as_ptr(),
                data_len
            ))
        };
        Ok(Self { inner })
    }

    pub fn name(&self) -> String {
        let f_name = unsafe { fz_font_name(context(), self.inner) };
        let c_name = unsafe { CStr::from_ptr(f_name) };
        c_name.to_string_lossy().into_owned()
    }

    pub fn encode_character(&self, unicode: i32) -> Result<i32, Error> {
        let glyph = unsafe { ffi_try!(mupdf_encode_character(context(), self.inner, unicode)) };
        Ok(glyph)
    }

    pub fn advance_glyph_with_wmode(&self, glyph: i32, wmode: bool) -> Result<f32, Error> {
        let advance = unsafe { ffi_try!(mupdf_advance_glyph(context(), self.inner, glyph, wmode)) };
        Ok(advance)
    }

    pub fn advance_glyph(&self, glyph: i32) -> Result<f32, Error> {
        self.advance_glyph_with_wmode(glyph, false)
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
}

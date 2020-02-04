use std::ffi::CString;

use mupdf_sys::*;

use crate::{context, Colorspace, DisplayList, Error, Pixmap};

#[derive(Debug)]
pub struct Image {
    pub(crate) inner: *mut fz_image,
}

impl Image {
    unsafe fn from_raw(image: *mut fz_image) -> Self {
        Self { inner: image }
    }

    pub fn from_pixmap(pixmap: &Pixmap) -> Result<Self, Error> {
        let inner = unsafe { ffi_try!(mupdf_new_image_from_pixmap(context(), pixmap.inner)) };
        Ok(Self { inner })
    }

    pub fn from_file(filename: &str) -> Result<Self, Error> {
        let c_filename = CString::new(filename)?;
        let inner = unsafe { ffi_try!(mupdf_new_image_from_file(context(), c_filename.as_ptr())) };
        Ok(Self { inner })
    }

    pub fn from_display_list(list: &DisplayList, width: f32, height: f32) -> Result<Self, Error> {
        let inner = unsafe {
            ffi_try!(mupdf_new_image_from_display_list(
                context(),
                list.inner,
                width,
                height
            ))
        };
        Ok(Self { inner })
    }

    pub fn width(&self) -> u32 {
        unsafe { (*self.inner).w as u32 }
    }

    pub fn height(&self) -> u32 {
        unsafe { (*self.inner).h as u32 }
    }

    pub fn number_of_components(&self) -> u8 {
        unsafe { (*self.inner).n }
    }

    pub fn bits_per_components(&self) -> u8 {
        unsafe { (*self.inner).bpc }
    }

    pub fn color_space(&self) -> Colorspace {
        unsafe { Colorspace::from_raw((*self.inner).colorspace) }
    }

    pub fn resolution(&self) -> (i32, i32) {
        unsafe {
            let x_res = (*self.inner).xres;
            let y_res = (*self.inner).yres;
            (x_res, y_res)
        }
    }

    pub fn mask(&self) -> Option<Self> {
        unsafe {
            if (*self.inner).mask.is_null() {
                return None;
            }
            Some(Self::from_raw((*self.inner).mask))
        }
    }

    pub fn to_pixmap(&self) -> Result<Pixmap, Error> {
        unsafe {
            let inner = ffi_try!(mupdf_get_pixmap_from_image(context(), self.inner));
            Ok(Pixmap::from_raw(inner))
        }
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                fz_drop_image(context(), self.inner);
            }
        }
    }
}

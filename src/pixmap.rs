use std::ffi::CString;

use mupdf_sys::*;

use crate::{context, ColorSpace, Error, IRect};

/// Pixmaps (pixel maps) are objects at the heart of MuPDF’s rendering capabilities.
///
/// They represent plane rectangular sets of pixels.
/// Each pixel is described by a number of bytes (components) defining its color,
/// plus an optional alpha byte defining its transparency.
#[derive(Debug)]
pub struct Pixmap {
    pub(crate) inner: *mut fz_pixmap,
}

impl Pixmap {
    pub(crate) unsafe fn from_raw(pixmap: *mut fz_pixmap) -> Self {
        Self { inner: pixmap }
    }

    /// Create an empty pixmap of size and origin.
    ///
    /// Note that the image area is not initialized and will contain crap data
    pub fn new(
        cs: &ColorSpace,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        alpha: bool,
    ) -> Result<Self, Error> {
        let ctx = context();
        let inner = unsafe { ffi_try!(mupdf_new_pixmap(ctx, cs.inner, x, y, w, h, alpha)) };
        Ok(Self { inner })
    }

    /// Create an empty pixmap of size and origin given by the rectangle.
    ///
    /// Note that the image area is not initialized and will contain crap data
    pub fn new_with_rect(cs: &ColorSpace, rect: IRect, alpha: bool) -> Result<Self, Error> {
        let x = rect.x0;
        let y = rect.y0;
        let w = rect.x1 - rect.x0;
        let h = rect.y1 - rect.y0;
        Self::new(cs, x, y, w, h, alpha)
    }

    /// Create an empty pixmap of size with origin set to `(0, 0)`.
    ///
    /// Note that the image area is not initialized and will contain crap data
    pub fn new_with_w_h(cs: &ColorSpace, w: i32, h: i32, alpha: bool) -> Result<Self, Error> {
        Self::new(cs, 0, 0, w, h, alpha)
    }

    /// X-coordinate of top-left corner
    pub fn x(&self) -> i32 {
        unsafe { (*self.inner).x }
    }

    /// Y-coordinate of top-left corner
    pub fn y(&self) -> i32 {
        unsafe { (*self.inner).y }
    }

    /// Width of the region in pixels.
    pub fn width(&self) -> u32 {
        unsafe { (*self.inner).w as u32 }
    }

    /// Height of the region in pixels.
    pub fn height(&self) -> u32 {
        unsafe { (*self.inner).h as u32 }
    }

    /// Contains the length of one row of image data in Pixmap samples.
    ///
    /// This is primarily used for calculation purposes. The following expressions are true:
    /// * `samples.len() == height * stride`
    /// * `width * n == stride`
    pub fn stride(&self) -> isize {
        unsafe { (*self.inner).stride }
    }

    /// Number of components per pixel.
    pub fn n(&self) -> u8 {
        unsafe { (*self.inner).n }
    }

    /// Indicates whether the pixmap contains transparency information.
    pub fn alpha(&self) -> bool {
        unsafe { (*self.inner).alpha > 0 }
    }

    // The colorspace of the pixmap.
    //
    // This value may be None if the image is to be treated as a so-called image mask or stencil mask
    pub fn color_space(&self) -> Option<ColorSpace> {
        unsafe {
            let ptr = (*self.inner).colorspace;
            if ptr.is_null() {
                return None;
            }
            Some(ColorSpace::from_raw(ptr))
        }
    }

    /// Horizontal and vertical resolution in dpi (dots per inch).
    pub fn resolution(&self) -> (i32, i32) {
        unsafe {
            let x_res = (*self.inner).xres;
            let y_res = (*self.inner).yres;
            (x_res, y_res)
        }
    }

    /// Size of pixmap
    pub fn size(&self) -> usize {
        unsafe { fz_pixmap_size(context(), self.inner) }
    }

    pub fn rect(&self) -> IRect {
        unsafe { fz_pixmap_bbox(context(), self.inner).into() }
    }

    /// Initialize the samples area with 0x00
    pub fn clear(&mut self) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_clear_pixmap(context(), self.inner));
        }
        Ok(())
    }

    /// Initialize the samples area
    ///
    /// ## Params
    ///
    /// * `value` - values from 0 to 255 are valid. Each color byte of each pixel will be set to this value, while alpha will be set to 255 (non-transparent) if present
    pub fn clear_with(&mut self, value: i32) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_clear_pixmap_with_value(context(), self.inner, value));
        }
        Ok(())
    }

    pub fn save_as_png(&self, filename: &str) -> Result<(), Error> {
        let c_filename = CString::new(filename)?;
        unsafe {
            ffi_try!(mupdf_save_pixmap_as_png(
                context(),
                self.inner,
                c_filename.as_ptr()
            ));
        }
        Ok(())
    }

    pub fn invert(&mut self) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_invert_pixmap(context(), self.inner));
        }
        Ok(())
    }

    /// Apply a gamma factor to a pixmap, i.e. lighten or darken it.
    ///
    /// Pixmaps with no colorspace are ignored
    ///
    /// ## Params
    ///
    /// * `gamma` - gamma = 1.0 does nothing, gamma < 1.0 lightens, gamma > 1.0 darkens the image.
    pub fn gamma(&mut self, gamma: f32) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_gamma_pixmap(context(), self.inner, gamma));
        }
        Ok(())
    }

    /// Tint pixmap with color
    pub fn tint(&mut self, black: i32, white: i32) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_tint_pixmap(context(), self.inner, black, white));
        }
        Ok(())
    }

    /// Shrink the pixmap by dividing both its width and height by `2^n`.
    pub fn shrink(&mut self, n: i32) -> Result<(), Error> {
        if n >= 1 {
            unsafe {
                fz_subsample_pixmap(context(), self.inner, n);
            }
        }
        Ok(())
    }
}

impl Drop for Pixmap {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe { fz_drop_pixmap(context(), self.inner) };
        }
    }
}

#[cfg(test)]
mod test {
    use super::{ColorSpace, IRect, Pixmap};

    #[test]
    fn test_pixmap_properties() {
        let cs = ColorSpace::device_rgb();
        let pixmap = Pixmap::new_with_w_h(&cs, 100, 100, false).expect("Pixmap::new_with_w_h");
        let pixmap_cs = pixmap.color_space().unwrap();
        assert_eq!(cs, pixmap_cs);

        let resolution = pixmap.resolution();
        assert_eq!(resolution, (96, 96));

        let size = pixmap.size();
        assert!(size > 0);

        let rect = pixmap.rect();
        assert_eq!(rect, IRect::new(0, 0, 100, 100));
    }

    #[test]
    fn test_pixmap_clear() {
        let cs = ColorSpace::device_rgb();
        let mut pixmap = Pixmap::new_with_w_h(&cs, 100, 100, false).expect("Pixmap::new_with_w_h");
        pixmap.clear().unwrap();
        pixmap.clear_with(1).unwrap();
    }

    #[test]
    fn test_pixmap_invert() {
        let cs = ColorSpace::device_rgb();
        let mut pixmap = Pixmap::new_with_w_h(&cs, 100, 100, false).expect("Pixmap::new_with_w_h");
        pixmap.invert().unwrap();
    }

    #[test]
    fn test_pixmap_gamma() {
        let cs = ColorSpace::device_rgb();
        let mut pixmap = Pixmap::new_with_w_h(&cs, 100, 100, false).expect("Pixmap::new_with_w_h");
        pixmap.gamma(2.0).unwrap();
    }

    #[test]
    fn test_pixmap_tint() {
        let cs = ColorSpace::device_rgb();
        let mut pixmap = Pixmap::new_with_w_h(&cs, 100, 100, false).expect("Pixmap::new_with_w_h");
        pixmap.tint(0, 255).unwrap();
    }
}

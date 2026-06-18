use std::collections::{HashMap, HashSet};
use std::ffi::CString;
use std::io::{self, Write};
use std::slice;

use mupdf_sys::*;

use crate::{context, Buffer, Colorspace, Error, IRect, Quad};

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub enum ImageFormat {
    PNG = 0,
    PNM = 1,
    PAM = 2,
    PSD = 3,
    PS = 4,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pixel {
    components: Vec<u8>,
}

impl Pixel {
    pub fn new(components: impl Into<Vec<u8>>) -> Self {
        Self {
            components: components.into(),
        }
    }

    pub fn gray(value: u8) -> Self {
        Self::new([value])
    }

    pub fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self::new([red, green, blue])
    }

    pub fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self::new([red, green, blue, alpha])
    }

    pub fn components(&self) -> &[u8] {
        &self.components
    }
}

impl AsRef<[u8]> for Pixel {
    fn as_ref(&self) -> &[u8] {
        self.components()
    }
}

impl From<Vec<u8>> for Pixel {
    fn from(components: Vec<u8>) -> Self {
        Self::new(components)
    }
}

impl<const N: usize> From<[u8; N]> for Pixel {
    fn from(components: [u8; N]) -> Self {
        Self::new(components)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PixmapDigest([u8; 16]);

impl PixmapDigest {
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorUsage {
    pub pixel: Pixel,
    pub count: usize,
    pub ratio: f32,
}

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
        cs: &Colorspace,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        alpha: bool,
    ) -> Result<Self, Error> {
        unsafe { ffi_try!(mupdf_new_pixmap(context(), cs.inner, x, y, w, h, alpha)) }
            .map(|inner| Self { inner })
    }

    /// Create an empty pixmap of size and origin given by the rectangle.
    ///
    /// Note that the image area is not initialized and will contain crap data
    pub fn new_with_rect(cs: &Colorspace, rect: IRect, alpha: bool) -> Result<Self, Error> {
        let x = rect.x0;
        let y = rect.y0;
        let w = rect.x1 - rect.x0;
        let h = rect.y1 - rect.y0;
        Self::new(cs, x, y, w, h, alpha)
    }

    /// Create an empty pixmap of size with origin set to `(0, 0)`.
    ///
    /// Note that the image area is not initialized and will contain crap data
    pub fn new_with_w_h(cs: &Colorspace, w: i32, h: i32, alpha: bool) -> Result<Self, Error> {
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

    /// Pixmap origin, `(x, y)`
    pub fn origin(&self) -> (i32, i32) {
        unsafe { ((*self.inner).x, (*self.inner).y) }
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
    pub fn color_space(&self) -> Option<Colorspace> {
        unsafe {
            let ptr = (*self.inner).colorspace;
            if ptr.is_null() {
                return None;
            }
            Some(Colorspace::from_raw(ptr))
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

    // Set resolution
    pub fn set_resolution(&mut self, x_res: i32, y_res: i32) {
        unsafe {
            fz_set_pixmap_resolution(context(), self.inner, x_res, y_res);
        }
    }

    pub fn rect(&self) -> IRect {
        unsafe { fz_pixmap_bbox(context(), self.inner).into() }
    }

    pub fn samples(&self) -> &[u8] {
        let len = match crate::samples_len(self.stride(), self.height()) {
            Some(len) => len,
            None => return &[],
        };
        let ptr = unsafe { (*self.inner).samples };
        if ptr.is_null() {
            &[]
        } else {
            unsafe { slice::from_raw_parts(ptr, len) }
        }
    }

    pub fn samples_mut(&mut self) -> &mut [u8] {
        let len = match crate::samples_len(self.stride(), self.height()) {
            Some(len) => len,
            None => return &mut [],
        };
        let ptr = unsafe { (*self.inner).samples };
        if ptr.is_null() {
            &mut []
        } else {
            unsafe { slice::from_raw_parts_mut(ptr, len) }
        }
    }

    fn pixel_rows(&self) -> impl Iterator<Item = &[u8]> {
        let n = self.n() as usize;
        let row_len = (self.width() as usize).saturating_mul(n);
        self.samples()
            .chunks(self.stride().max(1) as usize)
            .map(move |row| &row[..row_len.min(row.len())])
    }

    fn checked_stride(&self) -> Result<usize, Error> {
        usize::try_from(self.stride()).map_err(|_| {
            Error::InvalidArgument("pixmap stride cannot be represented as usize".to_owned())
        })
    }

    fn pixel_offset(&self, x: i32, y: i32) -> Result<usize, Error> {
        let rect = self.rect();
        if !rect.contains(x, y) {
            return Err(Error::InvalidArgument(format!(
                "pixel coordinate ({x}, {y}) is outside pixmap bounds {rect}"
            )));
        }

        let n = self.n() as usize;
        let stride = self.checked_stride()?;
        let row = usize::try_from(y - rect.y0).unwrap();
        let column = usize::try_from(x - rect.x0).unwrap();
        let offset = row
            .checked_mul(stride)
            .and_then(|offset| offset.checked_add(column.checked_mul(n)?))
            .ok_or_else(|| Error::InvalidArgument("pixel offset overflow".to_owned()))?;

        let len = self.samples().len();
        if offset.checked_add(n).is_none_or(|end| end > len) {
            return Err(Error::InvalidArgument(
                "pixel offset exceeds pixmap samples".to_owned(),
            ));
        }

        Ok(offset)
    }

    fn validate_pixel_components(&self, pixel: &Pixel) -> Result<(), Error> {
        let expected = self.n() as usize;
        let actual = pixel.components().len();
        if actual != expected {
            return Err(Error::InvalidArgument(format!(
                "pixel has {actual} components but pixmap requires {expected}"
            )));
        }
        Ok(())
    }

    pub fn pixel(&self, x: i32, y: i32) -> Result<Pixel, Error> {
        let offset = self.pixel_offset(x, y)?;
        let n = self.n() as usize;
        Ok(Pixel::new(self.samples()[offset..offset + n].to_vec()))
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, pixel: impl Into<Pixel>) -> Result<(), Error> {
        let pixel = pixel.into();
        self.validate_pixel_components(&pixel)?;
        let offset = self.pixel_offset(x, y)?;
        let n = self.n() as usize;
        self.samples_mut()[offset..offset + n].copy_from_slice(pixel.components());
        Ok(())
    }

    pub fn set_rect(&mut self, rect: IRect, pixel: impl Into<Pixel>) -> Result<(), Error> {
        if !rect.is_valid() {
            return Err(Error::InvalidArgument(format!(
                "pixmap rectangle {rect} is not valid"
            )));
        }

        let pixel = pixel.into();
        self.validate_pixel_components(&pixel)?;
        let rect = rect.intersect(&self.rect());
        if rect.is_empty() {
            return Ok(());
        }

        let n = self.n() as usize;
        let stride = self.checked_stride()?;
        let bounds = self.rect();
        let row_start = usize::try_from(rect.y0 - bounds.y0).unwrap();
        let row_end = usize::try_from(rect.y1 - bounds.y0).unwrap();
        let column_start = usize::try_from(rect.x0 - bounds.x0).unwrap();
        let column_end = usize::try_from(rect.x1 - bounds.x0).unwrap();
        let samples = self.samples_mut();

        for row in row_start..row_end {
            for column in column_start..column_end {
                let offset = row
                    .checked_mul(stride)
                    .and_then(|offset| offset.checked_add(column.checked_mul(n)?))
                    .ok_or_else(|| Error::InvalidArgument("pixel offset overflow".to_owned()))?;
                samples[offset..offset + n].copy_from_slice(pixel.components());
            }
        }

        Ok(())
    }

    pub fn set_alpha(&mut self, alpha: u8) -> Result<(), Error> {
        if !self.alpha() {
            return Err(Error::InvalidArgument(
                "pixmap does not have an alpha component".to_owned(),
            ));
        }

        let n = self.n() as usize;
        let alpha_index = n - 1;
        for pixel in self.samples_mut().chunks_exact_mut(n) {
            pixel[alpha_index] = alpha;
        }
        Ok(())
    }

    /// Only valid for RGBA or BGRA pixmaps
    pub fn pixels(&self) -> Option<&[u32]> {
        if self.n() != 4 || !self.alpha() {
            // invalid colorspace
            return None;
        }

        let size = (self.width() * self.height()) as usize;
        if size * 4 != (self.height() as usize * self.stride() as usize) {
            // invalid stride
            return None;
        }

        let ptr = unsafe { (*self.inner).samples };
        if ptr.is_null() {
            Some(&[])
        } else {
            Some(unsafe { slice::from_raw_parts(ptr.cast(), size) })
        }
    }

    /// Initialize the samples area with 0x00
    pub fn clear(&mut self) -> Result<(), Error> {
        unsafe { ffi_try!(mupdf_clear_pixmap(context(), self.inner)) }
    }

    /// Initialize the samples area
    ///
    /// ## Params
    ///
    /// * `value` - values from 0 to 255 are valid. Each color byte of each pixel will be set to this value, while alpha will be set to 255 (non-transparent) if present
    pub fn clear_with(&mut self, value: i32) -> Result<(), Error> {
        unsafe { ffi_try!(mupdf_clear_pixmap_with_value(context(), self.inner, value)) }
    }

    pub fn clear_rect_with(&mut self, rect: IRect, value: i32) -> Result<(), Error> {
        let rect = rect.intersect(&self.rect());
        if rect.is_empty() {
            return Ok(());
        }
        unsafe {
            ffi_try!(mupdf_clear_pixmap_rect_with_value(
                context(),
                self.inner,
                value,
                rect.into()
            ))
        }
    }

    pub fn save_as(&self, filename: &str, format: ImageFormat) -> Result<(), Error> {
        let c_filename = CString::new(filename)?;
        unsafe {
            ffi_try!(mupdf_save_pixmap_as(
                context(),
                self.inner,
                c_filename.as_ptr(),
                format as i32
            ))
        }
    }

    pub fn invert(&mut self) -> Result<(), Error> {
        unsafe { ffi_try!(mupdf_invert_pixmap(context(), self.inner)) }
    }

    pub fn invert_rect(&mut self, rect: IRect) -> Result<(), Error> {
        let rect = rect.intersect(&self.rect());
        if rect.is_empty() {
            return Ok(());
        }
        unsafe { ffi_try!(mupdf_invert_pixmap_rect(context(), self.inner, rect.into())) }
    }

    pub fn digest(&self) -> Result<PixmapDigest, Error> {
        let mut digest = [0; 16];
        unsafe { ffi_try!(mupdf_md5_pixmap(context(), self.inner, digest.as_mut_ptr()))? };
        Ok(PixmapDigest(digest))
    }

    pub fn color_count(&self) -> usize {
        let n = self.n() as usize;
        if n == 0 {
            return 0;
        }
        self.pixel_rows()
            .flat_map(|row| row.chunks_exact(n))
            .map(|components| Pixel::new(components.to_vec()))
            .collect::<HashSet<_>>()
            .len()
    }

    pub fn top_color_usage(&self) -> Option<ColorUsage> {
        let n = self.n() as usize;
        if n == 0 || self.samples().is_empty() {
            return None;
        }

        let mut counts = HashMap::<Pixel, usize>::new();
        for components in self.pixel_rows().flat_map(|row| row.chunks_exact(n)) {
            *counts.entry(Pixel::new(components.to_vec())).or_default() += 1;
        }

        let total = (self.width() as usize).checked_mul(self.height() as usize)?;
        counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(pixel, count)| ColorUsage {
                pixel,
                count,
                ratio: count as f32 / total as f32,
            })
    }

    /// Subsample this pixmap in place by a power-of-two level.
    ///
    /// A level of `1` halves the dimensions, `2` quarters them, and so on.
    pub fn shrink(&mut self, level: i32) -> Result<(), Error> {
        if level < 1 {
            return Err(Error::InvalidArgument(
                "shrink level must be at least 1".to_owned(),
            ));
        }
        unsafe { ffi_try!(mupdf_subsample_pixmap(context(), self.inner, level)) }
    }

    pub fn warp(&self, points: impl Into<Quad>, width: i32, height: i32) -> Result<Pixmap, Error> {
        if width <= 0 || height <= 0 {
            return Err(Error::InvalidArgument(
                "warp width and height must be positive".to_owned(),
            ));
        }
        unsafe {
            ffi_try!(mupdf_warp_pixmap(
                context(),
                self.inner,
                points.into().into(),
                width,
                height
            ))
        }
        .map(|inner| Self { inner })
    }

    /// Apply a gamma factor to a pixmap, i.e. lighten or darken it.
    ///
    /// Pixmaps with no colorspace are ignored
    ///
    /// ## Params
    ///
    /// * `gamma` - gamma = 1.0 does nothing, gamma < 1.0 lightens, gamma > 1.0 darkens the image.
    pub fn gamma(&mut self, gamma: f32) -> Result<(), Error> {
        unsafe { ffi_try!(mupdf_gamma_pixmap(context(), self.inner, gamma)) }
    }

    /// Tint pixmap with color
    pub fn tint(&mut self, black: i32, white: i32) -> Result<(), Error> {
        unsafe { ffi_try!(mupdf_tint_pixmap(context(), self.inner, black, white)) }
    }

    fn get_image_data(&self, format: ImageFormat) -> Result<Buffer, Error> {
        unsafe {
            ffi_try!(mupdf_pixmap_get_image_data(
                context(),
                self.inner,
                format as i32
            ))
        }
        .map(|inner| unsafe { Buffer::from_raw(inner) })
    }

    pub fn write_to<W: Write>(&self, w: &mut W, format: ImageFormat) -> Result<u64, Error> {
        let mut buf = self.get_image_data(format)?;
        Ok(io::copy(&mut buf, w)?)
    }

    pub fn try_clone(&self) -> Result<Self, Error> {
        unsafe { ffi_try!(mupdf_clone_pixmap(context(), self.inner)) }.map(|inner| Self { inner })
    }
}

impl Drop for Pixmap {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe { fz_drop_pixmap(context(), self.inner) };
        }
    }
}

impl Clone for Pixmap {
    fn clone(&self) -> Pixmap {
        self.try_clone().unwrap()
    }
}

#[cfg(test)]
mod test {
    use crate::Rect;

    use super::{Colorspace, IRect, Pixel, Pixmap};

    #[test]
    fn test_pixmap_properties() {
        let cs = Colorspace::device_rgb();
        let mut pixmap = Pixmap::new_with_w_h(&cs, 100, 100, false).expect("Pixmap::new_with_w_h");
        pixmap.clear_with(0).unwrap();
        let pixmap_cs = pixmap.color_space().unwrap();
        assert_eq!(cs, pixmap_cs);

        let resolution = pixmap.resolution();
        assert_eq!(resolution, (96, 96));

        let rect = pixmap.rect();
        assert_eq!(rect, IRect::new(0, 0, 100, 100));

        assert_eq!(pixmap.origin(), (0, 0));

        let samples = pixmap.samples();
        assert!(samples.iter().all(|x| *x == 0));
        assert_eq!(samples.len(), 100 * 100 * pixmap_cs.n() as usize);
    }

    #[test]
    fn test_pixmap_clear() {
        let cs = Colorspace::device_rgb();
        let mut pixmap = Pixmap::new_with_w_h(&cs, 100, 100, false).expect("Pixmap::new_with_w_h");
        pixmap.clear().unwrap();
        pixmap.clear_with(1).unwrap();
    }

    #[test]
    fn test_pixmap_invert() {
        let cs = Colorspace::device_rgb();
        let mut pixmap = Pixmap::new_with_w_h(&cs, 100, 100, false).expect("Pixmap::new_with_w_h");
        pixmap.clear().unwrap();
        pixmap.invert().unwrap();
    }

    #[test]
    fn test_pixmap_gamma() {
        let cs = Colorspace::device_rgb();
        let mut pixmap = Pixmap::new_with_w_h(&cs, 100, 100, false).expect("Pixmap::new_with_w_h");
        pixmap.clear().unwrap();
        pixmap.gamma(2.0).unwrap();
    }

    #[test]
    fn test_pixmap_tint() {
        let cs = Colorspace::device_rgb();
        let mut pixmap = Pixmap::new_with_w_h(&cs, 100, 100, false).expect("Pixmap::new_with_w_h");
        pixmap.clear().unwrap();
        pixmap.tint(0, 255).unwrap();
    }

    #[test]
    fn test_pixmap_pixels() {
        let cs = Colorspace::device_rgb();

        // alpha: false
        let mut pixmap = Pixmap::new_with_w_h(&cs, 100, 100, false).expect("Pixmap::new_with_w_h");
        pixmap.clear().unwrap();
        let pixels = pixmap.pixels();
        assert!(pixels.is_none());

        // alpha: true
        let mut pixmap = Pixmap::new_with_w_h(&cs, 100, 100, true).expect("Pixmap::new_with_w_h");
        pixmap.clear().unwrap();
        let pixels = pixmap.pixels();
        assert!(pixels.is_some());
    }

    #[test]
    fn test_pixmap_empty() {
        let cs = Colorspace::device_rgb();

        let mut pixmap = Pixmap::new_with_w_h(&cs, 0, 0, false).expect("Pixmap::new_with_w_h");
        assert!(pixmap.samples().is_empty());
        assert!(pixmap.samples_mut().is_empty());
    }

    #[test]
    fn test_pixmap_pixel_editing_and_color_usage() {
        let cs = Colorspace::device_rgb();
        let mut pixmap = Pixmap::new_with_w_h(&cs, 3, 3, false).unwrap();
        pixmap.clear_with(0).unwrap();

        pixmap.set_pixel(1, 1, Pixel::rgb(10, 20, 30)).unwrap();
        assert_eq!(pixmap.pixel(1, 1).unwrap(), Pixel::rgb(10, 20, 30));
        assert!(pixmap.set_pixel(3, 1, Pixel::rgb(1, 2, 3)).is_err());
        assert!(pixmap.set_pixel(1, 1, Pixel::rgba(1, 2, 3, 4)).is_err());

        pixmap
            .set_rect(IRect::new(0, 0, 2, 2), Pixel::rgb(7, 8, 9))
            .unwrap();
        assert_eq!(pixmap.pixel(0, 0).unwrap(), Pixel::rgb(7, 8, 9));
        assert_eq!(pixmap.pixel(1, 1).unwrap(), Pixel::rgb(7, 8, 9));
        assert_eq!(pixmap.color_count(), 2);

        let usage = pixmap.top_color_usage().unwrap();
        assert_eq!(usage.pixel, Pixel::new(vec![0, 0, 0]));
        assert_eq!(usage.count, 5);
        assert!((usage.ratio - 5.0 / 9.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_pixmap_alpha_digest_rect_invert_shrink_and_warp() {
        let cs = Colorspace::device_rgb();
        let mut pixmap = Pixmap::new_with_w_h(&cs, 4, 4, true).unwrap();
        pixmap.clear_with(16).unwrap();
        pixmap.set_alpha(127).unwrap();
        assert_eq!(pixmap.pixel(0, 0).unwrap(), Pixel::rgba(16, 16, 16, 127));

        let before = pixmap.digest().unwrap();
        pixmap.invert_rect(IRect::new(0, 0, 2, 2)).unwrap();
        let after = pixmap.digest().unwrap();
        assert_ne!(before, after);

        let warped = pixmap
            .warp(Rect::new(0.0, 0.0, 4.0, 4.0).quad(), 2, 2)
            .unwrap();
        assert_eq!(warped.width(), 2);
        assert_eq!(warped.height(), 2);

        pixmap.shrink(1).unwrap();
        assert_eq!(pixmap.width(), 2);
        assert_eq!(pixmap.height(), 2);

        let mut no_alpha = Pixmap::new_with_w_h(&cs, 1, 1, false).unwrap();
        assert!(no_alpha.set_alpha(255).is_err());
    }
}

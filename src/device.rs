use std::ffi::CString;
use std::ptr;

use mupdf_sys::*;

use crate::{
    context, ColorParams, Colorspace, DisplayList, Error, IRect, Image, Matrix, Path, Pixmap, Rect,
    Shade, StrokeState, Text, TextPage, TextPageOptions,
};

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub enum BlendMode {
    /* PDF 1.4 -- standard separable */
    Normal = 0,
    Multiply = 1,
    Screen = 2,
    Overlay = 3,
    Darken = 4,
    Lighten = 5,
    ColorDodge = 6,
    ColorBurn = 7,
    HardLight = 8,
    SoftLight = 9,
    Difference = 10,
    Exclusion = 11,
    /* PDF 1.4 -- standard non-separable */
    Hue = 12,
    Saturation = 13,
    Color = 14,
    Luminosity = 15,
}

#[derive(Debug)]
pub struct Device {
    pub(crate) dev: *mut fz_device,
    pub(crate) list: *mut fz_display_list,
}

impl Device {
    pub(crate) unsafe fn from_raw(dev: *mut fz_device, list: *mut fz_display_list) -> Self {
        Self { dev, list }
    }

    pub fn from_pixmap_with_clip(pixmap: &Pixmap, clip: IRect) -> Result<Self, Error> {
        let dev = unsafe { ffi_try!(mupdf_new_draw_device(context(), pixmap.inner, clip.into())) };
        Ok(Self {
            dev,
            list: ptr::null_mut(),
        })
    }

    pub fn from_pixmap(pixmap: &Pixmap) -> Result<Self, Error> {
        Self::from_pixmap_with_clip(pixmap, IRect::INF)
    }

    pub fn from_display_list(list: &DisplayList) -> Result<Self, Error> {
        let dev = unsafe { ffi_try!(mupdf_new_display_list_device(context(), list.inner)) };
        Ok(Self {
            dev,
            list: list.inner,
        })
    }

    pub fn from_text_page(page: &TextPage, opts: TextPageOptions) -> Result<Self, Error> {
        let dev = unsafe {
            ffi_try!(mupdf_new_stext_device(
                context(),
                page.inner,
                opts.bits() as _
            ))
        };
        Ok(Self {
            dev,
            list: ptr::null_mut(),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn fill_path(
        &self,
        path: &Path,
        even_odd: bool,
        ctm: &Matrix,
        cs: &Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_fill_path(
                context(),
                self.dev,
                path.inner,
                even_odd,
                ctm.into(),
                cs.inner,
                color.as_ptr(),
                alpha,
                cp.into()
            ));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn stroke_path(
        &self,
        path: &Path,
        stroke: &StrokeState,
        ctm: &Matrix,
        cs: &Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_stroke_path(
                context(),
                self.dev,
                path.inner,
                stroke.inner,
                ctm.into(),
                cs.inner,
                color.as_ptr(),
                alpha,
                cp.into()
            ));
        }
        Ok(())
    }

    pub fn clip_path(&self, path: &Path, even_odd: bool, ctm: &Matrix) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_clip_path(
                context(),
                self.dev,
                path.inner,
                even_odd,
                ctm.into()
            ));
        }
        Ok(())
    }

    pub fn clip_stroke_path(
        &self,
        path: &Path,
        stroke: &StrokeState,
        ctm: &Matrix,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_clip_stroke_path(
                context(),
                self.dev,
                path.inner,
                stroke.inner,
                ctm.into()
            ));
        }
        Ok(())
    }

    pub fn fill_text(
        &self,
        text: &Text,
        ctm: &Matrix,
        cs: &Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_fill_text(
                context(),
                self.dev,
                text.inner,
                ctm.into(),
                cs.inner,
                color.as_ptr(),
                alpha,
                cp.into()
            ));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn stroke_text(
        &self,
        text: &Text,
        stroke: &StrokeState,
        ctm: &Matrix,
        cs: &Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_stroke_text(
                context(),
                self.dev,
                text.inner,
                stroke.inner,
                ctm.into(),
                cs.inner,
                color.as_ptr(),
                alpha,
                cp.into()
            ));
        }
        Ok(())
    }

    pub fn clip_text(&self, text: &Text, ctm: &Matrix) -> Result<(), Error> {
        unsafe { ffi_try!(mupdf_clip_text(context(), self.dev, text.inner, ctm.into())) }
        Ok(())
    }

    pub fn clip_stroke_text(
        &self,
        text: &Text,
        stroke: &StrokeState,
        ctm: &Matrix,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_clip_stroke_text(
                context(),
                self.dev,
                text.inner,
                stroke.inner,
                ctm.into()
            ));
        }
        Ok(())
    }

    pub fn ignore_text(&self, text: &Text, ctm: &Matrix) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_ignore_text(
                context(),
                self.dev,
                text.inner,
                ctm.into()
            ));
        }
        Ok(())
    }

    pub fn fill_shade(
        &self,
        shd: &Shade,
        ctm: &Matrix,
        alpha: f32,
        cp: ColorParams,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_fill_shade(
                context(),
                self.dev,
                shd.inner,
                ctm.into(),
                alpha,
                cp.into()
            ));
        }
        Ok(())
    }

    pub fn fill_image(
        &self,
        image: &Image,
        ctm: &Matrix,
        alpha: f32,
        cp: ColorParams,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_fill_image(
                context(),
                self.dev,
                image.inner,
                ctm.into(),
                alpha,
                cp.into()
            ));
        }
        Ok(())
    }

    pub fn fill_image_mask(
        &self,
        image: &Image,
        ctm: &Matrix,
        cs: &Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_fill_image_mask(
                context(),
                self.dev,
                image.inner,
                ctm.into(),
                cs.inner,
                color.as_ptr(),
                alpha,
                cp.into()
            ));
        }
        Ok(())
    }

    pub fn clip_image_mask(&self, image: &Image, ctm: &Matrix) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_clip_image_mask(
                context(),
                self.dev,
                image.inner,
                ctm.into()
            ));
        }
        Ok(())
    }

    pub fn pop_clip(&self) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_pop_clip(context(), self.dev));
        }
        Ok(())
    }

    pub fn begin_mask(
        &self,
        area: Rect,
        luminosity: bool,
        cs: &Colorspace,
        bc: &[f32],
        cp: ColorParams,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_begin_mask(
                context(),
                self.dev,
                area.into(),
                luminosity,
                cs.inner,
                bc.as_ptr(),
                cp.into()
            ));
        }
        Ok(())
    }

    pub fn end_mask(&self) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_end_mask(context(), self.dev));
        }
        Ok(())
    }

    pub fn begin_group(
        &self,
        area: Rect,
        cs: &Colorspace,
        isolated: bool,
        knockout: bool,
        blend_mode: BlendMode,
        alpha: f32,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_begin_group(
                context(),
                self.dev,
                area.into(),
                cs.inner,
                isolated,
                knockout,
                blend_mode as _,
                alpha
            ));
        }
        Ok(())
    }

    pub fn end_group(&self) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_end_group(context(), self.dev));
        }
        Ok(())
    }

    pub fn begin_tile(
        &self,
        area: Rect,
        view: Rect,
        xstep: f32,
        ystep: f32,
        ctm: &Matrix,
        id: i32,
    ) -> Result<i32, Error> {
        let i = unsafe {
            ffi_try!(mupdf_begin_tile(
                context(),
                self.dev,
                area.into(),
                view.into(),
                xstep,
                ystep,
                ctm.into(),
                id
            ))
        };
        Ok(i)
    }

    pub fn end_tile(&self) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_end_tile(context(), self.dev));
        }
        Ok(())
    }

    pub fn begin_layer(&self, name: &str) -> Result<(), Error> {
        let c_name = CString::new(name)?;
        unsafe {
            ffi_try!(mupdf_begin_layer(context(), self.dev, c_name.as_ptr()));
        }
        Ok(())
    }

    pub fn end_layer(&self) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_end_layer(context(), self.dev));
        }
        Ok(())
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        if !self.dev.is_null() {
            unsafe {
                fz_close_device(context(), self.dev);
                fz_drop_device(context(), self.dev);
            }
        }
        if !self.list.is_null() {
            unsafe {
                fz_drop_display_list(context(), self.list);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{Colorspace, Device, DisplayList, Pixmap, Rect};

    #[test]
    fn test_new_device_from_pixmap() {
        let cs = Colorspace::device_rgb();
        let mut pixmap = Pixmap::new_with_w_h(&cs, 100, 100, false).expect("Pixmap::new_with_w_h");
        pixmap.clear().unwrap();
        let _device = Device::from_pixmap(&pixmap).unwrap();
    }

    #[test]
    fn test_new_device_from_display_list() {
        let list = DisplayList::new(Rect::new(0.0, 0.0, 100.0, 100.0)).unwrap();
        let _device = Device::from_display_list(&list).unwrap();
    }
}

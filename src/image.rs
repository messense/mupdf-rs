use std::{ffi::CString, marker::PhantomData};

use mupdf_sys::*;

use crate::{context, Buffer, Colorspace, DisplayList, Error, Pixmap};

#[derive(Debug)]
pub struct Image<T = ()> {
    pub(crate) inner: *mut fz_image,
    _marker: PhantomData<T>,
}

/// An image backed by a display list.
///
/// MuPDF renders this kind of image lazily from the display list it was created from, so the image
/// carries a shared borrow of that list for as long as the image is alive.
pub type DisplayListImage<'a> = Image<&'a DisplayList>;

impl<T> Image<T> {
    pub(crate) unsafe fn from_raw(image: *mut fz_image) -> Self {
        Self {
            inner: image,
            _marker: PhantomData,
        }
    }

    pub fn width(&self) -> u32 {
        unsafe { (*self.inner).w as u32 }
    }

    pub fn height(&self) -> u32 {
        unsafe { (*self.inner).h as u32 }
    }

    pub fn n(&self) -> u8 {
        unsafe { (*self.inner).n }
    }

    pub fn bits_per_components(&self) -> u8 {
        unsafe { (*self.inner).bpc }
    }

    /// The colorspace of the image.
    ///
    /// This is `None` for image masks, which have no colorspace (they are 1-bit
    /// stencils painted with a fill color).
    pub fn color_space(&self) -> Option<Colorspace> {
        let ptr = unsafe { (*self.inner).colorspace };
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { Colorspace::from_raw(ptr) })
        }
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
            let mask = (*self.inner).mask;
            if mask.is_null() {
                return None;
            }
            Some(Self::from_raw(fz_keep_image(context(), mask)))
        }
    }

    pub fn to_pixmap(&self) -> Result<Pixmap, Error> {
        unsafe { ffi_try!(mupdf_get_pixmap_from_image(context(), self.inner)) }
            .map(|inner| unsafe { Pixmap::from_raw(inner) })
    }

    pub fn interpolate(&self) -> bool {
        unsafe { (*self.inner).interpolate() > 0 }
    }

    pub fn set_interpolate(&mut self, interpolate: bool) {
        unsafe {
            (*self.inner).set_interpolate(interpolate.into());
        }
    }

    pub fn scalable(&self) -> bool {
        unsafe { (*self.inner).scalable() > 0 }
    }

    pub fn set_scalable(&mut self, scalable: bool) {
        unsafe {
            (*self.inner).set_scalable(scalable.into());
        }
    }
}

impl Image {
    pub fn from_pixmap(pixmap: &Pixmap) -> Result<Self, Error> {
        unsafe { ffi_try!(mupdf_new_image_from_pixmap(context(), pixmap.inner)) }
            .map(|inner| unsafe { Self::from_raw(inner) })
    }

    pub fn from_file(filename: &str) -> Result<Self, Error> {
        let c_filename = CString::new(filename)?;
        unsafe { ffi_try!(mupdf_new_image_from_file(context(), c_filename.as_ptr())) }
            .map(|inner| unsafe { Self::from_raw(inner) })
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let buffer = Buffer::from_bytes(bytes)?;
        unsafe { ffi_try!(mupdf_new_image_from_buffer(context(), buffer.inner)) }
            .map(|inner| unsafe { Self::from_raw(inner) })
    }

    /// Creates a lazily rendered image backed by `list`.
    ///
    /// The returned image keeps `list` immutably borrowed until the image is dropped.
    pub fn from_display_list<'a>(
        list: &'a DisplayList,
        width: f32,
        height: f32,
    ) -> Result<DisplayListImage<'a>, Error> {
        unsafe {
            ffi_try!(mupdf_new_image_from_display_list(
                context(),
                list.as_ptr(),
                width,
                height
            ))
        }
        .map(|inner| unsafe { DisplayListImage::from_raw(inner) })
    }
}

impl<T> Drop for Image<T> {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                fz_drop_image(context(), self.inner);
            }
        }
    }
}

impl<T> Clone for Image<T> {
    fn clone(&self) -> Self {
        unsafe { Self::from_raw(fz_keep_image(context(), self.inner)) }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod test {
    use std::cell::RefCell;
    use std::rc::Rc;

    use crate::{ColorParams, Colorspace, Device, Document, Image, Matrix, NativeDevice};

    /// An image mask has no colorspace. `Image::color_space()` must report
    /// `None` for it rather than handing back a `Colorspace` that wraps a NULL
    /// pointer (any later colorspace query on such a handle would dereference
    /// NULL). We render a page that paints an image mask through a
    /// `NativeDevice` and inspect the image delivered to `fill_image_mask`.
    #[test]
    fn test_image_mask_color_space_is_none() {
        struct MaskCapturer {
            image: Option<Image>,
        }
        impl NativeDevice for MaskCapturer {
            fn fill_image_mask(
                &mut self,
                img: &Image,
                _ctm: Matrix,
                _cs: &Colorspace,
                _color: &[f32],
                _alpha: f32,
                _cp: ColorParams,
            ) {
                self.image = Some(img.clone());
            }
        }

        let doc = Document::open("tests/files/image-mask.pdf").expect("open fixture");
        let page = doc.load_page(0).expect("load page");
        let state = Rc::new(RefCell::new(MaskCapturer { image: None }));
        let dev = Device::from_native(state.clone()).expect("build device");
        page.run(&dev, &Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0))
            .expect("run page");

        let captured = state
            .borrow_mut()
            .image
            .take()
            .expect("fill_image_mask was not called; fixture did not render a mask");
        assert!(
            captured.color_space().is_none(),
            "image mask must report no colorspace"
        );
    }
}

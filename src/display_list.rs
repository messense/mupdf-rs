use std::ffi::{c_int, c_void, CString};
use std::{io::Read, ptr::NonNull};

use mupdf_sys::*;

use crate::{
    array::FzArray, context, non_null, rust_vec_from_ffi_ptr, Buffer, Colorspace, Cookie, Device,
    DisplayListImage, Error, Image, Matrix, Pixmap, Quad, Rect, TextPage, TextPageFlags,
};

#[derive(Debug)]
pub struct DisplayList {
    pub(crate) inner: NonNull<fz_display_list>,
    /// The context family that recorded the list; see [`Self::check_family`].
    family: *const c_void,
}

impl DisplayList {
    /// # Safety
    ///
    /// `ptr` may be null, in which case this returns [`Error::UnexpectedNullPtr`]. If non-null, it
    /// must be a valid, well-aligned [`fz_display_list`] pointer owned by the returned wrapper.
    pub(crate) unsafe fn from_raw(ptr: *mut fz_display_list) -> Result<Self, Error> {
        Ok(Self {
            inner: non_null(ptr)?,
            family: crate::context::current_family(),
        })
    }

    pub(crate) fn as_ptr(&self) -> *mut fz_display_list {
        self.inner.as_ptr()
    }

    /// MuPDF guards reference counts with the calling context's lock, so a
    /// list may only be handed to MuPDF under a context of the family that
    /// recorded it. Threads that use the default shared context are one
    /// family; see [`init_thread_context`](crate::init_thread_context).
    pub(crate) fn check_family(&self) -> Result<(), Error> {
        if self.family == crate::context::current_family() {
            Ok(())
        } else {
            Err(Error::ForeignContext)
        }
    }

    pub fn new(media_box: Rect) -> Result<Self, Error> {
        unsafe { ffi_try!(mupdf_new_display_list(context(), media_box.into())) }
            .and_then(|inner| unsafe { Self::from_raw(inner) })
    }

    pub fn bounds(&self) -> Rect {
        let rect = unsafe { fz_bound_display_list(context(), self.as_ptr()) };
        rect.into()
    }

    pub fn to_pixmap(&self, ctm: &Matrix, cs: &Colorspace, alpha: bool) -> Result<Pixmap, Error> {
        self.check_family()?;
        unsafe {
            ffi_try!(mupdf_display_list_to_pixmap(
                context(),
                self.as_ptr(),
                ctm.into(),
                cs.inner,
                alpha
            ))
        }
        .map(|inner| unsafe { Pixmap::from_raw(inner) })
    }

    pub fn to_svg(&self, ctm: &Matrix) -> Result<String, Error> {
        self.check_family()?;
        let inner = unsafe {
            ffi_try!(mupdf_display_list_to_svg(
                context(),
                self.as_ptr(),
                ctm.into(),
                std::ptr::null_mut()
            ))
        }?;
        let mut buf = unsafe { Buffer::from_raw(inner) };
        let mut svg = String::new();
        buf.read_to_string(&mut svg)?;
        Ok(svg)
    }

    pub fn to_svg_with_cookie(&self, ctm: &Matrix, cookie: &Cookie) -> Result<String, Error> {
        self.check_family()?;
        let inner = unsafe {
            ffi_try!(mupdf_display_list_to_svg(
                context(),
                self.as_ptr(),
                ctm.into(),
                cookie.inner
            ))
        }?;
        let mut buf = unsafe { Buffer::from_raw(inner) };
        let mut svg = String::new();
        buf.read_to_string(&mut svg)?;
        Ok(svg)
    }

    pub fn to_text_page(&self, opts: TextPageFlags) -> Result<TextPage, Error> {
        self.check_family()?;
        let inner = unsafe {
            ffi_try!(mupdf_display_list_to_text_page(
                context(),
                self.as_ptr(),
                opts.bits() as _
            ))?
        };

        let inner = non_null(inner)?;

        Ok(TextPage { inner })
    }

    pub fn to_image(&self, width: f32, height: f32) -> Result<DisplayListImage<'_>, Error> {
        Image::from_display_list(self, width, height)
    }

    pub fn run(&self, device: &Device, ctm: &Matrix, area: Rect) -> Result<(), Error> {
        self.check_family()?;
        unsafe {
            ffi_try!(mupdf_display_list_run(
                context(),
                self.as_ptr(),
                device.dev,
                ctm.into(),
                area.into(),
                std::ptr::null_mut()
            ))
        }
    }

    pub fn run_with_cookie(
        &self,
        device: &Device,
        ctm: &Matrix,
        area: Rect,
        cookie: &Cookie,
    ) -> Result<(), Error> {
        self.check_family()?;
        unsafe {
            ffi_try!(mupdf_display_list_run(
                context(),
                self.as_ptr(),
                device.dev,
                ctm.into(),
                area.into(),
                cookie.inner
            ))
        }
    }

    pub fn is_empty(&self) -> bool {
        unsafe { fz_display_list_is_empty(context(), self.as_ptr()) > 0 }
    }

    pub fn search(&self, needle: &str, hit_max: u32) -> Result<FzArray<Quad>, Error> {
        self.check_family()?;
        let c_needle = CString::new(needle)?;
        let hit_max = if hit_max < 1 { 16 } else { hit_max };
        let hit_max = c_int::try_from(hit_max)?;
        let mut hit_count = 0;
        unsafe {
            ffi_try!(mupdf_search_display_list(
                context(),
                self.as_ptr(),
                c_needle.as_ptr(),
                hit_max,
                &mut hit_count
            ))
        }
        .and_then(|quads| unsafe { rust_vec_from_ffi_ptr(quads, hit_count) })
    }
}

impl Drop for DisplayList {
    fn drop(&mut self) {
        // Releasing under another family's context would race with the
        // recording thread's reference counting, so a list dropped away from
        // its family is leaked instead. This cannot happen unless a thread
        // opted into its own context; see `check_family`.
        if self.check_family().is_err() {
            return;
        }
        // SAFETY: `self.inner` is the owned display-list pointer for this wrapper and must be
        // released exactly once when the Rust wrapper is dropped.
        unsafe { fz_drop_display_list(context(), self.as_ptr()) };
    }
}

// SAFETY: MuPDF display lists may be used by multiple threads once recording has completed. Safe
// APIs that keep a display-list pointer alive carry Rust borrows for the retained access:
// `Device::from_display_list` returns a recording device with a mutable borrow, and
// `Image::from_display_list` returns an image with a shared borrow.
unsafe impl Send for DisplayList {}

// SAFETY: See the `Send` impl.
unsafe impl Sync for DisplayList {}

#[cfg(test)]
mod test {
    use crate::{document::test_document, Document};

    /// MuPDF reference counts are guarded only by the calling context's
    /// lock, so a display list must not be handed to MuPDF under a context
    /// of another family: that would race with its owner's thread.
    #[test]
    fn display_list_from_another_family_is_rejected() {
        use crate::{init_thread_context, Colorspace, DisplayList, Error, Matrix, Rect};

        let list = std::thread::spawn(|| {
            init_thread_context(None).unwrap();
            DisplayList::new(Rect::new(0.0, 0.0, 10.0, 10.0)).unwrap()
        })
        .join()
        .unwrap();

        // Plain reads of the list's own fields are fine.
        assert!(list.is_empty());
        assert_eq!(list.bounds(), Rect::new(0.0, 0.0, 10.0, 10.0));

        let cs = Colorspace::device_rgb();
        assert!(matches!(
            list.to_pixmap(&Matrix::IDENTITY, &cs, false),
            Err(Error::ForeignContext)
        ));
        assert!(matches!(
            list.to_image(10.0, 10.0),
            Err(Error::ForeignContext)
        ));
        // Dropping `list` here leaks it instead of racing.
    }

    /// Threads that share the process-wide base context are one family, so
    /// the check must not get in the way of the default configuration.
    #[test]
    fn display_list_crosses_threads_within_the_shared_family() {
        use crate::{Colorspace, DisplayList, Matrix, Rect};

        let list =
            std::thread::spawn(|| DisplayList::new(Rect::new(0.0, 0.0, 10.0, 10.0)).unwrap())
                .join()
                .unwrap();
        let cs = Colorspace::device_rgb();
        let pixmap = list.to_pixmap(&Matrix::IDENTITY, &cs, false).unwrap();
        assert_eq!((pixmap.width(), pixmap.height()), (10, 10));
    }

    #[test]
    fn test_display_list_search() {
        use crate::{Point, Quad};

        let doc = test_document!("..", "files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let list = page0.to_display_list(false).unwrap();
        let hits = list.search("Dummy", 1).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(
            &*hits,
            [Quad {
                ul: Point {
                    x: 56.8,
                    y: 69.32953
                },
                ur: Point {
                    x: 115.85159,
                    y: 69.32953
                },
                ll: Point {
                    x: 56.8,
                    y: 87.29713
                },
                lr: Point {
                    x: 115.85159,
                    y: 87.29713
                }
            }]
        );

        let hits = list.search("Not Found", 1).unwrap();
        assert_eq!(hits.len(), 0);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_multi_threaded_display_list_search() {
        use crossbeam_utils::thread;

        let doc = test_document!("..", "files/dummy.pdf").unwrap();
        let page0 = doc.load_page(0).unwrap();
        let list = page0.to_display_list(false).unwrap();

        thread::scope(|scope| {
            for _ in 0..5 {
                scope.spawn(|_| {
                    let hits = list.search("Dummy", 1).unwrap();
                    assert_eq!(hits.len(), 1);
                    let hits = list.search("Not Found", 1).unwrap();
                    assert_eq!(hits.len(), 0);
                });
            }
        })
        .unwrap();
    }
}

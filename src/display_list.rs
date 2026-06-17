use std::{
    ffi::CString,
    io::Read,
    ptr::NonNull,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use mupdf_sys::*;

use crate::{
    array::FzArray, context, non_null, rust_vec_from_ffi_ptr, Buffer, Colorspace, Cookie, Device,
    Error, Image, Matrix, Pixmap, Quad, Rect, TextPage, TextPageFlags,
};

const DISPLAY_LIST_RECORDING: usize = 1 << (usize::BITS - 1);

fn display_list_recording_error() -> Error {
    Error::InvalidArgument("display list is currently being recorded".into())
}

#[derive(Debug)]
pub(crate) struct DisplayListReadGuard {
    access: Arc<AtomicUsize>,
}

impl Drop for DisplayListReadGuard {
    fn drop(&mut self) {
        self.access.fetch_sub(1, Ordering::Release);
    }
}

#[derive(Debug)]
pub(crate) struct DisplayListRecordingGuard {
    access: Arc<AtomicUsize>,
}

impl Drop for DisplayListRecordingGuard {
    fn drop(&mut self) {
        self.access.store(0, Ordering::Release);
    }
}

#[derive(Debug)]
pub struct DisplayList {
    pub(crate) inner: NonNull<fz_display_list>,
    access: Arc<AtomicUsize>,
}

impl DisplayList {
    /// # Safety
    ///
    /// `ptr` may be null, in which case this returns [`Error::UnexpectedNullPtr`]. If non-null, it
    /// must be a valid, well-aligned [`fz_display_list`] pointer owned by the returned wrapper.
    pub(crate) unsafe fn from_raw(ptr: *mut fz_display_list) -> Result<Self, Error> {
        Ok(Self {
            inner: non_null(ptr)?,
            access: Arc::new(AtomicUsize::new(0)),
        })
    }

    pub(crate) fn as_ptr(&self) -> *mut fz_display_list {
        self.inner.as_ptr()
    }

    pub(crate) fn read_guard(&self) -> Result<DisplayListReadGuard, Error> {
        loop {
            let state = self.access.load(Ordering::Acquire);
            if state & DISPLAY_LIST_RECORDING != 0 {
                return Err(display_list_recording_error());
            }
            if state == DISPLAY_LIST_RECORDING - 1 {
                return Err(Error::InvalidArgument(
                    "too many concurrent display list readers".into(),
                ));
            }
            if self
                .access
                .compare_exchange_weak(state, state + 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return Ok(DisplayListReadGuard {
                    access: Arc::clone(&self.access),
                });
            }
        }
    }

    pub(crate) fn recording_guard(&self) -> Result<DisplayListRecordingGuard, Error> {
        self.access
            .compare_exchange(
                0,
                DISPLAY_LIST_RECORDING,
                Ordering::AcqRel,
                Ordering::Relaxed,
            )
            .map(|_| DisplayListRecordingGuard {
                access: Arc::clone(&self.access),
            })
            .map_err(|_| display_list_recording_error())
    }

    fn read_guard_or_panic(&self) -> DisplayListReadGuard {
        self.read_guard()
            .expect("display list is currently being recorded")
    }

    pub fn new(media_box: Rect) -> Result<Self, Error> {
        unsafe { ffi_try!(mupdf_new_display_list(context(), media_box.into())) }
            .and_then(|inner| unsafe { Self::from_raw(inner) })
    }

    pub fn bounds(&self) -> Rect {
        let _guard = self.read_guard_or_panic();
        let rect = unsafe { fz_bound_display_list(context(), self.as_ptr()) };
        rect.into()
    }

    pub fn to_pixmap(&self, ctm: &Matrix, cs: &Colorspace, alpha: bool) -> Result<Pixmap, Error> {
        let _guard = self.read_guard()?;
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
        let _guard = self.read_guard()?;
        let inner = unsafe {
            ffi_try!(mupdf_display_list_to_svg(
                context(),
                self.as_ptr(),
                ctm.into(),
                ptr::null_mut()
            ))
        }?;
        let mut buf = unsafe { Buffer::from_raw(inner) };
        let mut svg = String::new();
        buf.read_to_string(&mut svg)?;
        Ok(svg)
    }

    pub fn to_svg_with_cookie(&self, ctm: &Matrix, cookie: &Cookie) -> Result<String, Error> {
        let _guard = self.read_guard()?;
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
        let _guard = self.read_guard()?;
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

    pub fn to_image(&self, width: f32, height: f32) -> Result<Image, Error> {
        Image::from_display_list(self, width, height)
    }

    pub fn run(&self, device: &Device, ctm: &Matrix, area: Rect) -> Result<(), Error> {
        let _guard = self.read_guard()?;
        unsafe {
            ffi_try!(mupdf_display_list_run(
                context(),
                self.as_ptr(),
                device.dev,
                ctm.into(),
                area.into(),
                ptr::null_mut()
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
        let _guard = self.read_guard()?;
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
        let _guard = self.read_guard_or_panic();
        unsafe { fz_display_list_is_empty(context(), self.as_ptr()) > 0 }
    }

    pub fn search(&self, needle: &str, hit_max: u32) -> Result<FzArray<Quad>, Error> {
        let _guard = self.read_guard()?;
        let c_needle = CString::new(needle)?;
        let hit_max = if hit_max < 1 { 16 } else { hit_max };
        let mut hit_count = 0;
        unsafe {
            ffi_try!(mupdf_search_display_list(
                context(),
                self.as_ptr(),
                c_needle.as_ptr(),
                hit_max as i32,
                &mut hit_count
            ))
        }
        .and_then(|quads| unsafe { rust_vec_from_ffi_ptr(quads, hit_count) })
    }
}

impl Drop for DisplayList {
    fn drop(&mut self) {
        // SAFETY: `self.inner` is the owned display-list pointer for this wrapper and must be
        // released exactly once when the Rust wrapper is dropped.
        unsafe { fz_drop_display_list(context(), self.as_ptr()) };
    }
}

// SAFETY: `DisplayList` coordinates access with an atomic reader/recording state. Read-only MuPDF
// operations may run concurrently, while `Device::from_display_list` holds an exclusive recording
// guard until that device is dropped.
unsafe impl Send for DisplayList {}

// SAFETY: See the `Send` impl.
unsafe impl Sync for DisplayList {}

#[cfg(test)]
mod test {
    use crate::{document::test_document, Document};

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

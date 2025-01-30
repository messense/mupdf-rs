use std::mem;
use std::os::raw::c_void;

use mupdf_sys::*;

use crate::{context, Error, Matrix, Point, Rect, StrokeState};

pub trait PathWalker {
    fn move_to(&mut self, x: f32, y: f32);
    fn line_to(&mut self, x: f32, y: f32);
    fn curve_to(&mut self, cx1: f32, cy1: f32, cx2: f32, cy2: f32, ex: f32, ey: f32);
    fn close(&mut self);

    fn curve_to_y(&mut self, cx: f32, cy: f32, ex: f32, ey: f32) {
        self.curve_to(cx, cy, ex, ey, ex, ey);
    }
    fn rect(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        self.move_to(x1, y1);
        self.line_to(x2, y1);
        self.line_to(x2, y2);
        self.line_to(x1, y2);
        self.close();
    }
}

impl<W: PathWalker + ?Sized> PathWalker for &mut W {
    fn move_to(&mut self, x: f32, y: f32) {
        (**self).move_to(x, y)
    }
    fn line_to(&mut self, x: f32, y: f32) {
        (**self).line_to(x, y)
    }
    fn curve_to(&mut self, cx1: f32, cy1: f32, cx2: f32, cy2: f32, ex: f32, ey: f32) {
        (**self).curve_to(cx1, cy1, cx2, cy2, ex, ey)
    }
    fn close(&mut self) {
        (**self).close()
    }

    fn curve_to_y(&mut self, cx: f32, cy: f32, ex: f32, ey: f32) {
        (**self).curve_to_y(cx, cy, ex, ey)
    }
    fn rect(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        (**self).rect(x1, y1, x2, y2)
    }
}

unsafe fn with_path_walker<W: PathWalker>(arg: *mut c_void, f: impl FnOnce(&mut W)) {
    let mut walker: Box<W> = unsafe { Box::from_raw(arg.cast()) };
    f(&mut walker);
    mem::forget(walker);
}

unsafe extern "C" fn path_walk_move_to<W: PathWalker>(
    _ctx: *mut fz_context,
    arg: *mut c_void,
    x: f32,
    y: f32,
) {
    with_path_walker::<W>(arg, |walker| walker.move_to(x, y));
}

unsafe extern "C" fn path_walk_line_to<W: PathWalker>(
    _ctx: *mut fz_context,
    arg: *mut c_void,
    x: f32,
    y: f32,
) {
    with_path_walker::<W>(arg, |walker| walker.line_to(x, y));
}

unsafe extern "C" fn path_walk_curve_to<W: PathWalker>(
    _ctx: *mut fz_context,
    arg: *mut c_void,
    cx1: f32,
    cy1: f32,
    cx2: f32,
    cy2: f32,
    ex: f32,
    ey: f32,
) {
    with_path_walker::<W>(arg, |walker| walker.curve_to(cx1, cy1, cx2, cy2, ex, ey));
}

unsafe extern "C" fn path_walk_close<W: PathWalker>(_ctx: *mut fz_context, arg: *mut c_void) {
    with_path_walker::<W>(arg, |walker| walker.close());
}

unsafe extern "C" fn path_walk_curve_to_y<W: PathWalker>(
    _ctx: *mut fz_context,
    arg: *mut c_void,
    cx: f32,
    cy: f32,
    ex: f32,
    ey: f32,
) {
    with_path_walker::<W>(arg, |walker| walker.curve_to_y(cx, cy, ex, ey));
}

unsafe extern "C" fn path_walk_rect<W: PathWalker>(
    _ctx: *mut fz_context,
    arg: *mut c_void,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
) {
    with_path_walker::<W>(arg, |walker| walker.rect(x1, y1, x2, y2));
}

#[derive(Debug)]
pub struct Path {
    pub(crate) inner: *mut fz_path,
}

impl Path {
    pub(crate) unsafe fn from_raw(ptr: *mut fz_path) -> Self {
        Self { inner: ptr }
    }

    pub fn new() -> Result<Self, Error> {
        let inner = unsafe { ffi_try!(mupdf_new_path(context())) };
        Ok(Self { inner })
    }

    pub fn try_clone(&self) -> Result<Self, Error> {
        let inner = unsafe { ffi_try!(mupdf_clone_path(context(), self.inner)) };
        Ok(Self { inner })
    }

    pub fn walk<W: PathWalker>(&self, walker: W) -> Result<(), Error> {
        unsafe {
            let c_walker = fz_path_walker {
                moveto: Some(path_walk_move_to::<W>),
                lineto: Some(path_walk_line_to::<W>),
                curveto: Some(path_walk_curve_to::<W>),
                closepath: Some(path_walk_close::<W>),
                quadto: None,
                curvetov: None,
                curvetoy: Some(path_walk_curve_to_y::<W>),
                rectto: Some(path_walk_rect::<W>),
            };
            let raw_ptr = Box::into_raw(Box::new(walker));
            ffi_try!(mupdf_walk_path(
                context(),
                self.inner,
                &c_walker,
                raw_ptr.cast()
            ));
            drop(Box::from_raw(raw_ptr));
        }
        Ok(())
    }

    pub fn current_point(&self) -> Point {
        let inner = unsafe { fz_currentpoint(context(), self.inner) };
        inner.into()
    }

    pub fn move_to(&mut self, x: f32, y: f32) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_moveto(context(), self.inner, x, y));
        }
        Ok(())
    }

    pub fn line_to(&mut self, x: f32, y: f32) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_lineto(context(), self.inner, x, y));
        }
        Ok(())
    }

    pub fn curve_to(
        &mut self,
        cx1: f32,
        cy1: f32,
        cx2: f32,
        cy2: f32,
        ex: f32,
        ey: f32,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_curveto(
                context(),
                self.inner,
                cx1,
                cy1,
                cx2,
                cy2,
                ex,
                ey
            ));
        }
        Ok(())
    }

    pub fn curve_to_v(&mut self, cx: f32, cy: f32, ex: f32, ey: f32) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_curvetov(context(), self.inner, cx, cy, ex, ey));
        }
        Ok(())
    }

    pub fn curve_to_y(&mut self, cx: f32, cy: f32, ex: f32, ey: f32) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_curvetoy(context(), self.inner, cx, cy, ex, ey));
        }
        Ok(())
    }

    pub fn rect(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_rectto(context(), self.inner, x1, y1, x2, y2));
        }
        Ok(())
    }

    pub fn close(&mut self) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_closepath(context(), self.inner));
        }
        Ok(())
    }

    pub fn transform(&mut self, mat: &Matrix) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_transform_path(context(), self.inner, mat.into()));
        }
        Ok(())
    }

    pub fn bounds(&self, stroke: &StrokeState, ctm: &Matrix) -> Result<Rect, Error> {
        let rect = unsafe {
            ffi_try!(mupdf_bound_path(
                context(),
                self.inner,
                stroke.inner,
                ctm.into()
            ))
        };
        Ok(rect.into())
    }

    pub fn trim(&mut self) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_trim_path(context(), self.inner));
        }
        Ok(())
    }
}

impl Drop for Path {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                fz_drop_path(context(), self.inner);
            }
        }
    }
}

impl Clone for Path {
    fn clone(&self) -> Self {
        self.try_clone().unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::{Path, PathWalker};

    #[derive(Default)]
    struct TestPathWalker {
        move_to: bool,
        line_to: bool,
        curve_to: bool,
        close: u8,
        curve_to_y: bool,
        rect: bool,
    }

    impl PathWalker for TestPathWalker {
        fn move_to(&mut self, x: f32, y: f32) {
            if x == 0.0 && y == 0.0 {
                self.move_to = true;
            }
        }

        fn line_to(&mut self, x: f32, y: f32) {
            if x == 10.0 && y == 10.0 {
                self.line_to = true;
            }
            if x == -5.0 && y == 5.0 {
                self.rect = true;
            }
        }

        fn curve_to(&mut self, _cx1: f32, _cy1: f32, _cx2: f32, _cy2: f32, _ex: f32, _ey: f32) {
            self.curve_to = true;
        }

        fn close(&mut self) {
            self.close += 1;
        }

        fn curve_to_y(&mut self, cx: f32, cy: f32, ex: f32, ey: f32) {
            if cx == 5.0 && cy == 10.0 && ex == 20.0 && ey == 30.0 {
                self.curve_to_y = true;
            }
        }
    }

    #[test]
    fn test_walk_path() {
        let mut path = Path::new().unwrap();
        path.move_to(0.0, 0.0).unwrap();
        path.line_to(10.0, 10.0).unwrap();
        path.close().unwrap();
        path.curve_to_y(5.0, 10.0, 20.0, 30.0).unwrap();
        path.rect(-5.0, -5.0, 5.0, 5.0).unwrap();
        path.close().unwrap(); // close after rect is ignored

        let mut walker = TestPathWalker::default();
        path.walk(&mut walker).unwrap();

        assert!(walker.move_to);
        assert!(walker.line_to);
        assert!(!walker.curve_to);
        assert_eq!(walker.close, 2);
        assert!(walker.curve_to_y);

        let mut dyn_walker: &mut dyn PathWalker = &mut TestPathWalker::default();
        path.walk(dyn_walker).unwrap();
    }
}

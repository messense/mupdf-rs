use mupdf_sys::*;

use crate::{context, Error, Matrix, Point, Rect, StrokeState};

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

    pub fn rect(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) -> Result<(), Error> {
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

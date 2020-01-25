use mupdf_sys::*;

use crate::{context, Error};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineCap {
    Butt = 0,
    Round = 1,
    Square = 2,
    Triangle = 3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineJoin {
    Miter = 0,
    Round = 1,
    Bevel = 2,
    MiterXps = 3,
}

#[derive(Debug)]
pub struct StrokeState {
    pub(crate) inner: *mut fz_stroke_state,
}

impl StrokeState {
    pub fn new(
        start_cap: u32,
        dash_cap: u32,
        end_cap: u32,
        line_join: u32,
        line_width: f32,
        miter_limit: f32,
        dash_phase: f32,
        dash: &[f32],
    ) -> Result<Self, Error> {
        let dash_len = dash.len() as i32;
        let inner = unsafe {
            ffi_try!(mupdf_new_stroke_state(
                context(),
                start_cap,
                dash_cap,
                end_cap,
                line_join,
                line_width,
                miter_limit,
                dash_phase,
                dash.as_ptr(),
                dash_len
            ))
        };
        Ok(Self { inner })
    }

    pub fn start_cap(&self) -> u32 {
        unsafe { (*self.inner).start_cap }
    }

    pub fn dash_cap(&self) -> u32 {
        unsafe { (*self.inner).dash_cap }
    }

    pub fn end_cap(&self) -> u32 {
        unsafe { (*self.inner).end_cap }
    }

    pub fn line_join(&self) -> u32 {
        unsafe { (*self.inner).linejoin }
    }

    pub fn line_width(&self) -> f32 {
        unsafe { (*self.inner).linewidth }
    }

    pub fn miter_limit(&self) -> f32 {
        unsafe { (*self.inner).miterlimit }
    }

    pub fn dash_phase(&self) -> f32 {
        unsafe { (*self.inner).dash_phase }
    }

    pub fn dashes(&self) -> Vec<f32> {
        todo!()
    }
}

impl Drop for StrokeState {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                fz_drop_stroke_state(context(), self.inner);
            }
        }
    }
}

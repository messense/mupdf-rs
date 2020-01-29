use std::convert::TryFrom;

use mupdf_sys::*;
use num_enum::TryFromPrimitive;

use crate::{context, Error};

#[derive(Debug, Clone, Copy, PartialEq, TryFromPrimitive)]
#[repr(u32)]
pub enum LineCap {
    Butt = 0,
    Round = 1,
    Square = 2,
    Triangle = 3,
}

impl Default for LineCap {
    fn default() -> Self {
        Self::Butt
    }
}

#[derive(Debug, Clone, Copy, PartialEq, TryFromPrimitive)]
#[repr(u32)]
pub enum LineJoin {
    Miter = 0,
    Round = 1,
    Bevel = 2,
    MiterXps = 3,
}

impl Default for LineJoin {
    fn default() -> Self {
        Self::Miter
    }
}

#[derive(Debug)]
pub struct StrokeState {
    pub(crate) inner: *mut fz_stroke_state,
}

impl StrokeState {
    pub fn new(
        start_cap: LineCap,
        dash_cap: LineCap,
        end_cap: LineCap,
        line_join: LineJoin,
        line_width: f32,
        miter_limit: f32,
        dash_phase: f32,
        dash: &[f32],
    ) -> Result<Self, Error> {
        let dash_len = dash.len() as i32;
        let inner = unsafe {
            ffi_try!(mupdf_new_stroke_state(
                context(),
                start_cap as fz_linecap,
                dash_cap as fz_linecap,
                end_cap as fz_linecap,
                line_join as fz_linejoin,
                line_width,
                miter_limit,
                dash_phase,
                dash.as_ptr(),
                dash_len
            ))
        };
        Ok(Self { inner })
    }

    pub fn try_clone(&self) -> Result<Self, Error> {
        let start_cap = self.start_cap();
        let dash_cap = self.dash_cap();
        let end_cap = self.end_cap();
        let line_join = self.line_join();
        let line_width = self.line_width();
        let miter_limit = self.miter_limit();
        let dash_phase = self.dash_phase();
        let dashes = self.dashes();
        Self::new(
            start_cap,
            dash_cap,
            end_cap,
            line_join,
            line_width,
            miter_limit,
            dash_phase,
            &dashes,
        )
    }

    pub fn start_cap(&self) -> LineCap {
        LineCap::try_from(unsafe { (*self.inner).start_cap }).unwrap()
    }

    pub fn dash_cap(&self) -> LineCap {
        LineCap::try_from(unsafe { (*self.inner).dash_cap }).unwrap()
    }

    pub fn end_cap(&self) -> LineCap {
        LineCap::try_from(unsafe { (*self.inner).end_cap }).unwrap()
    }

    pub fn line_join(&self) -> LineJoin {
        LineJoin::try_from(unsafe { (*self.inner).linejoin }).unwrap()
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
        unsafe {
            let dash_len = (*self.inner).dash_len as usize;
            let mut dash_list = Vec::with_capacity(dash_len);
            dash_list.extend_from_slice(&(*self.inner).dash_list[0..dash_len]);
            dash_list
        }
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

impl Clone for StrokeState {
    fn clone(&self) -> StrokeState {
        self.try_clone().unwrap()
    }
}

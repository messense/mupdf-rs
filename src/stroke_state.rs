use std::convert::TryFrom;

use mupdf_sys::*;

use crate::{context, from_enum, Error};

from_enum! { fz_linecap,
    #[derive(Debug, Default, Clone, Copy, PartialEq)]
    pub enum LineCap {
        #[default]
        Butt = FZ_LINECAP_BUTT,
        Round = FZ_LINECAP_ROUND,
        Square = FZ_LINECAP_SQUARE,
        Triangle = FZ_LINECAP_TRIANGLE,
    }
}

from_enum! { fz_linejoin,
    #[derive(Debug, Default, Clone, Copy, PartialEq)]
    pub enum LineJoin {
        #[default]
        Miter = FZ_LINEJOIN_MITER,
        Round = FZ_LINEJOIN_ROUND,
        Bevel = FZ_LINEJOIN_BEVEL,
        MiterXps = FZ_LINEJOIN_MITER_XPS,
    }
}

#[derive(Debug)]
pub struct StrokeState {
    pub(crate) inner: *mut fz_stroke_state,
}

impl StrokeState {
    #[allow(clippy::too_many_arguments)]
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
        unsafe {
            ffi_try!(mupdf_new_stroke_state(
                context(),
                start_cap.into(),
                dash_cap.into(),
                end_cap.into(),
                line_join.into(),
                line_width,
                miter_limit,
                dash_phase,
                dash.as_ptr(),
                dash_len
            ))
        }
        .map(|inner| Self { inner })
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
            let dash_ptr = (*self.inner).dash_list.as_ptr();
            let mut dash_list = Vec::with_capacity(dash_len);
            dash_list.extend_from_slice(std::slice::from_raw_parts(dash_ptr, dash_len));
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

impl Default for StrokeState {
    fn default() -> Self {
        let inner = unsafe { mupdf_default_stroke_state(context()) };
        Self { inner }
    }
}

#[cfg(test)]
mod test {
    use super::StrokeState;

    #[test]
    fn test_default_stroke_state() {
        let _stroke = StrokeState::default();
    }
}

use std::{ffi::c_int, mem, ptr, slice};

use mupdf_sys::*;

use crate::{context, ColorParams, Colorspace, Device, Matrix, Path, StrokeState, Text};

#[allow(unused_variables)]
pub trait CustomDevice {
    fn close_device(&mut self) {}

    fn fill_path(
        &mut self,
        path: &Path,
        even_odd: bool,
        cmt: Matrix,
        color_space: Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) {
    }

    fn stroke_path(
        &mut self,
        path: &Path,
        stroke_state: &StrokeState,
        cmt: Matrix,
        color_space: Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) {
    }

    fn fill_text(
        &mut self,
        text: &Text,
        cmt: Matrix,
        color_space: Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) {
    }
}

impl<T: CustomDevice + ?Sized> CustomDevice for &mut T {
    fn close_device(&mut self) {
        (**self).close_device();
    }

    fn fill_path(
        &mut self,
        path: &Path,
        even_odd: bool,
        cmt: Matrix,
        color_space: Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) {
        (**self).fill_path(path, even_odd, cmt, color_space, color, alpha, cp);
    }

    fn stroke_path(
        &mut self,
        path: &Path,
        stroke_state: &StrokeState,
        cmt: Matrix,
        color_space: Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) {
        (**self).stroke_path(path, stroke_state, cmt, color_space, color, alpha, cp);
    }

    fn fill_text(
        &mut self,
        text: &Text,
        cmt: Matrix,
        color_space: Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) {
        (**self).fill_text(text, cmt, color_space, color, alpha, cp);
    }
}

pub(crate) fn create<D: CustomDevice>(device: D) -> Device {
    let d = unsafe {
        let c_device = mupdf_new_derived_device::<CDevice<D>>(context(), c"RustDevice".as_ptr());
        ptr::write(&raw mut (*c_device).rust_device, device);

        (*c_device).base.close_device = Some(close_device::<D>);
        (*c_device).base.drop_device = Some(drop_device::<D>);
        (*c_device).base.fill_path = Some(fill_path::<D>);
        (*c_device).base.stroke_path = Some(stroke_path::<D>);
        (*c_device).base.fill_text = Some(fill_text::<D>);

        Device::from_raw(c_device.cast(), ptr::null_mut())
    };
    d
}

#[repr(C)]
struct CDevice<D> {
    base: fz_device,
    rust_device: D,
}

unsafe fn with_rust_device<'a, D: CustomDevice>(dev: *mut fz_device, f: impl FnOnce(&mut D)) {
    let c_device: *mut CDevice<D> = dev.cast();
    let rust_device = &mut (*c_device).rust_device;
    f(rust_device);
    let _ = rust_device;
}

unsafe extern "C" fn close_device<D: CustomDevice>(_ctx: *mut fz_context, dev: *mut fz_device) {
    with_rust_device::<D>(dev, |dev| dev.close_device());
}

unsafe extern "C" fn drop_device<D: CustomDevice>(_ctx: *mut fz_context, dev: *mut fz_device) {
    let c_device: *mut CDevice<D> = dev.cast();
    let rust_device = &raw mut (*c_device).rust_device;

    ptr::drop_in_place(rust_device);
}

unsafe extern "C" fn fill_path<D: CustomDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    path: *const fz_path,
    even_odd: c_int,
    cmt: fz_matrix,
    color_space: *mut fz_colorspace,
    color: *const f32,
    alpha: f32,
    color_params: fz_color_params,
) {
    with_rust_device::<D>(dev, |dev| {
        let cs = Colorspace::from_raw(color_space);
        let cs_n = cs.n() as usize;

        let path = Path::from_raw(path.cast_mut());

        dev.fill_path(
            &path,
            even_odd != 0,
            cmt.into(),
            cs,
            slice::from_raw_parts(color, cs_n),
            alpha,
            color_params.into(),
        );

        mem::forget(path);
    });
}

unsafe extern "C" fn stroke_path<D: CustomDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    path: *const fz_path,
    stroke_state: *const fz_stroke_state,
    cmt: fz_matrix,
    color_space: *mut fz_colorspace,
    color: *const f32,
    alpha: f32,
    color_params: fz_color_params,
) {
    with_rust_device::<D>(dev, |dev| {
        let cs = Colorspace::from_raw(color_space);
        let cs_n = cs.n() as usize;

        let path = Path::from_raw(path.cast_mut());
        let stroke_state = StrokeState {
            inner: stroke_state.cast_mut(),
        };

        dev.stroke_path(
            &path,
            &stroke_state,
            cmt.into(),
            cs,
            slice::from_raw_parts(color, cs_n),
            alpha,
            color_params.into(),
        );

        mem::forget(stroke_state);
        mem::forget(path);
    });
}

unsafe extern "C" fn fill_text<D: CustomDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    text: *const fz_text,
    cmt: fz_matrix,
    color_space: *mut fz_colorspace,
    color: *const f32,
    alpha: f32,
    color_params: fz_color_params,
) {
    with_rust_device::<D>(dev, |dev| {
        let text = Text {
            inner: text.cast_mut(),
        };

        let cs = Colorspace::from_raw(color_space);
        let cs_n = cs.n() as usize;
        dev.fill_text(
            &text,
            cmt.into(),
            cs,
            slice::from_raw_parts(color, cs_n),
            alpha,
            color_params.into(),
        );

        mem::forget(text);
    });
}

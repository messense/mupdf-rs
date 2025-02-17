use std::{
    ffi::{c_char, c_int, CStr},
    mem::ManuallyDrop,
    num::NonZero,
    ptr, slice,
};

use mupdf_sys::*;

use crate::{
    context, BlendMode, ColorParams, Colorspace, Device, Function, Image, Matrix, Path, Rect,
    Shade, StrokeState, Text,
};

use super::{DefaultColorspaces, DeviceFlag, Metatext, Structure};

#[allow(unused_variables, clippy::too_many_arguments)]
pub trait NativeDevice {
    fn close_device(&mut self) {}

    fn fill_path(
        &mut self,
        path: &Path,
        even_odd: bool,
        cmt: Matrix,
        color_space: &Colorspace,
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
        color_space: &Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) {
    }

    fn clip_path(&mut self, path: &Path, even_odd: bool, cmt: Matrix, scissor: Rect) {}

    fn clip_stroke_path(
        &mut self,
        path: &Path,
        stroke_state: &StrokeState,
        cmt: Matrix,
        scissor: Rect,
    ) {
    }

    fn fill_text(
        &mut self,
        text: &Text,
        cmt: Matrix,
        color_space: &Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) {
    }

    fn stroke_text(
        &mut self,
        text: &Text,
        stroke_state: &StrokeState,
        cmt: Matrix,
        color_space: &Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) {
    }

    fn clip_text(&mut self, text: &Text, cmt: Matrix, scissor: Rect) {}

    fn clip_stroke_text(
        &mut self,
        text: &Text,
        stroke_state: &StrokeState,
        cmt: Matrix,
        scissor: Rect,
    ) {
    }

    fn ignore_text(&mut self, text: &Text, cmt: Matrix) {}

    fn fill_shade(&mut self, shade: &Shade, cmt: Matrix, alpha: f32, cp: ColorParams) {}

    fn fill_image(&mut self, img: &Image, cmt: Matrix, alpha: f32, cp: ColorParams) {}

    fn fill_image_mask(
        &mut self,
        img: &Image,
        cmt: Matrix,
        color_space: &Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) {
    }

    fn clip_image_mask(&mut self, img: &Image, cmt: Matrix, scissor: Rect) {}

    fn pop_clip(&mut self) {}

    fn begin_mask(
        &mut self,
        area: Rect,
        luminosity: bool,
        color_space: &Colorspace,
        color: &[f32],
        cp: ColorParams,
    ) {
    }

    fn end_mask(&mut self, f: &Function) {}

    fn begin_group(
        &mut self,
        area: Rect,
        cs: &Colorspace,
        isolated: bool,
        knockout: bool,
        blendmode: BlendMode,
        alpha: f32,
    ) {
    }

    fn end_group(&mut self) {}

    fn begin_tile(
        &mut self,
        area: Rect,
        view: Rect,
        x_step: f32,
        y_step: f32,
        ctm: Matrix,
        id: Option<NonZero<i32>>,
    ) -> Option<NonZero<i32>> {
        None
    }

    fn end_tile(&mut self) {}

    fn render_flags(&mut self, set: DeviceFlag, clear: DeviceFlag) {}

    fn set_default_colorspaces(&mut self, default_cs: &DefaultColorspaces) {}

    fn begin_layer(&mut self, name: &str) {}

    fn end_layer(&mut self) {}

    fn begin_structure(&mut self, standard: Structure, raw: &str, idx: i32) {}

    fn end_structure(&mut self) {}

    fn begin_metatext(&mut self, meta: Metatext, text: &str) {}

    fn end_metatext(&mut self) {}
}

impl<T: NativeDevice + ?Sized> NativeDevice for &mut T {
    fn close_device(&mut self) {
        (**self).close_device();
    }

    fn fill_path(
        &mut self,
        path: &Path,
        even_odd: bool,
        cmt: Matrix,
        color_space: &Colorspace,
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
        color_space: &Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) {
        (**self).stroke_path(path, stroke_state, cmt, color_space, color, alpha, cp);
    }

    fn clip_path(&mut self, path: &Path, even_odd: bool, cmt: Matrix, scissor: Rect) {
        (**self).clip_path(path, even_odd, cmt, scissor);
    }

    fn clip_stroke_path(
        &mut self,
        path: &Path,
        stroke_state: &StrokeState,
        cmt: Matrix,
        scissor: Rect,
    ) {
        (**self).clip_stroke_path(path, stroke_state, cmt, scissor)
    }

    fn fill_text(
        &mut self,
        text: &Text,
        cmt: Matrix,
        color_space: &Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) {
        (**self).fill_text(text, cmt, color_space, color, alpha, cp);
    }

    fn stroke_text(
        &mut self,
        text: &Text,
        stroke_state: &StrokeState,
        cmt: Matrix,
        color_space: &Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) {
        (**self).stroke_text(text, stroke_state, cmt, color_space, color, alpha, cp);
    }

    fn clip_text(&mut self, text: &Text, cmt: Matrix, scissor: Rect) {
        (**self).clip_text(text, cmt, scissor);
    }

    fn clip_stroke_text(
        &mut self,
        text: &Text,
        stroke_state: &StrokeState,
        cmt: Matrix,
        scissor: Rect,
    ) {
        (**self).clip_stroke_text(text, stroke_state, cmt, scissor);
    }

    fn ignore_text(&mut self, text: &Text, cmt: Matrix) {
        (**self).ignore_text(text, cmt);
    }

    fn fill_shade(&mut self, shade: &Shade, cmt: Matrix, alpha: f32, cp: ColorParams) {
        (**self).fill_shade(shade, cmt, alpha, cp);
    }

    fn fill_image(&mut self, img: &Image, cmt: Matrix, alpha: f32, cp: ColorParams) {
        (**self).fill_image(img, cmt, alpha, cp);
    }

    fn fill_image_mask(
        &mut self,
        img: &Image,
        cmt: Matrix,
        color_space: &Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) {
        (**self).fill_image_mask(img, cmt, color_space, color, alpha, cp);
    }

    fn clip_image_mask(&mut self, img: &Image, cmt: Matrix, scissor: Rect) {
        (**self).clip_image_mask(img, cmt, scissor);
    }

    fn pop_clip(&mut self) {
        (**self).pop_clip();
    }

    fn begin_mask(
        &mut self,
        area: Rect,
        luminosity: bool,
        color_space: &Colorspace,
        color: &[f32],
        cp: ColorParams,
    ) {
        (**self).begin_mask(area, luminosity, color_space, color, cp);
    }

    fn end_mask(&mut self, f: &Function) {
        (**self).end_mask(f);
    }

    fn begin_group(
        &mut self,
        area: Rect,
        cs: &Colorspace,
        isolated: bool,
        knockout: bool,
        blendmode: BlendMode,
        alpha: f32,
    ) {
        (**self).begin_group(area, cs, isolated, knockout, blendmode, alpha);
    }

    fn end_group(&mut self) {
        (**self).end_group();
    }

    fn begin_tile(
        &mut self,
        area: Rect,
        view: Rect,
        x_step: f32,
        y_step: f32,
        ctm: Matrix,
        id: Option<NonZero<i32>>,
    ) -> Option<NonZero<i32>> {
        (**self).begin_tile(area, view, x_step, y_step, ctm, id)
    }

    fn end_tile(&mut self) {
        (**self).end_tile();
    }

    fn render_flags(&mut self, set: DeviceFlag, clear: DeviceFlag) {
        (**self).render_flags(set, clear);
    }

    fn set_default_colorspaces(&mut self, default_cs: &DefaultColorspaces) {
        (**self).set_default_colorspaces(default_cs);
    }

    fn begin_layer(&mut self, name: &str) {
        (**self).begin_layer(name);
    }

    fn end_layer(&mut self) {
        (**self).end_layer();
    }

    fn begin_structure(&mut self, standard: Structure, raw: &str, idx: i32) {
        (**self).begin_structure(standard, raw, idx);
    }

    fn end_structure(&mut self) {
        (**self).end_structure();
    }

    fn begin_metatext(&mut self, meta: Metatext, text: &str) {
        (**self).begin_metatext(meta, text);
    }

    fn end_metatext(&mut self) {
        (**self).end_metatext();
    }
}

pub(crate) fn create<D: NativeDevice>(device: D) -> Device {
    unsafe {
        let c_device = mupdf_new_derived_device::<CDevice<D>>(context(), c"RustDevice".as_ptr());
        ptr::write(&raw mut (*c_device).rust_device, device);

        (*c_device).base.close_device = Some(close_device::<D>);
        (*c_device).base.drop_device = Some(drop_device::<D>);

        (*c_device).base.fill_path = Some(fill_path::<D>);
        (*c_device).base.stroke_path = Some(stroke_path::<D>);
        (*c_device).base.clip_path = Some(clip_path::<D>);
        (*c_device).base.clip_stroke_path = Some(clip_stroke_path::<D>);

        (*c_device).base.fill_text = Some(fill_text::<D>);
        (*c_device).base.stroke_text = Some(stroke_text::<D>);
        (*c_device).base.clip_text = Some(clip_text::<D>);
        (*c_device).base.clip_stroke_text = Some(clip_stroke_text::<D>);
        (*c_device).base.ignore_text = Some(ignore_text::<D>);

        (*c_device).base.fill_shade = Some(fill_shade::<D>);
        (*c_device).base.fill_image = Some(fill_image::<D>);
        (*c_device).base.fill_image_mask = Some(fill_image_mask::<D>);
        (*c_device).base.clip_image_mask = Some(clip_image_mask::<D>);

        (*c_device).base.pop_clip = Some(pop_clip::<D>);

        (*c_device).base.begin_mask = Some(begin_mask::<D>);
        (*c_device).base.end_mask = Some(end_mask::<D>);
        (*c_device).base.begin_group = Some(begin_group::<D>);
        (*c_device).base.end_group = Some(end_group::<D>);

        (*c_device).base.begin_tile = Some(begin_tile::<D>);
        (*c_device).base.end_tile = Some(end_tile::<D>);

        (*c_device).base.render_flags = Some(render_flags::<D>);
        (*c_device).base.set_default_colorspaces = Some(set_default_colorspaces::<D>);

        (*c_device).base.begin_layer = Some(begin_layer::<D>);
        (*c_device).base.end_layer = Some(end_layer::<D>);

        (*c_device).base.begin_structure = Some(begin_structure::<D>);
        (*c_device).base.end_structure = Some(end_structure::<D>);

        (*c_device).base.begin_metatext = Some(begin_metatext::<D>);
        (*c_device).base.end_metatext = Some(end_metatext::<D>);

        Device::from_raw(c_device.cast(), ptr::null_mut())
    }
}

#[repr(C)]
struct CDevice<D> {
    base: fz_device,
    rust_device: D,
}

unsafe fn with_rust_device<D: NativeDevice, T>(
    dev: *mut fz_device,
    f: impl FnOnce(&mut D) -> T,
) -> T {
    let c_device: *mut CDevice<D> = dev.cast();
    let rust_device = &mut (*c_device).rust_device;
    f(rust_device)
}

unsafe extern "C" fn close_device<D: NativeDevice>(_ctx: *mut fz_context, dev: *mut fz_device) {
    with_rust_device::<D, _>(dev, |dev| dev.close_device());
}

unsafe extern "C" fn drop_device<D: NativeDevice>(_ctx: *mut fz_context, dev: *mut fz_device) {
    let c_device: *mut CDevice<D> = dev.cast();
    let rust_device = &raw mut (*c_device).rust_device;

    ptr::drop_in_place(rust_device);
}

unsafe extern "C" fn fill_path<D: NativeDevice>(
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
    with_rust_device::<D, _>(dev, |dev| {
        let cs = ManuallyDrop::new(Colorspace::from_raw(color_space));
        let cs_n = cs.n() as usize;

        let path = ManuallyDrop::new(Path::from_raw(path.cast_mut()));

        dev.fill_path(
            &path,
            even_odd != 0,
            cmt.into(),
            &cs,
            slice::from_raw_parts(color, cs_n),
            alpha,
            color_params.into(),
        );
    });
}

unsafe extern "C" fn stroke_path<D: NativeDevice>(
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
    with_rust_device::<D, _>(dev, |dev| {
        let cs = ManuallyDrop::new(Colorspace::from_raw(color_space));
        let cs_n = cs.n() as usize;

        let path = ManuallyDrop::new(Path::from_raw(path.cast_mut()));
        let stroke_state = ManuallyDrop::new(StrokeState {
            inner: stroke_state.cast_mut(),
        });

        dev.stroke_path(
            &path,
            &stroke_state,
            cmt.into(),
            &cs,
            slice::from_raw_parts(color, cs_n),
            alpha,
            color_params.into(),
        );
    });
}

unsafe extern "C" fn clip_path<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    path: *const fz_path,
    even_odd: c_int,
    cmt: fz_matrix,
    scissor: fz_rect,
) {
    with_rust_device::<D, _>(dev, |dev| {
        let path = ManuallyDrop::new(Path::from_raw(path.cast_mut()));

        dev.clip_path(&path, even_odd != 0, cmt.into(), scissor.into());
    });
}

unsafe extern "C" fn clip_stroke_path<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    path: *const fz_path,
    stroke_state: *const fz_stroke_state,
    cmt: fz_matrix,
    scissor: fz_rect,
) {
    with_rust_device::<D, _>(dev, |dev| {
        let path = ManuallyDrop::new(Path::from_raw(path.cast_mut()));
        let stroke_state = ManuallyDrop::new(StrokeState {
            inner: stroke_state.cast_mut(),
        });

        dev.clip_stroke_path(&path, &stroke_state, cmt.into(), scissor.into());
    });
}

unsafe extern "C" fn fill_text<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    text: *const fz_text,
    cmt: fz_matrix,
    color_space: *mut fz_colorspace,
    color: *const f32,
    alpha: f32,
    color_params: fz_color_params,
) {
    with_rust_device::<D, _>(dev, |dev| {
        let cs = ManuallyDrop::new(Colorspace::from_raw(color_space));
        let cs_n = cs.n() as usize;

        let text = ManuallyDrop::new(Text {
            inner: text.cast_mut(),
        });

        dev.fill_text(
            &text,
            cmt.into(),
            &cs,
            slice::from_raw_parts(color, cs_n),
            alpha,
            color_params.into(),
        );
    });
}

unsafe extern "C" fn stroke_text<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    text: *const fz_text,
    stroke_state: *const fz_stroke_state,
    cmt: fz_matrix,
    color_space: *mut fz_colorspace,
    color: *const f32,
    alpha: f32,
    color_params: fz_color_params,
) {
    with_rust_device::<D, _>(dev, |dev| {
        let cs = ManuallyDrop::new(Colorspace::from_raw(color_space));
        let cs_n = cs.n() as usize;

        let text = ManuallyDrop::new(Text {
            inner: text.cast_mut(),
        });
        let stroke_state = ManuallyDrop::new(StrokeState {
            inner: stroke_state.cast_mut(),
        });

        dev.stroke_text(
            &text,
            &stroke_state,
            cmt.into(),
            &cs,
            slice::from_raw_parts(color, cs_n),
            alpha,
            color_params.into(),
        );
    });
}

unsafe extern "C" fn clip_text<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    text: *const fz_text,
    cmt: fz_matrix,
    scissor: fz_rect,
) {
    with_rust_device::<D, _>(dev, |dev| {
        let text = ManuallyDrop::new(Text {
            inner: text.cast_mut(),
        });

        dev.clip_text(&text, cmt.into(), scissor.into());
    });
}

unsafe extern "C" fn clip_stroke_text<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    text: *const fz_text,
    stroke_state: *const fz_stroke_state,
    cmt: fz_matrix,
    scissor: fz_rect,
) {
    with_rust_device::<D, _>(dev, |dev| {
        let text = ManuallyDrop::new(Text {
            inner: text.cast_mut(),
        });
        let stroke_state = ManuallyDrop::new(StrokeState {
            inner: stroke_state.cast_mut(),
        });

        dev.clip_stroke_text(&text, &stroke_state, cmt.into(), scissor.into());
    });
}

unsafe extern "C" fn ignore_text<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    text: *const fz_text,
    cmt: fz_matrix,
) {
    with_rust_device::<D, _>(dev, |dev| {
        let text = ManuallyDrop::new(Text {
            inner: text.cast_mut(),
        });

        dev.ignore_text(&text, cmt.into());
    });
}

unsafe extern "C" fn fill_shade<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    shd: *mut fz_shade,
    ctm: fz_matrix,
    alpha: f32,
    color_params: fz_color_params,
) {
    with_rust_device::<D, _>(dev, |dev| {
        let shade = ManuallyDrop::new(Shade { inner: shd });

        dev.fill_shade(&shade, ctm.into(), alpha, color_params.into());
    });
}

unsafe extern "C" fn fill_image<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    img: *mut fz_image,
    ctm: fz_matrix,
    alpha: f32,
    color_params: fz_color_params,
) {
    with_rust_device::<D, _>(dev, |dev| {
        let img = ManuallyDrop::new(Image::from_raw(img));

        dev.fill_image(&img, ctm.into(), alpha, color_params.into());
    });
}

unsafe extern "C" fn fill_image_mask<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    img: *mut fz_image,
    ctm: fz_matrix,
    color_space: *mut fz_colorspace,
    color: *const f32,
    alpha: f32,
    color_params: fz_color_params,
) {
    with_rust_device::<D, _>(dev, |dev| {
        let cs = ManuallyDrop::new(Colorspace::from_raw(color_space));
        let cs_n = cs.n() as usize;

        let img = ManuallyDrop::new(Image::from_raw(img));

        dev.fill_image_mask(
            &img,
            ctm.into(),
            &cs,
            slice::from_raw_parts(color, cs_n),
            alpha,
            color_params.into(),
        );
    });
}

unsafe extern "C" fn clip_image_mask<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    img: *mut fz_image,
    cmt: fz_matrix,
    scissor: fz_rect,
) {
    with_rust_device::<D, _>(dev, |dev| {
        let img = ManuallyDrop::new(Image::from_raw(img));

        dev.clip_image_mask(&img, cmt.into(), scissor.into());
    });
}

unsafe extern "C" fn pop_clip<D: NativeDevice>(_ctx: *mut fz_context, dev: *mut fz_device) {
    with_rust_device::<D, _>(dev, |dev| {
        dev.pop_clip();
    });
}

unsafe extern "C" fn begin_mask<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    area: fz_rect,
    luminosity: c_int,
    color_space: *mut fz_colorspace,
    color: *const f32,
    color_params: fz_color_params,
) {
    with_rust_device::<D, _>(dev, |dev| {
        let cs = ManuallyDrop::new(Colorspace::from_raw(color_space));
        let cs_n = cs.n() as usize;

        dev.begin_mask(
            area.into(),
            luminosity != 0,
            &cs,
            slice::from_raw_parts(color, cs_n),
            color_params.into(),
        );
    });
}

unsafe extern "C" fn end_mask<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    f: *mut fz_function,
) {
    with_rust_device::<D, _>(dev, |dev| {
        let f = ManuallyDrop::new(Function { inner: f });

        dev.end_mask(&f);
    });
}

unsafe extern "C" fn begin_group<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    area: fz_rect,
    color_space: *mut fz_colorspace,
    isolated: c_int,
    knockout: c_int,
    blendmode: c_int,
    alpha: f32,
) {
    with_rust_device::<D, _>(dev, |dev| {
        let cs = ManuallyDrop::new(Colorspace::from_raw(color_space));

        let blendmode = BlendMode::try_from(blendmode as u32).unwrap();

        dev.begin_group(
            area.into(),
            &cs,
            isolated != 0,
            knockout != 0,
            blendmode,
            alpha,
        );
    });
}

unsafe extern "C" fn end_group<D: NativeDevice>(_ctx: *mut fz_context, dev: *mut fz_device) {
    with_rust_device::<D, _>(dev, |dev| {
        dev.end_group();
    });
}

unsafe extern "C" fn begin_tile<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    area: fz_rect,
    view: fz_rect,
    xstep: f32,
    ystep: f32,
    ctm: fz_matrix,
    id: c_int,
) -> c_int {
    let i = with_rust_device::<D, _>(dev, |dev| {
        dev.begin_tile(
            area.into(),
            view.into(),
            xstep,
            ystep,
            ctm.into(),
            NonZero::new(id as i32),
        )
    });
    i.map_or(0, NonZero::get) as c_int
}

unsafe extern "C" fn end_tile<D: NativeDevice>(_ctx: *mut fz_context, dev: *mut fz_device) {
    with_rust_device::<D, _>(dev, |dev| {
        dev.end_tile();
    });
}

unsafe extern "C" fn render_flags<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    set: c_int,
    clear: c_int,
) {
    with_rust_device::<D, _>(dev, |dev| {
        dev.render_flags(
            DeviceFlag::from_bits(set as u32).unwrap(),
            DeviceFlag::from_bits(clear as u32).unwrap(),
        );
    });
}

unsafe extern "C" fn set_default_colorspaces<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    dcs: *mut fz_default_colorspaces,
) {
    with_rust_device::<D, _>(dev, |dev| {
        let dcs = ManuallyDrop::new(DefaultColorspaces { inner: dcs });

        dev.set_default_colorspaces(&dcs);
    });
}

unsafe extern "C" fn begin_layer<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    layer_name: *const c_char,
) {
    with_rust_device::<D, _>(dev, |dev| {
        let name = unsafe { CStr::from_ptr(layer_name) }.to_str().unwrap();

        dev.begin_layer(name);
    });
}

unsafe extern "C" fn end_layer<D: NativeDevice>(_ctx: *mut fz_context, dev: *mut fz_device) {
    with_rust_device::<D, _>(dev, |dev| {
        dev.end_layer();
    });
}

unsafe extern "C" fn begin_structure<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    standard: fz_structure,
    raw: *const c_char,
    idx: c_int,
) {
    with_rust_device::<D, _>(dev, |dev| {
        let standard = Structure::try_from(standard as i32).unwrap();
        let raw = unsafe { CStr::from_ptr(raw) }.to_str().unwrap();

        dev.begin_structure(standard, raw, idx as i32);
    });
}

unsafe extern "C" fn end_structure<D: NativeDevice>(_ctx: *mut fz_context, dev: *mut fz_device) {
    with_rust_device::<D, _>(dev, |dev| {
        dev.end_structure();
    });
}

unsafe extern "C" fn begin_metatext<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    meta: fz_metatext,
    text: *const c_char,
) {
    with_rust_device::<D, _>(dev, |dev| {
        let meta = Metatext::try_from(meta as u32).unwrap();
        let text = unsafe { CStr::from_ptr(text) }.to_str().unwrap();

        dev.begin_metatext(meta, text);
    });
}

unsafe extern "C" fn end_metatext<D: NativeDevice>(_ctx: *mut fz_context, dev: *mut fz_device) {
    with_rust_device::<D, _>(dev, |dev| {
        dev.end_metatext();
    });
}

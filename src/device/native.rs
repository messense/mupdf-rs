use std::{
    borrow::Cow,
    cell::RefCell,
    ffi::{c_char, c_int, CStr},
    mem::ManuallyDrop,
    num::NonZero,
    ptr,
    rc::Rc,
    slice,
};

use mupdf_sys::*;

// On windows-msvc, bindgen with recent libclang (observed with LLVM 22) does
// not emit `max_align_t` in the mupdf-sys bindings even though build.rs
// allowlists it. On MSVC `max_align_t` is a double: alignment 8. This local
// type shadows the glob import above when the binding is missing.
#[cfg(target_env = "msvc")]
#[repr(C, align(8))]
#[allow(non_camel_case_types, dead_code)]
struct max_align_t(f64);

use crate::{
    context, guard_ffi_callback, BlendMode, ColorParams, Colorspace, Device, Error, Function,
    Image, Matrix, Path, Rect, Shade, StrokeState, Text,
};

use super::{DefaultColorspaces, DeviceFlag, Metatext, Structure};

#[allow(unused_variables, clippy::too_many_arguments)]
pub trait NativeDevice: 'static {
    fn close_device(&mut self) {}

    /// The colors in `color` must match the colorspace `cs`, as described in
    /// [`Colorspace::convert_color`]
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

    /// The colors in `color` must match the colorspace `cs`, as described in
    /// [`Colorspace::convert_color`]
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

    /// The colors in `color` must match the colorspace `cs`, as described in
    /// [`Colorspace::convert_color`]
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

    /// The colors in `color` must match the colorspace `cs`, as described in
    /// [`Colorspace::convert_color`]
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

    /// The colors in `color` must match the colorspace `cs`, as described in
    /// [`Colorspace::convert_color`]
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

    /// Like [`NativeDevice::fill_shade`], with the device's current scissor rectangle (MuPDF's
    /// `fz_device_current_scissor`: the intersection of the clips pushed so far, infinite
    /// when there is none). MuPDF calls this method; the default forwards to
    /// [`NativeDevice::fill_shade`], so existing implementations are unaffected.
    fn fill_shade_in_scissor(
        &mut self,
        shade: &Shade,
        cmt: Matrix,
        alpha: f32,
        cp: ColorParams,
        scissor: Rect,
    ) {
        self.fill_shade(shade, cmt, alpha, cp);
    }

    /// Like [`NativeDevice::fill_image`], with the device's current scissor rectangle. See
    /// [`NativeDevice::fill_shade_in_scissor`].
    fn fill_image_in_scissor(
        &mut self,
        img: &Image,
        cmt: Matrix,
        alpha: f32,
        cp: ColorParams,
        scissor: Rect,
    ) {
        self.fill_image(img, cmt, alpha, cp);
    }

    /// Like [`NativeDevice::fill_image_mask`], with the device's current scissor rectangle. See
    /// [`NativeDevice::fill_shade_in_scissor`].
    fn fill_image_mask_in_scissor(
        &mut self,
        img: &Image,
        cmt: Matrix,
        color_space: &Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
        scissor: Rect,
    ) {
        self.fill_image_mask(img, cmt, color_space, color, alpha, cp);
    }

    fn clip_image_mask(&mut self, img: &Image, cmt: Matrix, scissor: Rect) {}

    fn pop_clip(&mut self) {}

    /// The colors in `color` must match the colorspace `cs`, as described in
    /// [`Colorspace::convert_color`]
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
        doc_id: Option<NonZero<i32>>,
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

impl<T: NativeDevice + ?Sized> NativeDevice for Box<T> {
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

    fn fill_shade_in_scissor(
        &mut self,
        shade: &Shade,
        cmt: Matrix,
        alpha: f32,
        cp: ColorParams,
        scissor: Rect,
    ) {
        (**self).fill_shade_in_scissor(shade, cmt, alpha, cp, scissor);
    }

    fn fill_image_in_scissor(
        &mut self,
        img: &Image,
        cmt: Matrix,
        alpha: f32,
        cp: ColorParams,
        scissor: Rect,
    ) {
        (**self).fill_image_in_scissor(img, cmt, alpha, cp, scissor);
    }

    fn fill_image_mask_in_scissor(
        &mut self,
        img: &Image,
        cmt: Matrix,
        color_space: &Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
        scissor: Rect,
    ) {
        (**self).fill_image_mask_in_scissor(img, cmt, color_space, color, alpha, cp, scissor);
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
        doc_id: Option<NonZero<i32>>,
    ) -> Option<NonZero<i32>> {
        (**self).begin_tile(area, view, x_step, y_step, ctm, id, doc_id)
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

impl<T: NativeDevice + ?Sized> NativeDevice for Rc<RefCell<T>> {
    fn close_device(&mut self) {
        self.borrow_mut().close_device();
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
        self.borrow_mut()
            .fill_path(path, even_odd, cmt, color_space, color, alpha, cp);
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
        self.borrow_mut()
            .stroke_path(path, stroke_state, cmt, color_space, color, alpha, cp);
    }

    fn clip_path(&mut self, path: &Path, even_odd: bool, cmt: Matrix, scissor: Rect) {
        self.borrow_mut().clip_path(path, even_odd, cmt, scissor);
    }

    fn clip_stroke_path(
        &mut self,
        path: &Path,
        stroke_state: &StrokeState,
        cmt: Matrix,
        scissor: Rect,
    ) {
        self.borrow_mut()
            .clip_stroke_path(path, stroke_state, cmt, scissor);
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
        self.borrow_mut()
            .fill_text(text, cmt, color_space, color, alpha, cp);
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
        self.borrow_mut()
            .stroke_text(text, stroke_state, cmt, color_space, color, alpha, cp);
    }

    fn clip_text(&mut self, text: &Text, cmt: Matrix, scissor: Rect) {
        self.borrow_mut().clip_text(text, cmt, scissor);
    }

    fn clip_stroke_text(
        &mut self,
        text: &Text,
        stroke_state: &StrokeState,
        cmt: Matrix,
        scissor: Rect,
    ) {
        self.borrow_mut()
            .clip_stroke_text(text, stroke_state, cmt, scissor);
    }

    fn ignore_text(&mut self, text: &Text, cmt: Matrix) {
        self.borrow_mut().ignore_text(text, cmt);
    }

    fn fill_shade(&mut self, shade: &Shade, cmt: Matrix, alpha: f32, cp: ColorParams) {
        self.borrow_mut().fill_shade(shade, cmt, alpha, cp);
    }

    fn fill_image(&mut self, img: &Image, cmt: Matrix, alpha: f32, cp: ColorParams) {
        self.borrow_mut().fill_image(img, cmt, alpha, cp);
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
        self.borrow_mut()
            .fill_image_mask(img, cmt, color_space, color, alpha, cp);
    }

    fn fill_shade_in_scissor(
        &mut self,
        shade: &Shade,
        cmt: Matrix,
        alpha: f32,
        cp: ColorParams,
        scissor: Rect,
    ) {
        self.borrow_mut()
            .fill_shade_in_scissor(shade, cmt, alpha, cp, scissor);
    }

    fn fill_image_in_scissor(
        &mut self,
        img: &Image,
        cmt: Matrix,
        alpha: f32,
        cp: ColorParams,
        scissor: Rect,
    ) {
        self.borrow_mut()
            .fill_image_in_scissor(img, cmt, alpha, cp, scissor);
    }

    fn fill_image_mask_in_scissor(
        &mut self,
        img: &Image,
        cmt: Matrix,
        color_space: &Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
        scissor: Rect,
    ) {
        self.borrow_mut().fill_image_mask_in_scissor(
            img,
            cmt,
            color_space,
            color,
            alpha,
            cp,
            scissor,
        );
    }

    fn clip_image_mask(&mut self, img: &Image, cmt: Matrix, scissor: Rect) {
        self.borrow_mut().clip_image_mask(img, cmt, scissor);
    }

    fn pop_clip(&mut self) {
        self.borrow_mut().pop_clip();
    }

    fn begin_mask(
        &mut self,
        area: Rect,
        luminosity: bool,
        color_space: &Colorspace,
        color: &[f32],
        cp: ColorParams,
    ) {
        self.borrow_mut()
            .begin_mask(area, luminosity, color_space, color, cp);
    }

    fn end_mask(&mut self, f: &Function) {
        self.borrow_mut().end_mask(f);
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
        self.borrow_mut()
            .begin_group(area, cs, isolated, knockout, blendmode, alpha);
    }

    fn end_group(&mut self) {
        self.borrow_mut().end_group();
    }

    fn begin_tile(
        &mut self,
        area: Rect,
        view: Rect,
        x_step: f32,
        y_step: f32,
        ctm: Matrix,
        id: Option<NonZero<i32>>,
        doc_id: Option<NonZero<i32>>,
    ) -> Option<NonZero<i32>> {
        self.borrow_mut()
            .begin_tile(area, view, x_step, y_step, ctm, id, doc_id)
    }

    fn end_tile(&mut self) {
        self.borrow_mut().end_tile();
    }

    fn render_flags(&mut self, set: DeviceFlag, clear: DeviceFlag) {
        self.borrow_mut().render_flags(set, clear);
    }

    fn set_default_colorspaces(&mut self, default_cs: &DefaultColorspaces) {
        self.borrow_mut().set_default_colorspaces(default_cs);
    }

    fn begin_layer(&mut self, name: &str) {
        self.borrow_mut().begin_layer(name);
    }

    fn end_layer(&mut self) {
        self.borrow_mut().end_layer();
    }

    fn begin_structure(&mut self, standard: Structure, raw: &str, idx: i32) {
        self.borrow_mut().begin_structure(standard, raw, idx);
    }

    fn end_structure(&mut self) {
        self.borrow_mut().end_structure();
    }

    fn begin_metatext(&mut self, meta: Metatext, text: &str) {
        self.borrow_mut().begin_metatext(meta, text);
    }

    fn end_metatext(&mut self) {
        self.borrow_mut().end_metatext();
    }
}

/// ```compile_fail
/// use mupdf::{Device, NativeDevice};
///
/// #[repr(align(32))]
/// struct U256(#[allow(dead_code)] [u128; 2]);
///
/// impl NativeDevice for U256 {}
///
/// Device::from_native(U256([0, 0])).unwrap();
/// ```
pub(crate) fn create<D: NativeDevice>(device: D) -> Result<Device, Error> {
    const {
        assert!(align_of::<D>() <= align_of::<max_align_t>());
    }

    let ret = unsafe {
        let c_device: *mut CDevice<D> =
            ffi_try!(mupdf_new_derived_device(context(), c"RustDevice"))?;
        ptr::write(
            &raw mut (*c_device).rust_device,
            ManuallyDrop::new(Some(device)),
        );

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
    };
    Ok(ret)
}

#[repr(C)]
struct CDevice<D> {
    base: fz_device,
    rust_device: ManuallyDrop<Option<D>>,
}

// MuPDF invokes these callbacks with pointers borrowed for the duration of each callback. The
// temporary Rust wrappers created here must not take ownership of those pointers, so callbacks use
// `ManuallyDrop` to prevent Drop impls from releasing borrowed MuPDF objects. Color slices are
// formed from `color` with the component count reported by the corresponding MuPDF colorspace.
unsafe fn with_rust_device<D: NativeDevice, T>(
    dev: *mut fz_device,
    f: impl FnOnce(&mut D) -> T,
) -> T {
    // SAFETY: `dev` is the `base` field of a `CDevice<D>` allocated by `create`. MuPDF calls these
    // callbacks only while that device is alive, and `drop_device` drops the embedded Rust value
    // exactly once.
    guard_ffi_callback(|| unsafe {
        let c_device: *mut CDevice<D> = dev.cast();
        let rust_device = (*c_device)
            .rust_device
            .as_mut()
            .expect("native device callback invoked after drop");
        f(rust_device)
    })
}

unsafe extern "C" fn close_device<D: NativeDevice>(_ctx: *mut fz_context, dev: *mut fz_device) {
    unsafe {
        with_rust_device::<D, _>(dev, |dev| dev.close_device());
    }
}

unsafe extern "C" fn drop_device<D: NativeDevice>(_ctx: *mut fz_context, dev: *mut fz_device) {
    guard_ffi_callback(|| unsafe {
        let c_device: *mut CDevice<D> = dev.cast();
        let rust_device = &raw mut (*c_device).rust_device;
        if let Some(device) = (*rust_device).take() {
            drop(device);
        }
    });
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
    unsafe {
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
    unsafe {
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
}

unsafe extern "C" fn clip_path<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    path: *const fz_path,
    even_odd: c_int,
    cmt: fz_matrix,
    scissor: fz_rect,
) {
    unsafe {
        with_rust_device::<D, _>(dev, |dev| {
            let path = ManuallyDrop::new(Path::from_raw(path.cast_mut()));

            dev.clip_path(&path, even_odd != 0, cmt.into(), scissor.into());
        });
    }
}

unsafe extern "C" fn clip_stroke_path<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    path: *const fz_path,
    stroke_state: *const fz_stroke_state,
    cmt: fz_matrix,
    scissor: fz_rect,
) {
    unsafe {
        with_rust_device::<D, _>(dev, |dev| {
            let path = ManuallyDrop::new(Path::from_raw(path.cast_mut()));
            let stroke_state = ManuallyDrop::new(StrokeState {
                inner: stroke_state.cast_mut(),
            });

            dev.clip_stroke_path(&path, &stroke_state, cmt.into(), scissor.into());
        });
    }
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
    unsafe {
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
    unsafe {
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
}

unsafe extern "C" fn clip_text<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    text: *const fz_text,
    cmt: fz_matrix,
    scissor: fz_rect,
) {
    unsafe {
        with_rust_device::<D, _>(dev, |dev| {
            let text = ManuallyDrop::new(Text {
                inner: text.cast_mut(),
            });

            dev.clip_text(&text, cmt.into(), scissor.into());
        });
    }
}

unsafe extern "C" fn clip_stroke_text<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    text: *const fz_text,
    stroke_state: *const fz_stroke_state,
    cmt: fz_matrix,
    scissor: fz_rect,
) {
    unsafe {
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
}

unsafe extern "C" fn ignore_text<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    text: *const fz_text,
    cmt: fz_matrix,
) {
    unsafe {
        with_rust_device::<D, _>(dev, |dev| {
            let text = ManuallyDrop::new(Text {
                inner: text.cast_mut(),
            });

            dev.ignore_text(&text, cmt.into());
        });
    }
}

unsafe extern "C" fn fill_shade<D: NativeDevice>(
    ctx: *mut fz_context,
    dev: *mut fz_device,
    shd: *mut fz_shade,
    ctm: fz_matrix,
    alpha: f32,
    color_params: fz_color_params,
) {
    unsafe {
        let scissor = fz_device_current_scissor(ctx, dev);
        with_rust_device::<D, _>(dev, |dev| {
            let shade = ManuallyDrop::new(Shade { inner: shd });

            dev.fill_shade_in_scissor(
                &shade,
                ctm.into(),
                alpha,
                color_params.into(),
                scissor.into(),
            );
        });
    }
}

unsafe extern "C" fn fill_image<D: NativeDevice>(
    ctx: *mut fz_context,
    dev: *mut fz_device,
    img: *mut fz_image,
    ctm: fz_matrix,
    alpha: f32,
    color_params: fz_color_params,
) {
    unsafe {
        let scissor = fz_device_current_scissor(ctx, dev);
        with_rust_device::<D, _>(dev, |dev| {
            let img = ManuallyDrop::new(Image::from_raw(img));

            dev.fill_image_in_scissor(&img, ctm.into(), alpha, color_params.into(), scissor.into());
        });
    }
}

unsafe extern "C" fn fill_image_mask<D: NativeDevice>(
    ctx: *mut fz_context,
    dev: *mut fz_device,
    img: *mut fz_image,
    ctm: fz_matrix,
    color_space: *mut fz_colorspace,
    color: *const f32,
    alpha: f32,
    color_params: fz_color_params,
) {
    unsafe {
        let scissor = fz_device_current_scissor(ctx, dev);
        with_rust_device::<D, _>(dev, |dev| {
            let cs = ManuallyDrop::new(Colorspace::from_raw(color_space));
            let cs_n = cs.n() as usize;

            let img = ManuallyDrop::new(Image::from_raw(img));

            dev.fill_image_mask_in_scissor(
                &img,
                ctm.into(),
                &cs,
                slice::from_raw_parts(color, cs_n),
                alpha,
                color_params.into(),
                scissor.into(),
            );
        });
    }
}

unsafe extern "C" fn clip_image_mask<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    img: *mut fz_image,
    cmt: fz_matrix,
    scissor: fz_rect,
) {
    unsafe {
        with_rust_device::<D, _>(dev, |dev| {
            let img = ManuallyDrop::new(Image::from_raw(img));

            dev.clip_image_mask(&img, cmt.into(), scissor.into());
        });
    }
}

unsafe extern "C" fn pop_clip<D: NativeDevice>(_ctx: *mut fz_context, dev: *mut fz_device) {
    unsafe {
        with_rust_device::<D, _>(dev, |dev| {
            dev.pop_clip();
        });
    }
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
    unsafe {
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
}

unsafe extern "C" fn end_mask<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    f: *mut fz_function,
) {
    unsafe {
        with_rust_device::<D, _>(dev, |dev| {
            let f = ManuallyDrop::new(Function { inner: f });

            dev.end_mask(&f);
        });
    }
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
    unsafe {
        with_rust_device::<D, _>(dev, |dev| {
            let cs = ManuallyDrop::new(Colorspace::from_raw(color_space));

            // Fall back to Normal for blend modes this binding does not know: MuPDF pairs every
            // begin_group with an end_group, so skipping the call would unbalance user devices.
            let blendmode = blendmode.try_into().unwrap_or(BlendMode::Normal);
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
}

unsafe extern "C" fn end_group<D: NativeDevice>(_ctx: *mut fz_context, dev: *mut fz_device) {
    unsafe {
        with_rust_device::<D, _>(dev, |dev| {
            dev.end_group();
        });
    }
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
    doc_id: c_int,
) -> c_int {
    unsafe {
        let i = with_rust_device::<D, _>(dev, |dev| {
            dev.begin_tile(
                area.into(),
                view.into(),
                xstep,
                ystep,
                ctm.into(),
                NonZero::new(id as i32),
                NonZero::new(doc_id as i32),
            )
        });
        i.map_or(0, NonZero::get) as c_int
    }
}

unsafe extern "C" fn end_tile<D: NativeDevice>(_ctx: *mut fz_context, dev: *mut fz_device) {
    unsafe {
        with_rust_device::<D, _>(dev, |dev| {
            dev.end_tile();
        });
    }
}

unsafe extern "C" fn render_flags<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    set: c_int,
    clear: c_int,
) {
    unsafe {
        with_rust_device::<D, _>(dev, |dev| {
            dev.render_flags(
                DeviceFlag::from_bits_truncate(set as u32),
                DeviceFlag::from_bits_truncate(clear as u32),
            );
        });
    }
}

unsafe extern "C" fn set_default_colorspaces<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    dcs: *mut fz_default_colorspaces,
) {
    unsafe {
        with_rust_device::<D, _>(dev, |dev| {
            let dcs = ManuallyDrop::new(DefaultColorspaces { inner: dcs });

            dev.set_default_colorspaces(&dcs);
        });
    }
}

unsafe extern "C" fn begin_layer<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    layer_name: *const c_char,
) {
    unsafe {
        with_rust_device::<D, _>(dev, |dev| {
            // Use an empty name for null pointers: MuPDF pairs every begin_layer with an
            // end_layer, so skipping the call would unbalance user devices.
            let name = if layer_name.is_null() {
                Cow::Borrowed("")
            } else {
                CStr::from_ptr(layer_name).to_string_lossy()
            };
            dev.begin_layer(&name);
        });
    }
}

unsafe extern "C" fn end_layer<D: NativeDevice>(_ctx: *mut fz_context, dev: *mut fz_device) {
    unsafe {
        with_rust_device::<D, _>(dev, |dev| {
            dev.end_layer();
        });
    }
}

unsafe extern "C" fn begin_structure<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    standard: fz_structure,
    raw: *const c_char,
    idx: c_int,
) {
    unsafe {
        with_rust_device::<D, _>(dev, |dev| {
            // Fall back to Structure::Invalid / an empty string for values this binding does not
            // know: MuPDF pairs every begin_structure with an end_structure, so skipping the call
            // would unbalance user devices.
            let standard = Structure::try_from(standard as i32).unwrap_or(Structure::Invalid);
            let raw = if raw.is_null() {
                Cow::Borrowed("")
            } else {
                CStr::from_ptr(raw).to_string_lossy()
            };
            dev.begin_structure(standard, &raw, idx as i32);
        });
    }
}

unsafe extern "C" fn end_structure<D: NativeDevice>(_ctx: *mut fz_context, dev: *mut fz_device) {
    unsafe {
        with_rust_device::<D, _>(dev, |dev| {
            dev.end_structure();
        });
    }
}

unsafe extern "C" fn begin_metatext<D: NativeDevice>(
    _ctx: *mut fz_context,
    dev: *mut fz_device,
    meta: fz_metatext,
    text: *const c_char,
) {
    unsafe {
        with_rust_device::<D, _>(dev, |dev| {
            // Fall back to ActualText / an empty string for values this binding does not know:
            // MuPDF pairs every begin_metatext with an end_metatext, so skipping the call would
            // unbalance user devices.
            let meta = Metatext::try_from(meta).unwrap_or(Metatext::ActualText);
            let text = if text.is_null() {
                Cow::Borrowed("")
            } else {
                CStr::from_ptr(text).to_string_lossy()
            };
            dev.begin_metatext(meta, &text);
        });
    }
}

unsafe extern "C" fn end_metatext<D: NativeDevice>(_ctx: *mut fz_context, dev: *mut fz_device) {
    unsafe {
        with_rust_device::<D, _>(dev, |dev| {
            dev.end_metatext();
        });
    }
}

#[cfg(test)]
mod test {
    use std::{cell::RefCell, rc::Rc};

    use crate::{
        device::{Metatext, Structure},
        ColorParams, Colorspace, Device, Document, Image, Matrix, NativeDevice, Rect, Shade,
    };

    #[test]
    fn native_device() {
        #[derive(Default)]
        struct Test {
            begin_layer: u8,
            end_layer: u8,
            begin_structure: u8,
            end_structure: u8,
            begin_metatext: u8,
            end_metatext: u8,
        }

        impl NativeDevice for Test {
            fn begin_layer(&mut self, name: &str) {
                assert_eq!(name, "begin layer name");
                self.begin_layer += 1;
            }

            fn end_layer(&mut self) {
                self.end_layer += 1;
            }

            fn begin_structure(&mut self, standard: Structure, raw: &str, idx: i32) {
                assert_eq!(standard, Structure::Div);
                assert_eq!(raw, "div");
                assert_eq!(idx, 5);
                self.begin_structure += 1;
            }

            fn end_structure(&mut self) {
                self.end_structure += 1;
            }

            fn begin_metatext(&mut self, meta: Metatext, text: &str) {
                assert_eq!(meta, Metatext::Title);
                assert_eq!(text, "some text");
                self.begin_metatext += 1;
            }

            fn end_metatext(&mut self) {
                self.end_metatext += 1;
            }
        }

        let ndev = Rc::new(RefCell::new(Test::default()));
        let dev = Device::from_native(ndev.clone()).unwrap();

        dev.begin_layer("begin layer name").unwrap();
        assert_eq!(ndev.borrow().begin_layer, 1);
        dev.end_layer().unwrap();
        assert_eq!(ndev.borrow().end_layer, 1);

        dev.begin_structure(Structure::Div, "div", 5).unwrap();
        assert_eq!(ndev.borrow().begin_structure, 1);
        dev.end_structure().unwrap();
        assert_eq!(ndev.borrow().end_structure, 1);

        dev.begin_metatext(Metatext::Title, "some text").unwrap();
        assert_eq!(ndev.borrow().begin_metatext, 1);
        dev.end_metatext().unwrap();
        assert_eq!(ndev.borrow().end_metatext, 1);
    }

    #[test]
    fn native_device_drop() {
        struct DropDevice(#[allow(dead_code)] Rc<()>);

        impl NativeDevice for DropDevice {}

        let proof = Rc::new(());
        let detector = proof.clone();
        assert_eq!(Rc::strong_count(&proof), 2, "setup failed");

        let dev = Device::from_native(DropDevice(detector)).unwrap();
        assert_eq!(Rc::strong_count(&proof), 2, "dropped too early");
        drop(dev);
        assert_eq!(Rc::strong_count(&proof), 1, "not dropped");
    }

    /// What the recording device saw for each fill callback.
    #[derive(Default)]
    struct Recorder {
        shades: Vec<(Rect, Rect, Matrix)>,
        images: Vec<(Rect, Matrix)>,
        image_masks: Vec<(Rect, Matrix)>,
        plain_shade_calls: usize,
    }

    impl NativeDevice for Recorder {
        fn fill_shade(&mut self, _shade: &Shade, _cmt: Matrix, _alpha: f32, _cp: ColorParams) {
            self.plain_shade_calls += 1;
        }

        fn fill_shade_in_scissor(
            &mut self,
            shade: &Shade,
            cmt: Matrix,
            alpha: f32,
            cp: ColorParams,
            scissor: Rect,
        ) {
            // The default forwards to the unscissored callback; keep that contract visible.
            self.fill_shade(shade, cmt.clone(), alpha, cp);
            let bounds = shade.bounds(&cmt).unwrap();
            self.shades.push((bounds, scissor, cmt));
        }

        fn fill_image_in_scissor(
            &mut self,
            _img: &Image,
            cmt: Matrix,
            _alpha: f32,
            _cp: ColorParams,
            scissor: Rect,
        ) {
            self.images.push((scissor, cmt));
        }

        fn fill_image_mask_in_scissor(
            &mut self,
            _img: &Image,
            cmt: Matrix,
            _color_space: &Colorspace,
            _color: &[f32],
            _alpha: f32,
            _cp: ColorParams,
            scissor: Rect,
        ) {
            self.image_masks.push((scissor, cmt));
        }
    }

    fn pdf_with_content(content: &str) -> Vec<u8> {
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R \
             /Resources << /Shading << /Sh0 5 0 R >> >> >>"
                .to_owned(),
            format!(
                "<< /Length {} >>\nstream\n{content}\nendstream",
                content.len()
            ),
            "<< /ShadingType 2 /ColorSpace /DeviceGray /Coords [0 0 100 0] \
             /BBox [10 20 110 120] /Function << /FunctionType 2 /Domain [0 1] \
             /C0 [0] /C1 [1] /N 1 >> >>"
                .to_owned(),
        ];
        let mut pdf = b"%PDF-1.4\n".to_vec();
        for (i, body) in objects.iter().enumerate() {
            pdf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
        }
        pdf.extend_from_slice(b"trailer\n<< /Root 1 0 R /Size 6 >>\n%%EOF\n");
        pdf
    }

    fn record(content: &str) -> Rc<RefCell<Recorder>> {
        let doc =
            Document::from_copied_bytes(&pdf_with_content(content), "application/pdf").unwrap();
        let page = doc.load_page(0).unwrap();
        let rec = Rc::new(RefCell::new(Recorder::default()));
        let dev = Device::from_native(rec.clone()).unwrap();
        page.run(&dev, &Matrix::IDENTITY).unwrap();
        drop(dev);
        rec
    }

    #[test]
    fn shade_bounds_and_scissor_reach_fill_shade() {
        let rec = record("q 0 0 50 50 re W n /Sh0 sh Q /Sh0 sh");
        let rec = rec.borrow();
        assert_eq!(rec.shades.len(), 2);
        // The same fz_bound_shade rectangle with and without a clip: the transformed /BBox (the
        // page run flips y: device y is 200 minus PDF y).
        for (bounds, _, _) in &rec.shades {
            assert_eq!(*bounds, Rect::new(10.0, 80.0, 110.0, 180.0));
        }
        // Inside the clip the scissor is the clip; outside it is not restricted to it.
        assert_eq!(rec.shades[0].1, Rect::new(0.0, 150.0, 50.0, 200.0));
        let outside = rec.shades[1].1;
        assert!(
            outside.x0 <= 0.0 && outside.y0 <= 0.0 && outside.x1 >= 200.0 && outside.y1 >= 200.0
        );
        // The default `fill_shade_in_scissor` contract: the unscissored callback still runs
        // when an implementation forwards to it.
        assert_eq!(rec.plain_shade_calls, 2);
    }

    #[test]
    fn shade_bounds_follow_the_matrix() {
        let rec = record("q 2 0 0 2 5 5 cm /Sh0 sh Q");
        let rec = rec.borrow();
        assert_eq!(rec.shades.len(), 1);
        assert_eq!(rec.shades[0].0, Rect::new(25.0, -45.0, 225.0, 155.0));
    }

    #[test]
    fn scissor_reaches_fill_image() {
        let rec = record(
            "q 0 0 30 30 re W n q 40 0 0 40 10 10 cm \
             BI /W 1 /H 1 /CS /G /BPC 8 ID A EI Q Q",
        );
        let rec = rec.borrow();
        assert_eq!(rec.images.len(), 1);
        assert_eq!(rec.images[0].0, Rect::new(0.0, 170.0, 30.0, 200.0));
        assert_eq!(
            rec.images[0].1,
            Matrix::new(40.0, 0.0, 0.0, 40.0, 10.0, 150.0)
        );
    }

    #[test]
    fn scissor_reaches_fill_image_mask() {
        let rec = record(
            "q 0 0 30 30 re W n q 40 0 0 40 10 10 cm \
             BI /W 1 /H 1 /IM true /BPC 1 ID A EI Q Q",
        );
        let rec = rec.borrow();
        assert_eq!(rec.image_masks.len(), 1);
        assert_eq!(rec.image_masks[0].0, Rect::new(0.0, 170.0, 30.0, 200.0));
    }
}

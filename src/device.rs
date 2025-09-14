use std::{
    ffi::{c_int, CString},
    num::NonZero,
    ptr,
};

use bitflags::bitflags;

use mupdf_sys::*;

use crate::{
    context, from_enum, ColorParams, Colorspace, DisplayList, Error, FFIWrapper, IRect, Image,
    Matrix, Path, Pixmap, Rect, Shade, StrokeState, Text, TextPage, TextPageFlags,
};

mod native;
pub use native::NativeDevice;

from_enum! { c_int,
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum BlendMode {
        /* PDF 1.4 -- standard separable */
        Normal = FZ_BLEND_NORMAL,
        Multiply = FZ_BLEND_MULTIPLY,
        Screen = FZ_BLEND_SCREEN,
        Overlay = FZ_BLEND_OVERLAY,
        Darken = FZ_BLEND_DARKEN,
        Lighten = FZ_BLEND_LIGHTEN,
        ColorDodge = FZ_BLEND_COLOR_DODGE,
        ColorBurn = FZ_BLEND_COLOR_BURN,
        HardLight = FZ_BLEND_HARD_LIGHT,
        SoftLight = FZ_BLEND_SOFT_LIGHT,
        Difference = FZ_BLEND_DIFFERENCE,
        Exclusion = FZ_BLEND_EXCLUSION,

        /* PDF 1.4 -- standard non-separable */
        Hue = FZ_BLEND_HUE,
        Saturation = FZ_BLEND_SATURATION,
        Color = FZ_BLEND_COLOR,
        Luminosity = FZ_BLEND_LUMINOSITY,
    }
}

bitflags! {
    pub struct DeviceFlag: u32 {
        const MASK = FZ_DEVFLAG_MASK as _;
        const COLOR = FZ_DEVFLAG_COLOR as _;
        const UNCACHEABLE = FZ_DEVFLAG_UNCACHEABLE as _;
        const FILLCOLOR_UNDEFINED = FZ_DEVFLAG_FILLCOLOR_UNDEFINED as _;
        const STROKECOLOR_UNDEFINED = FZ_DEVFLAG_STROKECOLOR_UNDEFINED as _;
        const STARTCAP_UNDEFINED = FZ_DEVFLAG_STARTCAP_UNDEFINED as _;
        const DASHCAP_UNDEFINED = FZ_DEVFLAG_DASHCAP_UNDEFINED as _;
        const ENDCAP_UNDEFINED = FZ_DEVFLAG_ENDCAP_UNDEFINED as _;
        const LINEJOIN_UNDEFINED = FZ_DEVFLAG_LINEJOIN_UNDEFINED as _;
        const MITERLIMIT_UNDEFINED = FZ_DEVFLAG_MITERLIMIT_UNDEFINED as _;
        const LINEWIDTH_UNDEFINED = FZ_DEVFLAG_LINEWIDTH_UNDEFINED as _;
        const BBOX_DEFINED = FZ_DEVFLAG_BBOX_DEFINED as _;
        const GRIDFIT_AS_TILED = FZ_DEVFLAG_GRIDFIT_AS_TILED as _;
        const DASH_PATTERN_UNDEFINED = FZ_DEVFLAG_DASH_PATTERN_UNDEFINED as _;
    }
}

from_enum! { fz_structure,
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub enum Structure {
        Invalid = FZ_STRUCTURE_INVALID,

        /* Grouping elements (PDF 1.7 - Table 10.20) */
        Document = FZ_STRUCTURE_DOCUMENT,
        Part = FZ_STRUCTURE_PART,
        Art = FZ_STRUCTURE_ART,
        Sect = FZ_STRUCTURE_SECT,
        Div = FZ_STRUCTURE_DIV,
        BlockQuote = FZ_STRUCTURE_BLOCKQUOTE,
        Caption = FZ_STRUCTURE_CAPTION,
        TOC = FZ_STRUCTURE_TOC,
        TOCI = FZ_STRUCTURE_TOCI,
        Index = FZ_STRUCTURE_INDEX,
        NonStruct = FZ_STRUCTURE_NONSTRUCT,
        Private = FZ_STRUCTURE_PRIVATE,
        /* Grouping elements (PDF 2.0 - Table 364) */
        DocumentFragment = FZ_STRUCTURE_DOCUMENTFRAGMENT,
        /* Grouping elements (PDF 2.0 - Table 365) */
        Aside = FZ_STRUCTURE_ASIDE,
        /* Grouping elements (PDF 2.0 - Table 366) */
        Title = FZ_STRUCTURE_TITLE,
        FENote = FZ_STRUCTURE_FENOTE,
        /* Grouping elements (PDF 2.0 - Table 367) */
        Sub = FZ_STRUCTURE_SUB,

        /* Paragraphlike elements (PDF 1.7 - Table 10.21) */
        P = FZ_STRUCTURE_P,
        H = FZ_STRUCTURE_H,
        H1 = FZ_STRUCTURE_H1,
        H2 = FZ_STRUCTURE_H2,
        H3 = FZ_STRUCTURE_H3,
        H4 = FZ_STRUCTURE_H4,
        H5 = FZ_STRUCTURE_H5,
        H6 = FZ_STRUCTURE_H6,

        /* List elements (PDF 1.7 - Table 10.23) */
        List = FZ_STRUCTURE_LIST,
        ListItem = FZ_STRUCTURE_LISTITEM,
        Label = FZ_STRUCTURE_LABEL,
        ListBody = FZ_STRUCTURE_LISTBODY,

        /* Table elements (PDF 1.7 - Table 10.24) */
        Table = FZ_STRUCTURE_TABLE,
        TR = FZ_STRUCTURE_TR,
        TH = FZ_STRUCTURE_TH,
        TD = FZ_STRUCTURE_TD,
        THead = FZ_STRUCTURE_THEAD,
        TBody = FZ_STRUCTURE_TBODY,
        TFoot = FZ_STRUCTURE_TFOOT,

        /* Inline elements (PDF 1.7 - Table 10.25) */
        Span = FZ_STRUCTURE_SPAN,
        Quote = FZ_STRUCTURE_QUOTE,
        Note = FZ_STRUCTURE_NOTE,
        Reference = FZ_STRUCTURE_REFERENCE,
        BibEntry = FZ_STRUCTURE_BIBENTRY,
        Code = FZ_STRUCTURE_CODE,
        Link = FZ_STRUCTURE_LINK,
        Annot = FZ_STRUCTURE_ANNOT,
        /* Inline elements (PDF 2.0 - Table 368) */
        Em = FZ_STRUCTURE_EM,
        Strong = FZ_STRUCTURE_STRONG,

        /* Ruby inline element (PDF 1.7 - Table 10.26) */
        Ruby = FZ_STRUCTURE_RUBY,
        RB = FZ_STRUCTURE_RB,
        RT = FZ_STRUCTURE_RT,
        RP = FZ_STRUCTURE_RP,

        /* Warichu inline element (PDF 1.7 - Table 10.26) */
        Warichu = FZ_STRUCTURE_WARICHU,
        WT = FZ_STRUCTURE_WT,
        WP = FZ_STRUCTURE_WP,

        /* Illustration elements (PDF 1.7 - Table 10.27) */
        Figure = FZ_STRUCTURE_FIGURE,
        Formula = FZ_STRUCTURE_FORMULA,
        Form = FZ_STRUCTURE_FORM,

        /* Artifact structure type (PDF 2.0 - Table 375) */
        Artifact = FZ_STRUCTURE_ARTIFACT,
    }
}

from_enum! { fz_metatext,
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub enum Metatext {
        ActualText = FZ_METATEXT_ACTUALTEXT,
        Alt = FZ_METATEXT_ALT,
        Abbreviation = FZ_METATEXT_ABBREVIATION,
        Title = FZ_METATEXT_TITLE,
    }
}

pub struct DefaultColorspaces {
    pub(crate) inner: *mut fz_default_colorspaces,
}

impl Drop for DefaultColorspaces {
    fn drop(&mut self) {
        unsafe { fz_drop_default_colorspaces(context(), self.inner) }
    }
}

pub struct Function {
    pub(crate) inner: *mut fz_function,
}

impl Drop for Function {
    fn drop(&mut self) {
        unsafe { fz_drop_function(context(), self.inner) }
    }
}

#[derive(Debug)]
pub struct Device {
    pub(crate) dev: *mut fz_device,
    pub(crate) list: *mut fz_display_list,
}

impl Device {
    pub(crate) unsafe fn from_raw(dev: *mut fz_device, list: *mut fz_display_list) -> Self {
        Self { dev, list }
    }

    pub fn from_native<D: NativeDevice>(device: D) -> Result<Self, Error> {
        native::create(device)
    }

    pub fn from_pixmap_with_clip(pixmap: &Pixmap, clip: IRect) -> Result<Self, Error> {
        unsafe { ffi_try!(mupdf_new_draw_device(context(), pixmap.inner, clip.into())) }.map(
            |dev| Self {
                dev,
                list: ptr::null_mut(),
            },
        )
    }

    pub fn from_pixmap(pixmap: &Pixmap) -> Result<Self, Error> {
        Self::from_pixmap_with_clip(pixmap, IRect::INF)
    }

    pub fn from_display_list(list: &DisplayList) -> Result<Self, Error> {
        unsafe { ffi_try!(mupdf_new_display_list_device(context(), list.inner)) }.map(|dev| Self {
            dev,
            list: list.inner,
        })
    }

    pub fn from_text_page(page: &TextPage, opts: TextPageFlags) -> Result<Self, Error> {
        unsafe {
            ffi_try!(mupdf_new_stext_device(
                context(),
                page.as_ptr().cast_mut(),
                opts.bits() as _
            ))
        }
        .map(|dev| Self {
            dev,
            list: ptr::null_mut(),
        })
    }

    /// The colors in `color` must match the colorspace `cs`, as described in
    /// [`Colorspace::convert_color`]
    #[allow(clippy::too_many_arguments)]
    pub fn fill_path(
        &self,
        path: &Path,
        even_odd: bool,
        ctm: &Matrix,
        cs: &Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_fill_path(
                context(),
                self.dev,
                path.inner,
                even_odd,
                ctm.into(),
                cs.inner,
                color.as_ptr(),
                alpha,
                cp.into()
            ))
        }
    }

    /// The colors in `color` must match the colorspace `cs`, as described in
    /// [`Colorspace::convert_color`]
    #[allow(clippy::too_many_arguments)]
    pub fn stroke_path(
        &self,
        path: &Path,
        stroke: &StrokeState,
        ctm: &Matrix,
        cs: &Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_stroke_path(
                context(),
                self.dev,
                path.inner,
                stroke.inner,
                ctm.into(),
                cs.inner,
                color.as_ptr(),
                alpha,
                cp.into()
            ))
        }
    }

    pub fn clip_path(&self, path: &Path, even_odd: bool, ctm: &Matrix) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_clip_path(
                context(),
                self.dev,
                path.inner,
                even_odd,
                ctm.into()
            ))
        }
    }

    pub fn clip_stroke_path(
        &self,
        path: &Path,
        stroke: &StrokeState,
        ctm: &Matrix,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_clip_stroke_path(
                context(),
                self.dev,
                path.inner,
                stroke.inner,
                ctm.into()
            ))
        }
    }

    /// The colors in `color` must match the colorspace `cs`, as described in
    /// [`Colorspace::convert_color`]
    pub fn fill_text(
        &self,
        text: &Text,
        ctm: &Matrix,
        cs: &Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_fill_text(
                context(),
                self.dev,
                text.inner,
                ctm.into(),
                cs.inner,
                color.as_ptr(),
                alpha,
                cp.into()
            ))
        }
    }

    /// The colors in `color` must match the colorspace `cs`, as described in
    /// [`Colorspace::convert_color`]
    #[allow(clippy::too_many_arguments)]
    pub fn stroke_text(
        &self,
        text: &Text,
        stroke: &StrokeState,
        ctm: &Matrix,
        cs: &Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_stroke_text(
                context(),
                self.dev,
                text.inner,
                stroke.inner,
                ctm.into(),
                cs.inner,
                color.as_ptr(),
                alpha,
                cp.into()
            ))
        }
    }

    pub fn clip_text(&self, text: &Text, ctm: &Matrix) -> Result<(), Error> {
        unsafe { ffi_try!(mupdf_clip_text(context(), self.dev, text.inner, ctm.into())) }
    }

    pub fn clip_stroke_text(
        &self,
        text: &Text,
        stroke: &StrokeState,
        ctm: &Matrix,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_clip_stroke_text(
                context(),
                self.dev,
                text.inner,
                stroke.inner,
                ctm.into()
            ))
        }
    }

    pub fn ignore_text(&self, text: &Text, ctm: &Matrix) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_ignore_text(
                context(),
                self.dev,
                text.inner,
                ctm.into()
            ))
        }
    }

    pub fn fill_shade(
        &self,
        shd: &Shade,
        ctm: &Matrix,
        alpha: f32,
        cp: ColorParams,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_fill_shade(
                context(),
                self.dev,
                shd.inner,
                ctm.into(),
                alpha,
                cp.into()
            ))
        }
    }

    pub fn fill_image(
        &self,
        image: &Image,
        ctm: &Matrix,
        alpha: f32,
        cp: ColorParams,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_fill_image(
                context(),
                self.dev,
                image.inner,
                ctm.into(),
                alpha,
                cp.into()
            ))
        }
    }

    /// The colors in `color` must match the colorspace `cs`, as described in
    /// [`Colorspace::convert_color`]
    pub fn fill_image_mask(
        &self,
        image: &Image,
        ctm: &Matrix,
        cs: &Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_fill_image_mask(
                context(),
                self.dev,
                image.inner,
                ctm.into(),
                cs.inner,
                color.as_ptr(),
                alpha,
                cp.into()
            ))
        }
    }

    pub fn clip_image_mask(&self, image: &Image, ctm: &Matrix) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_clip_image_mask(
                context(),
                self.dev,
                image.inner,
                ctm.into()
            ))
        }
    }

    pub fn pop_clip(&self) -> Result<(), Error> {
        unsafe { ffi_try!(mupdf_pop_clip(context(), self.dev)) }
    }

    pub fn begin_mask(
        &self,
        area: Rect,
        luminosity: bool,
        cs: &Colorspace,
        bc: &[f32],
        cp: ColorParams,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_begin_mask(
                context(),
                self.dev,
                area.into(),
                luminosity,
                cs.inner,
                bc.as_ptr(),
                cp.into()
            ))
        }
    }

    pub fn end_mask(&self, f: Option<&Function>) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_end_mask(
                context(),
                self.dev,
                f.map_or(ptr::null_mut(), |f| f.inner)
            ))
        }
    }

    pub fn begin_group(
        &self,
        area: Rect,
        cs: &Colorspace,
        isolated: bool,
        knockout: bool,
        blend_mode: BlendMode,
        alpha: f32,
    ) -> Result<(), Error> {
        unsafe {
            ffi_try!(mupdf_begin_group(
                context(),
                self.dev,
                area.into(),
                cs.inner,
                isolated,
                knockout,
                blend_mode.into(),
                alpha
            ))
        }
    }

    pub fn end_group(&self) -> Result<(), Error> {
        unsafe { ffi_try!(mupdf_end_group(context(), self.dev)) }
    }

    pub fn begin_tile(
        &self,
        area: Rect,
        view: Rect,
        xstep: f32,
        ystep: f32,
        ctm: &Matrix,
        id: Option<NonZero<i32>>,
    ) -> Result<Option<NonZero<i32>>, Error> {
        unsafe {
            ffi_try!(mupdf_begin_tile(
                context(),
                self.dev,
                area.into(),
                view.into(),
                xstep,
                ystep,
                ctm.into(),
                id.map_or(0, NonZero::get)
            ))
        }
        .map(NonZero::new)
    }

    pub fn end_tile(&self) -> Result<(), Error> {
        unsafe { ffi_try!(mupdf_end_tile(context(), self.dev)) }
    }

    pub fn begin_layer(&self, name: &str) -> Result<(), Error> {
        let c_name = CString::new(name)?;
        unsafe { ffi_try!(mupdf_begin_layer(context(), self.dev, c_name.as_ptr())) }
    }

    pub fn end_layer(&self) -> Result<(), Error> {
        unsafe { ffi_try!(mupdf_end_layer(context(), self.dev)) }
    }

    pub fn begin_structure(&self, standard: Structure, raw: &str, idx: i32) -> Result<(), Error> {
        let c_raw = CString::new(raw)?;
        unsafe {
            ffi_try!(mupdf_begin_structure(
                context(),
                self.dev,
                standard.into(),
                c_raw.as_ptr(),
                idx
            ))
        }
    }

    pub fn end_structure(&self) -> Result<(), Error> {
        unsafe { ffi_try!(mupdf_end_structure(context(), self.dev)) }
    }

    pub fn begin_metatext(&self, meta: Metatext, text: &str) -> Result<(), Error> {
        let c_text = CString::new(text)?;
        unsafe {
            ffi_try!(mupdf_begin_metatext(
                context(),
                self.dev,
                meta.into(),
                c_text.as_ptr()
            ))
        }
    }

    pub fn end_metatext(&self) -> Result<(), Error> {
        unsafe { ffi_try!(mupdf_end_metatext(context(), self.dev)) }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        if !self.dev.is_null() {
            unsafe {
                fz_close_device(context(), self.dev);
                fz_drop_device(context(), self.dev);
            }
        }
        if !self.list.is_null() {
            unsafe {
                fz_drop_display_list(context(), self.list);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{Colorspace, Device, DisplayList, Pixmap, Rect};

    #[test]
    fn test_new_device_from_pixmap() {
        let cs = Colorspace::device_rgb();
        let mut pixmap = Pixmap::new_with_w_h(&cs, 100, 100, false).expect("Pixmap::new_with_w_h");
        pixmap.clear().unwrap();
        let _device = Device::from_pixmap(&pixmap).unwrap();
    }

    #[test]
    fn test_new_device_from_display_list() {
        let list = DisplayList::new(Rect::new(0.0, 0.0, 100.0, 100.0)).unwrap();
        let _device = Device::from_display_list(&list).unwrap();
    }
}

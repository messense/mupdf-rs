use std::ptr;
use std::{ffi::CString, num::NonZero};

use bitflags::bitflags;
use mupdf_sys::*;
use num_enum::TryFromPrimitive;

use crate::{
    context, ColorParams, Colorspace, DisplayList, Error, FFIWrapper, IRect, Image, Matrix, Path,
    Pixmap, Rect, Shade, StrokeState, Text, TextPage, TextPageFlags,
};

mod native;
pub use native::NativeDevice;

#[derive(Debug, Clone, Copy, PartialEq, TryFromPrimitive)]
#[repr(u32)]
pub enum BlendMode {
    /* PDF 1.4 -- standard separable */
    Normal = FZ_BLEND_NORMAL as u32,
    Multiply = FZ_BLEND_MULTIPLY as u32,
    Screen = FZ_BLEND_SCREEN as u32,
    Overlay = FZ_BLEND_OVERLAY as u32,
    Darken = FZ_BLEND_DARKEN as u32,
    Lighten = FZ_BLEND_LIGHTEN as u32,
    ColorDodge = FZ_BLEND_COLOR_DODGE as u32,
    ColorBurn = FZ_BLEND_COLOR_BURN as u32,
    HardLight = FZ_BLEND_HARD_LIGHT as u32,
    SoftLight = FZ_BLEND_SOFT_LIGHT as u32,
    Difference = FZ_BLEND_DIFFERENCE as u32,
    Exclusion = FZ_BLEND_EXCLUSION as u32,
    /* PDF 1.4 -- standard non-separable */
    Hue = FZ_BLEND_HUE as u32,
    Saturation = FZ_BLEND_SATURATION as u32,
    Color = FZ_BLEND_COLOR as u32,
    Luminosity = FZ_BLEND_LUMINOSITY as u32,
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
        // will probably be released in 1.26.x
        // const DASH_PATTERN_UNDEFINED = FZ_DEVFLAG_DASH_PATTERN_UNDEFINED as _;
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, TryFromPrimitive)]
#[repr(i32)]
pub enum Structure {
    Invalid = fz_structure_FZ_STRUCTURE_INVALID as _,

    /* Grouping elements (PDF 1.7 - Table 10.20) */
    Document = fz_structure_FZ_STRUCTURE_DOCUMENT as _,
    Part = fz_structure_FZ_STRUCTURE_PART as _,
    Art = fz_structure_FZ_STRUCTURE_ART as _,
    Sect = fz_structure_FZ_STRUCTURE_SECT as _,
    Div = fz_structure_FZ_STRUCTURE_DIV as _,
    BlockQuote = fz_structure_FZ_STRUCTURE_BLOCKQUOTE as _,
    Caption = fz_structure_FZ_STRUCTURE_CAPTION as _,
    TOC = fz_structure_FZ_STRUCTURE_TOC as _,
    TOCI = fz_structure_FZ_STRUCTURE_TOCI as _,
    Index = fz_structure_FZ_STRUCTURE_INDEX as _,
    NonStruct = fz_structure_FZ_STRUCTURE_NONSTRUCT as _,
    Private = fz_structure_FZ_STRUCTURE_PRIVATE as _,
    /* Grouping elements (PDF 2.0 - Table 364) */
    DocumentFragment = fz_structure_FZ_STRUCTURE_DOCUMENTFRAGMENT as _,
    /* Grouping elements (PDF 2.0 - Table 365) */
    Aside = fz_structure_FZ_STRUCTURE_ASIDE as _,
    /* Grouping elements (PDF 2.0 - Table 366) */
    Title = fz_structure_FZ_STRUCTURE_TITLE as _,
    FENote = fz_structure_FZ_STRUCTURE_FENOTE as _,
    /* Grouping elements (PDF 2.0 - Table 367) */
    Sub = fz_structure_FZ_STRUCTURE_SUB as _,

    /* Paragraphlike elements (PDF 1.7 - Table 10.21) */
    P = fz_structure_FZ_STRUCTURE_P as _,
    H = fz_structure_FZ_STRUCTURE_H as _,
    H1 = fz_structure_FZ_STRUCTURE_H1 as _,
    H2 = fz_structure_FZ_STRUCTURE_H2 as _,
    H3 = fz_structure_FZ_STRUCTURE_H3 as _,
    H4 = fz_structure_FZ_STRUCTURE_H4 as _,
    H5 = fz_structure_FZ_STRUCTURE_H5 as _,
    H6 = fz_structure_FZ_STRUCTURE_H6 as _,

    /* List elements (PDF 1.7 - Table 10.23) */
    List = fz_structure_FZ_STRUCTURE_LIST as _,
    ListItem = fz_structure_FZ_STRUCTURE_LISTITEM as _,
    Label = fz_structure_FZ_STRUCTURE_LABEL as _,
    ListBody = fz_structure_FZ_STRUCTURE_LISTBODY as _,

    /* Table elements (PDF 1.7 - Table 10.24) */
    Table = fz_structure_FZ_STRUCTURE_TABLE as _,
    TR = fz_structure_FZ_STRUCTURE_TR as _,
    TH = fz_structure_FZ_STRUCTURE_TH as _,
    TD = fz_structure_FZ_STRUCTURE_TD as _,
    THead = fz_structure_FZ_STRUCTURE_THEAD as _,
    TBody = fz_structure_FZ_STRUCTURE_TBODY as _,
    TFoot = fz_structure_FZ_STRUCTURE_TFOOT as _,

    /* Inline elements (PDF 1.7 - Table 10.25) */
    Span = fz_structure_FZ_STRUCTURE_SPAN as _,
    Quote = fz_structure_FZ_STRUCTURE_QUOTE as _,
    Note = fz_structure_FZ_STRUCTURE_NOTE as _,
    Reference = fz_structure_FZ_STRUCTURE_REFERENCE as _,
    BibEntry = fz_structure_FZ_STRUCTURE_BIBENTRY as _,
    Code = fz_structure_FZ_STRUCTURE_CODE as _,
    Link = fz_structure_FZ_STRUCTURE_LINK as _,
    Annot = fz_structure_FZ_STRUCTURE_ANNOT as _,
    /* Inline elements (PDF 2.0 - Table 368) */
    Em = fz_structure_FZ_STRUCTURE_EM as _,
    Strong = fz_structure_FZ_STRUCTURE_STRONG as _,

    /* Ruby inline element (PDF 1.7 - Table 10.26) */
    Ruby = fz_structure_FZ_STRUCTURE_RUBY as _,
    RB = fz_structure_FZ_STRUCTURE_RB as _,
    RT = fz_structure_FZ_STRUCTURE_RT as _,
    RP = fz_structure_FZ_STRUCTURE_RP as _,

    /* Warichu inline element (PDF 1.7 - Table 10.26) */
    Warichu = fz_structure_FZ_STRUCTURE_WARICHU as _,
    WT = fz_structure_FZ_STRUCTURE_WT as _,
    WP = fz_structure_FZ_STRUCTURE_WP as _,

    /* Illustration elements (PDF 1.7 - Table 10.27) */
    Figure = fz_structure_FZ_STRUCTURE_FIGURE as _,
    Formula = fz_structure_FZ_STRUCTURE_FORMULA as _,
    Form = fz_structure_FZ_STRUCTURE_FORM as _,

    /* Artifact structure type (PDF 2.0 - Table 375) */
    Artifact = fz_structure_FZ_STRUCTURE_ARTIFACT as _,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, TryFromPrimitive)]
#[repr(u32)]
pub enum Metatext {
    ActualText = fz_metatext_FZ_METATEXT_ACTUALTEXT as _,
    Alt = fz_metatext_FZ_METATEXT_ALT as _,
    Abbreviation = fz_metatext_FZ_METATEXT_ABBREVIATION as _,
    Title = fz_metatext_FZ_METATEXT_TITLE as _,
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
                blend_mode as _,
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
                standard as _,
                c_raw.as_ptr(),
                idx as _
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
                meta as _,
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

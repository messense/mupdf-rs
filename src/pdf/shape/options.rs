use crate::{CjkFontOrdering, Matrix, Point, SimpleFontEncoding, WriteMode};

/// Color components for Shape drawing operators.
#[derive(Clone, Debug, PartialEq)]
pub enum PdfColor {
    /// DeviceGray color component.
    Gray(f32),
    /// DeviceRGB color components.
    Rgb([f32; 3]),
    /// DeviceCMYK color components.
    Cmyk([f32; 4]),
}

impl PdfColor {
    /// Creates a DeviceGray color.
    pub fn gray(gray: f32) -> Self {
        Self::Gray(gray)
    }

    /// Creates a DeviceRGB color.
    pub fn rgb(red: f32, green: f32, blue: f32) -> Self {
        Self::Rgb([red, green, blue])
    }

    /// Creates a DeviceCMYK color.
    pub fn cmyk(cyan: f32, magenta: f32, yellow: f32, key: f32) -> Self {
        Self::Cmyk([cyan, magenta, yellow, key])
    }

    pub(crate) fn components(&self) -> &[f32] {
        match self {
            Self::Gray(components) => std::slice::from_ref(components),
            Self::Rgb(components) => components,
            Self::Cmyk(components) => components,
        }
    }
}

impl From<[f32; 1]> for PdfColor {
    fn from(value: [f32; 1]) -> Self {
        Self::Gray(value[0])
    }
}

impl From<[f32; 3]> for PdfColor {
    fn from(value: [f32; 3]) -> Self {
        Self::Rgb(value)
    }
}

impl From<[f32; 4]> for PdfColor {
    fn from(value: [f32; 4]) -> Self {
        Self::Cmyk(value)
    }
}

/// Options controlling how the currently accumulated Shape path is painted.
#[derive(Clone, Debug, PartialEq)]
pub struct FinishOptions {
    /// Stroke color. `None` disables stroking.
    pub color: Option<PdfColor>,
    /// Fill color. `None` disables filling.
    pub fill: Option<PdfColor>,
    /// Stroke width in PDF user-space units.
    pub width: f32,
    /// Optional line cap style for the PDF `J` operator.
    pub line_cap: Option<i32>,
    /// Optional line join style for the PDF `j` operator.
    pub line_join: Option<i32>,
    /// Optional miter limit for the PDF `M` operator.
    pub miter_limit: Option<f32>,
    /// Optional dash pattern operand, excluding the trailing `d` operator.
    pub dashes: Option<String>,
    /// Whether fills use the even-odd rule.
    pub even_odd: bool,
    /// Whether to close the current path before painting.
    pub close_path: bool,
    /// Optional fixed-point morph transform applied to this finished drawing block.
    pub morph: Option<(Point, Matrix)>,
    /// Optional stroke alpha for PDF `/ExtGState` `/CA`.
    pub stroke_opacity: Option<f32>,
    /// Optional fill alpha for PDF `/ExtGState` `/ca`.
    pub fill_opacity: Option<f32>,
    /// Optional-content group or membership dictionary xref for PDF marked content.
    pub oc: Option<i32>,
}

impl Default for FinishOptions {
    fn default() -> Self {
        Self {
            color: Some(PdfColor::Rgb([0.0, 0.0, 0.0])),
            fill: None,
            width: 1.0,
            line_cap: None,
            line_join: None,
            miter_limit: None,
            dashes: None,
            even_odd: false,
            close_path: false,
            morph: None,
            stroke_opacity: None,
            fill_opacity: None,
            oc: None,
        }
    }
}

/// Options controlling text inserted by [`Shape::insert_text`](super::Shape::insert_text).
#[derive(Clone, Debug, PartialEq)]
pub struct TextOptions<'a> {
    /// Font size in PDF user-space units.
    pub fontsize: f32,
    /// Line height multiplier. Consecutive baselines are spaced by `fontsize * lineheight`.
    pub lineheight: f32,
    /// Base-14 font alias or canonical font name. Defaults to PyMuPDF's `helv`.
    pub fontname: String,
    /// Optional TrueType/OpenType font bytes to register for this text block.
    pub fontfile: Option<&'a [u8]>,
    /// Stroke color used by text rendering modes that stroke glyph outlines.
    pub color: Option<PdfColor>,
    /// Fill color used by text rendering modes that fill glyph outlines.
    pub fill: Option<PdfColor>,
    /// PDF text rendering mode for the `Tr` operator.
    pub render_mode: i32,
    /// Border width multiplier. Emitted line width is `border_width * fontsize`.
    pub border_width: f32,
    /// Optional miter limit for stroked glyph outlines.
    pub miter_limit: Option<f32>,
    /// Clockwise text rotation in degrees. Only 0, 90, 180, and 270 are supported.
    pub rotate: i32,
    /// Whether the font should be registered as a simple font.
    pub simple: bool,
    /// Encoding used when registering a simple font.
    pub encoding: SimpleFontEncoding,
    /// Optional CJK collection ordering for composite font registration.
    pub ordering: Option<CjkFontOrdering>,
    /// Writing mode used when registering a CJK font.
    pub wmode: WriteMode,
    /// Whether CJK fallback metrics should prefer serif glyphs.
    pub serif: bool,
    /// Optional stroke alpha for PDF `/ExtGState` `/CA`.
    pub stroke_opacity: Option<f32>,
    /// Optional fill alpha for PDF `/ExtGState` `/ca`.
    pub fill_opacity: Option<f32>,
    /// Optional-content group or membership dictionary xref for PDF marked content.
    pub oc: Option<i32>,
}

impl Default for TextOptions<'_> {
    fn default() -> Self {
        Self {
            fontsize: 11.0,
            lineheight: 1.2,
            fontname: "helv".to_owned(),
            fontfile: None,
            color: None,
            fill: None,
            render_mode: 0,
            border_width: 0.05,
            miter_limit: Some(1.0),
            rotate: 0,
            simple: true,
            encoding: SimpleFontEncoding::Latin,
            ordering: None,
            wmode: WriteMode::Horizontal,
            serif: false,
            stroke_opacity: None,
            fill_opacity: None,
            oc: None,
        }
    }
}

/// Text alignment for [`Shape::insert_textbox`](super::Shape::insert_textbox).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAlign {
    /// Align each line to the leading edge of the textbox.
    #[default]
    Left,
    /// Center each line within the textbox.
    Center,
    /// Align each line to the trailing edge of the textbox.
    Right,
    /// Fully justify non-last paragraph lines by distributing extra width across word gaps.
    Justify,
}

/// Options controlling text inserted by [`Shape::insert_textbox`](super::Shape::insert_textbox).
#[derive(Clone, Debug, PartialEq)]
pub struct TextboxOptions<'a> {
    /// Font size in PDF user-space units.
    pub fontsize: f32,
    /// Line height multiplier. Consecutive baselines are spaced by `fontsize * lineheight`.
    pub lineheight: f32,
    /// Base-14 font alias or canonical font name. Defaults to PyMuPDF's `helv`.
    pub fontname: String,
    /// Optional TrueType/OpenType font bytes to register for this text box.
    pub fontfile: Option<&'a [u8]>,
    /// Stroke color used by text rendering modes that stroke glyph outlines.
    pub color: Option<PdfColor>,
    /// Fill color used by text rendering modes that fill glyph outlines.
    pub fill: Option<PdfColor>,
    /// PDF text rendering mode for the `Tr` operator.
    pub render_mode: i32,
    /// Border width multiplier. Emitted line width is `border_width * fontsize`.
    pub border_width: f32,
    /// Optional miter limit for stroked glyph outlines.
    pub miter_limit: Option<f32>,
    /// Clockwise text rotation in degrees. Only 0, 90, 180, and 270 are supported.
    pub rotate: i32,
    /// Whether the font should be registered as a simple font.
    pub simple: bool,
    /// Encoding used when registering a simple font.
    pub encoding: SimpleFontEncoding,
    /// Optional CJK collection ordering for composite font registration.
    pub ordering: Option<CjkFontOrdering>,
    /// Writing mode used when registering a CJK font.
    pub wmode: WriteMode,
    /// Whether CJK fallback metrics should prefer serif glyphs.
    pub serif: bool,
    /// Line alignment within the textbox.
    pub align: TextAlign,
}

impl Default for TextboxOptions<'_> {
    fn default() -> Self {
        Self {
            fontsize: 11.0,
            lineheight: 1.2,
            fontname: "helv".to_owned(),
            fontfile: None,
            color: None,
            fill: None,
            render_mode: 0,
            border_width: 0.05,
            miter_limit: Some(1.0),
            rotate: 0,
            simple: true,
            encoding: SimpleFontEncoding::Latin,
            ordering: None,
            wmode: WriteMode::Horizontal,
            serif: false,
            align: TextAlign::Left,
        }
    }
}

impl<'a> From<TextOptions<'a>> for TextboxOptions<'a> {
    fn from(value: TextOptions<'a>) -> Self {
        Self {
            fontsize: value.fontsize,
            lineheight: value.lineheight,
            fontname: value.fontname,
            fontfile: value.fontfile,
            color: value.color,
            fill: value.fill,
            render_mode: value.render_mode,
            border_width: value.border_width,
            miter_limit: value.miter_limit,
            rotate: value.rotate,
            simple: value.simple,
            encoding: value.encoding,
            ordering: value.ordering,
            wmode: value.wmode,
            serif: value.serif,
            align: TextAlign::Left,
        }
    }
}

use crate::{CjkFontOrdering, Error, Matrix, Point, SimpleFontEncoding, WriteMode};

/// Color components for Shape drawing operators.
///
/// ```
/// use mupdf::shape::PdfColor;
///
/// let stroke = PdfColor::rgb(1.0, 0.0, 0.0);
/// let fill = PdfColor::gray(0.5);
/// assert_ne!(stroke, fill);
/// ```
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

    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self
            .components()
            .iter()
            .all(|component| component.is_finite() && (0.0..=1.0).contains(component))
        {
            return Ok(());
        }

        Err(Error::InvalidArgument(
            "color components must be finite values in the 0..=1 range".to_owned(),
        ))
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
///
/// ```
/// use mupdf::{pdf::PdfDocument, shape::{FinishOptions, PdfColor, Shape}, Point, Size};
///
/// # fn main() -> Result<(), mupdf::Error> {
/// let mut doc = PdfDocument::new();
/// let mut page = doc.new_page(Size::A4)?;
/// let mut shape = Shape::new(&mut page)?;
/// let opts = FinishOptions {
///     color: Some(PdfColor::rgb(0.0, 0.0, 1.0)),
///     width: 2.0,
///     ..Default::default()
/// };
/// shape
///     .draw_line(Point::new(72.0, 72.0), Point::new(180.0, 72.0))?
///     .finish(&opts)?
///     .commit(&mut doc, true)?;
/// # Ok(())
/// # }
/// ```
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
            close_path: true,
            morph: None,
            stroke_opacity: None,
            fill_opacity: None,
            oc: None,
        }
    }
}

/// Options controlling text inserted by [`Shape::insert_text`](super::Shape::insert_text).
///
/// ```
/// use mupdf::{pdf::PdfDocument, shape::{PdfColor, Shape, TextOptions}, Point, Size};
///
/// # fn main() -> Result<(), mupdf::Error> {
/// let mut doc = PdfDocument::new();
/// let mut page = doc.new_page(Size::A4)?;
/// let mut shape = Shape::new(&mut page)?;
/// let opts = TextOptions {
///     fontsize: 18.0,
///     fill: Some(PdfColor::rgb(0.0, 0.0, 0.0)),
///     ..Default::default()
/// };
/// shape
///     .insert_text(Point::new(72.0, 96.0), "Hello", &opts)?
///     .commit(&mut doc, true)?;
/// # Ok(())
/// # }
/// ```
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
    /// Clockwise text rotation in degrees. Any multiple of 90 is accepted and normalized.
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
///
/// ```
/// use mupdf::shape::{TextAlign, TextboxOptions};
///
/// let opts = TextboxOptions {
///     align: TextAlign::Justify,
///     ..Default::default()
/// };
/// assert_eq!(opts.align, TextAlign::Justify);
/// ```
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
///
/// ```
/// use mupdf::{pdf::PdfDocument, shape::{Shape, TextboxOptions}, Rect, Size};
///
/// # fn main() -> Result<(), mupdf::Error> {
/// let mut doc = PdfDocument::new();
/// let mut page = doc.new_page(Size::A4)?;
/// let mut shape = Shape::new(&mut page)?;
/// let opts = TextboxOptions {
///     fontsize: 14.0,
///     ..Default::default()
/// };
/// let unused = shape.insert_textbox(
///     Rect::new(72.0, 72.0, 220.0, 150.0),
///     "A short text box example.",
///     &opts,
/// )?;
/// assert!(unused >= 0.0);
/// shape.commit(&mut doc, true)?;
/// # Ok(())
/// # }
/// ```
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
    /// Clockwise text rotation in degrees. Any multiple of 90 is accepted and normalized.
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
            stroke_opacity: None,
            fill_opacity: None,
            oc: None,
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
            stroke_opacity: value.stroke_opacity,
            fill_opacity: value.fill_opacity,
            oc: value.oc,
            align: TextAlign::Left,
        }
    }
}

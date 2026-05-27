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
        }
    }
}

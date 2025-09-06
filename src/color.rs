#[derive(Copy, Clone, Default)]
pub struct Color {
    pub alpha: u8,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

// We'll be using this in a later PR, just adding the struct right now
#[expect(dead_code)]
impl Color {
    fn into_bytes(self) -> [u8; 4] {
        let Color {
            alpha,
            red,
            green,
            blue,
        } = self;
        [alpha, red, green, blue]
    }

    pub(crate) fn into_mupdf_float(self) -> f32 {
        f32::from_be_bytes(self.into_bytes())
    }

    pub(crate) fn from_mupdf_float(f: f32) -> Self {
        let [alpha, red, green, blue] = f.to_be_bytes();
        Self {
            alpha,
            red,
            green,
            blue,
        }
    }

    pub(crate) fn into_mupdf_int(self) -> i32 {
        i32::from_be_bytes(self.into_bytes())
    }

    pub(crate) fn from_mupdf_int(i: i32) -> Self {
        let [alpha, red, green, blue] = i.to_be_bytes();
        Self {
            alpha,
            red,
            green,
            blue,
        }
    }
}

/// The method used to set colors for [`PdfAnnotation::set_color`] - each float inside should
/// contain a value between [0, 1.0], with 1.0 being the most intense. A 1.0 for Self::Gray
/// indicates white.
///
/// [`PdfAnnotation::set_color`]: crate::pdf::annotation::PdfAnnotation::set_color
pub enum AnnotationColor {
    Gray(f32),
    Rgb {
        red: f32,
        green: f32,
        blue: f32,
    },
    Cmyk {
        cyan: f32,
        magenta: f32,
        yellow: f32,
        key: f32,
    },
}

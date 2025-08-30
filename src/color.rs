pub struct Color {
    pub alpha: u8,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

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
}

impl From<f32> for Color {
    fn from(value: f32) -> Self {
        let [alpha, red, green, blue] = value.to_be_bytes();
        Self {
            alpha,
            red,
            green,
            blue,
        }
    }
}

impl From<Color> for f32 {
    fn from(value: Color) -> Self {
        Self::from_be_bytes(value.into_bytes())
    }
}

impl From<i32> for Color {
    fn from(value: i32) -> Self {
        let [alpha, red, green, blue] = value.to_be_bytes();
        Self {
            alpha,
            red,
            green,
            blue,
        }
    }
}

impl From<Color> for i32 {
    fn from(value: Color) -> Self {
        Self::from_be_bytes(value.into_bytes())
    }
}

/// The method used to set colors for [`PdfAnnotation::set_color`] - each float inside should
/// contain a value between [0, 1.0], with 1.0 being the most intense. A 1.0 for Self::Gray
/// indicates white.
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

use mupdf_sys::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderingIntent {
    Perceptual,
    RelativeColorimetric,
    Saturation,
    AbsoluteColorimetric,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorParams(i32);

impl ColorParams {
    const BP: i32 = 32;
    const OP: i32 = 64;
    const OPM: i32 = 128;

    pub fn rendering_intent(flags: i32) -> RenderingIntent {
        match flags & 3 {
            0 => RenderingIntent::Perceptual,
            1 => RenderingIntent::RelativeColorimetric,
            2 => RenderingIntent::Saturation,
            3 => RenderingIntent::AbsoluteColorimetric,
            _ => RenderingIntent::Perceptual,
        }
    }

    pub fn is_bp(flags: i32) -> bool {
        flags & Self::BP != 0
    }

    pub fn is_op(flags: i32) -> bool {
        flags & Self::OP != 0
    }

    pub fn is_opm(flags: i32) -> bool {
        flags & Self::OPM != 0
    }

    pub fn new(ri: RenderingIntent, bp: bool, op: bool, opm: bool) -> Self {
        let mut flags = match ri {
            RenderingIntent::Perceptual => 0,
            RenderingIntent::RelativeColorimetric => 1,
            RenderingIntent::Saturation => 2,
            RenderingIntent::AbsoluteColorimetric => 3,
        };
        if bp {
            flags |= Self::BP;
        }
        if op {
            flags |= Self::OP;
        }
        if opm {
            flags |= Self::OPM;
        }
        Self(flags)
    }
}

impl From<ColorParams> for fz_color_params {
    fn from(val: ColorParams) -> Self {
        let flags = val.0;
        let bp = ((flags >> 5) & 1) as u8;
        let op = ((flags >> 6) & 1) as u8;
        let opm = ((flags >> 7) & 1) as u8;
        let ri = (flags & 32) as u8;
        fz_color_params { ri, bp, op, opm }
    }
}

impl Default for ColorParams {
    fn default() -> Self {
        Self::new(RenderingIntent::RelativeColorimetric, true, false, false)
    }
}

use std::fmt::{self, Debug, Formatter};

use mupdf_sys::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderingIntent {
    Perceptual,
    RelativeColorimetric,
    Saturation,
    AbsoluteColorimetric,
}

#[derive(Clone, Copy, PartialEq)]
pub struct ColorParams(u8);

impl ColorParams {
    const BP: u8 = 1 << 2;
    const OP: u8 = 1 << 3;
    const OPM: u8 = 1 << 4;

    pub fn new(ri: RenderingIntent, bp: bool, op: bool, opm: bool) -> Self {
        let ri = match ri {
            RenderingIntent::Perceptual => 0,
            RenderingIntent::RelativeColorimetric => 1,
            RenderingIntent::Saturation => 2,
            RenderingIntent::AbsoluteColorimetric => 3,
        };
        Self::raw_new(ri, bp, op, opm)
    }

    fn raw_new(ri: u8, bp: bool, op: bool, opm: bool) -> Self {
        let mut flags = ri;
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

    pub fn rendering_intent(self) -> RenderingIntent {
        match self.0 & 3 {
            0 => RenderingIntent::Perceptual,
            1 => RenderingIntent::RelativeColorimetric,
            2 => RenderingIntent::Saturation,
            3 => RenderingIntent::AbsoluteColorimetric,
            _ => unreachable!(),
        }
    }

    pub fn bp(self) -> bool {
        self.0 & Self::BP != 0
    }

    pub fn op(self) -> bool {
        self.0 & Self::OP != 0
    }

    pub fn opm(self) -> bool {
        self.0 & Self::OPM != 0
    }
}

impl From<fz_color_params> for ColorParams {
    fn from(value: fz_color_params) -> Self {
        Self::raw_new(value.ri & 3, value.bp != 0, value.op != 0, value.opm != 0)
    }
}

impl From<ColorParams> for fz_color_params {
    fn from(val: ColorParams) -> Self {
        fz_color_params {
            ri: val.rendering_intent() as u8,
            bp: val.bp() as u8,
            op: val.op() as u8,
            opm: val.opm() as u8,
        }
    }
}

impl Debug for ColorParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ColorParams")
            .field("rendering_intent", &self.rendering_intent())
            .field("bp", &self.bp())
            .field("op", &self.op())
            .field("opm", &self.opm())
            .finish()
    }
}

impl Default for ColorParams {
    fn default() -> Self {
        Self::new(RenderingIntent::RelativeColorimetric, true, false, false)
    }
}

#[cfg(test)]
mod test {
    use super::{fz_color_params, ColorParams, RenderingIntent};

    #[test]
    fn test_roundtrip_color_params() {
        let expected = ColorParams::new(RenderingIntent::Saturation, true, false, false);
        let got = ColorParams::from(fz_color_params::from(expected));
        assert_eq!(got, expected);
    }
}

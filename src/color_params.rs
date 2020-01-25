#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderingIntent {
    Perceptual,
    RelativeColorimetric,
    Saturation,
    AbsoluteColorimetric,
}

#[derive(Debug)]
pub struct ColorParams(i32);

impl ColorParams {
    pub const BP: i32 = 32;
    pub const OP: i32 = 64;
    pub const OPM: i32 = 128;

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

    pub fn pack(ri: RenderingIntent, bp: bool, op: bool, opm: bool) -> i32 {
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
        flags
    }
}

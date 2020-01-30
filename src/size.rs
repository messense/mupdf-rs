#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    // ISO Sizes A series
    pub const FOUR_A0: Self = Size::new(4768.0, 6741.0);
    pub const TWO_A0: Self = Size::new(3370.0, 4768.0);
    pub const A0: Self = Size::new(2384.0, 3370.0);
    pub const A1: Self = Size::new(1684.0, 2384.0);
    pub const A2: Self = Size::new(1191.0, 1684.0);
    pub const A3: Self = Size::new(842.0, 1191.0);
    pub const A4: Self = Size::new(595.0, 842.0);
    pub const A5: Self = Size::new(420.0, 595.0);
    pub const A6: Self = Size::new(298.0, 420.0);
    pub const A7: Self = Size::new(210.0, 298.0);
    pub const A8: Self = Size::new(147.0, 210.0);
    pub const A9: Self = Size::new(105.0, 147.0);
    pub const A10: Self = Size::new(74.0, 105.0);
    // ISO Sizes B series
    pub const B0: Self = Size::new(2836.0, 4008.0);
    pub const B1: Self = Size::new(2004.0, 2835.0);
    pub const B2: Self = Size::new(1417.0, 2004.0);
    pub const B3: Self = Size::new(1001.0, 1417.0);
    pub const B4: Self = Size::new(709.0, 1001.0);
    pub const B5: Self = Size::new(499.0, 709.0);
    pub const B6: Self = Size::new(354.0, 499.0);
    pub const B7: Self = Size::new(249.0, 354.0);
    pub const B8: Self = Size::new(176.0, 249.0);
    pub const B9: Self = Size::new(125.0, 176.0);
    pub const B10: Self = Size::new(88.0, 125.0);

    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

impl From<(f32, f32)> for Size {
    fn from(s: (f32, f32)) -> Self {
        Self {
            width: s.0,
            height: s.1,
        }
    }
}

impl From<(i32, i32)> for Size {
    fn from(s: (i32, i32)) -> Self {
        Self {
            width: s.0 as f32,
            height: s.1 as f32,
        }
    }
}

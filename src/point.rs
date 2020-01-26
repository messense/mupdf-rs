use mupdf_sys::fz_point;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    x: f32,
    y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl From<fz_point> for Point {
    fn from(p: fz_point) -> Self {
        Self { x: p.x, y: p.y }
    }
}

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

impl From<(f32, f32)> for Point {
    fn from(p: (f32, f32)) -> Self {
        Self { x: p.0, y: p.1 }
    }
}

impl From<(i32, i32)> for Point {
    fn from(p: (i32, i32)) -> Self {
        Self {
            x: p.0 as f32,
            y: p.1 as f32,
        }
    }
}

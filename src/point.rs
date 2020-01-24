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

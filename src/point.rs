use mupdf_sys::{fz_point, fz_transform_point};

use crate::{impl_ffi_traits, Matrix};

/// A point in a two-dimensional space.
#[derive(
    Debug, Clone, Copy, PartialEq, zerocopy::FromBytes, zerocopy::IntoBytes, zerocopy::Immutable,
)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Apply a transformation to a point.
    ///
    /// The NaN coordinates will be reset to 0.0,
    /// which make `fz_transform_point` works normally.
    /// Otherwise `(NaN, NaN)` will be returned.
    pub fn transform(mut self, matrix: &Matrix) -> Self {
        if self.x.is_nan() {
            self.x = 0.0;
        }
        if self.y.is_nan() {
            self.y = 0.0;
        }

        unsafe { fz_transform_point(self.into(), matrix.into()).into() }
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

impl_ffi_traits!(Point, fz_point);

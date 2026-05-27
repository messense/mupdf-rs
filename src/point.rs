use std::ops::{Mul, Sub};

use mupdf_sys::{fz_point, fz_transform_point};

use crate::{impl_ffi_traits, Matrix};

/// A point in a two-dimensional space.
/// This is marked `repr(c)` to ensure compatibility with the FFI analogue, [`fz_point`], so that
/// [`zerocopy::transmute`]ing between the two always preseves information correctly
#[derive(
    Debug, Clone, Copy, PartialEq, zerocopy::FromBytes, zerocopy::IntoBytes, zerocopy::Immutable,
)]
#[repr(C)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Returns the Euclidean length of the vector from the origin to this point.
    #[inline]
    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    /// Returns a unit vector in the same direction.
    ///
    /// Zero-length vectors return `(0, 0)` instead of producing `NaN`
    /// coordinates.
    #[inline]
    pub fn unit(self) -> Self {
        let length = self.length();
        if length == 0.0 {
            Self::new(0.0, 0.0)
        } else {
            Self::new(self.x / length, self.y / length)
        }
    }

    /// Applies a matrix transformation to this point.
    #[inline]
    pub fn mul_matrix(self, matrix: &Matrix) -> Self {
        self.transform(matrix)
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

impl Sub<Point> for Point {
    type Output = Self;

    fn sub(self, rhs: Point) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Mul<f32> for Point {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl_ffi_traits!(Point, fz_point);

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_point_near(actual: Point, expected: Point, epsilon: f32) {
        assert!(
            (actual.x - expected.x).abs() <= epsilon,
            "x mismatch: actual={}, expected={}",
            actual.x,
            expected.x
        );
        assert!(
            (actual.y - expected.y).abs() <= epsilon,
            "y mismatch: actual={}, expected={}",
            actual.y,
            expected.y
        );
    }

    #[test]
    fn arithmetic_length_and_unit_vectors() {
        assert_eq!(
            Point::new(5.0, 7.0) - Point::new(2.0, 3.0),
            Point::new(3.0, 4.0)
        );
        assert_eq!(Point::new(3.0, 4.0) * 2.5, Point::new(7.5, 10.0));
        assert!((Point::new(3.0, 4.0).length() - 5.0).abs() <= 1e-6);
        assert_point_near(Point::new(3.0, 4.0).unit(), Point::new(0.6, 0.8), 1e-6);

        let zero_unit = Point::new(0.0, 0.0).unit();
        assert_eq!(zero_unit, Point::new(0.0, 0.0));
        assert!(!zero_unit.x.is_nan());
        assert!(!zero_unit.y.is_nan());
    }

    #[test]
    fn mul_matrix_matches_transform() {
        let matrix = Matrix::new(1.5, 0.0, 0.0, 2.0, 10.0, 20.0);
        let point = Point::new(3.0, 4.0);
        let transformed = point.mul_matrix(&matrix);
        assert_point_near(transformed, point.transform(&matrix), 1e-6);
        assert_point_near(transformed, Point::new(14.5, 28.0), 1e-6);

        let rotated = Point::new(1.0, 0.0).mul_matrix(&Matrix::new_rotate(90.0));
        assert_point_near(rotated, Point::new(0.0, 1.0), 1e-4);
    }
}

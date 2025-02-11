use mupdf_sys::fz_quad;

use crate::{impl_ffi_traits, Point};

/// A representation for a region defined by 4 points
#[derive(
    Debug, Clone, PartialEq, zerocopy::FromBytes, zerocopy::IntoBytes, zerocopy::Immutable,
)]
pub struct Quad {
    pub ul: Point,
    pub ur: Point,
    pub ll: Point,
    pub lr: Point,
}

impl Quad {
    pub fn new(ul: Point, ur: Point, ll: Point, lr: Point) -> Self {
        Self { ul, ur, ll, lr }
    }
}

impl_ffi_traits!(Quad, fz_quad);

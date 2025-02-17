use mupdf_sys::fz_quad;

use crate::{impl_ffi_traits, Point};

/// A representation for a region defined by 4 points
/// This is marked `repr(c)` to ensure compatibility with the FFI analogue, [`fz_quad`], so that
/// [`zerocopy::transmute`]ing between the two always preseves information correctly
#[derive(
    Debug, Clone, PartialEq, zerocopy::FromBytes, zerocopy::IntoBytes, zerocopy::Immutable,
)]
#[repr(C)]
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

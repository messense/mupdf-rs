use mupdf_sys::*;

use crate::Point;

/// A representation for a region defined by 4 points
#[derive(Debug, Clone, PartialEq)]
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

impl From<fz_quad> for Quad {
    fn from(quad: fz_quad) -> Self {
        Self {
            ul: quad.ul.into(),
            ur: quad.ur.into(),
            ll: quad.ll.into(),
            lr: quad.lr.into(),
        }
    }
}

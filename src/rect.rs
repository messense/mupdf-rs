use std::fmt;

use mupdf_sys::{
    fz_intersect_irect, fz_intersect_rect, fz_irect, fz_irect_from_rect, fz_rect, fz_round_rect,
    fz_transform_rect, fz_union_rect, mupdf_adjust_rect_for_stroke,
};

use crate::{context, Error, Matrix, Point, Quad, Size, StrokeState};

const FZ_MIN_INF_RECT: i32 = 0x80000000u32 as i32;
const FZ_MAX_INF_RECT: i32 = 0x7fffff80u32 as i32;

/// A rectangle using integers instead of floats
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct IRect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl IRect {
    pub const INF: Self = IRect {
        x0: FZ_MIN_INF_RECT,
        y0: FZ_MIN_INF_RECT,
        x1: FZ_MAX_INF_RECT,
        y1: FZ_MAX_INF_RECT,
    };

    pub const fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        Self { x0, y0, x1, y1 }
    }

    pub fn is_empty(&self) -> bool {
        self.x0 >= self.x1 || self.y0 >= self.y1
    }

    pub fn is_valid(&self) -> bool {
        self.x0 <= self.x1 && self.y0 <= self.y1
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        if self.is_empty() {
            false
        } else {
            x >= self.x0 && x < self.x1 && y >= self.y0 && y < self.y1
        }
    }

    pub fn width(&self) -> i32 {
        if self.is_empty() {
            0
        } else {
            self.x1 - self.x0
        }
    }

    pub fn height(&self) -> i32 {
        if self.is_empty() {
            0
        } else {
            self.y1 - self.y0
        }
    }

    pub fn origin(&self) -> Point {
        Point::new(self.x0 as f32, self.y0 as f32)
    }

    pub fn size(&self) -> Size {
        Size::new(self.width() as f32, self.height() as f32)
    }

    pub fn r#union(&self, other: IRect) -> Self {
        if !self.is_valid() {
            other.clone()
        } else if !other.is_valid() {
            (*self).clone()
        } else {
            IRect {
                x0: if other.x0 < self.x0 {
                    other.x0
                } else {
                    self.x0
                },
                y0: if other.y0 < self.y0 {
                    other.y0
                } else {
                    self.y0
                },
                x1: if other.x1 > self.x1 {
                    other.x1
                } else {
                    self.x1
                },
                y1: if other.y1 > self.y1 {
                    other.y1
                } else {
                    self.y1
                },
            }
        }
    }

    pub fn intersect(&self, rect: &Self) -> Self {
        unsafe { fz_intersect_irect((*self).into(), (*rect).into()) }.into()
    }
}

impl fmt::Display for IRect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{} {} {} {}]", self.x0, self.y0, self.x1, self.y1)
    }
}

impl From<fz_irect> for IRect {
    fn from(r: fz_irect) -> IRect {
        IRect {
            x0: r.x0,
            y0: r.y0,
            x1: r.x1,
            y1: r.y1,
        }
    }
}

impl From<IRect> for fz_irect {
    fn from(r: IRect) -> Self {
        fz_irect {
            x0: r.x0,
            y0: r.y0,
            x1: r.x1,
            y1: r.y1,
        }
    }
}

impl From<Rect> for IRect {
    fn from(rect: Rect) -> Self {
        unsafe { fz_irect_from_rect(rect.into()) }.into()
    }
}

/// A rectangle represented by two diagonally opposite corners at arbitrary coordinates
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl Rect {
    pub const INF: Self = Rect {
        x0: FZ_MIN_INF_RECT as f32,
        y0: FZ_MIN_INF_RECT as f32,
        x1: FZ_MAX_INF_RECT as f32,
        y1: FZ_MAX_INF_RECT as f32,
    };

    pub const fn new(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        Self { x0, y0, x1, y1 }
    }

    pub fn is_empty(&self) -> bool {
        self.x0 >= self.x1 || self.y0 >= self.y1
    }

    pub fn is_valid(&self) -> bool {
        self.x0 <= self.x1 && self.y0 <= self.y1
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        if self.is_empty() {
            false
        } else {
            x >= self.x0 && x < self.x1 && y >= self.y0 && y < self.y1
        }
    }

    pub fn width(&self) -> f32 {
        if self.is_empty() {
            0.0
        } else {
            self.x1 - self.x0
        }
    }

    pub fn height(&self) -> f32 {
        if self.is_empty() {
            0.0
        } else {
            self.y1 - self.y0
        }
    }

    pub fn origin(&self) -> Point {
        Point::new(self.x0, self.y0)
    }

    pub fn size(&self) -> Size {
        Size::new(self.width(), self.height())
    }

    pub fn r#union(&self, other: &Self) -> Self {
        unsafe { fz_union_rect((*self).into(), (*other).into()) }.into()
    }

    pub fn adjust_for_stroke(&self, stroke: &StrokeState, ctm: &Matrix) -> Result<Self, Error> {
        let r = (*self).into();
        unsafe {
            ffi_try!(mupdf_adjust_rect_for_stroke(
                context(),
                r,
                stroke.inner,
                ctm.into()
            ))
        }
        .map(fz_rect::into)
    }

    pub fn transform(&self, matrix: &Matrix) -> Self {
        unsafe { fz_transform_rect((*self).into(), matrix.into()) }.into()
    }

    pub fn round(&self) -> IRect {
        unsafe { fz_round_rect((*self).into()) }.into()
    }

    pub fn intersect(&self, rect: &Self) -> Self {
        unsafe { fz_intersect_rect((*self).into(), (*rect).into()) }.into()
    }
}

impl fmt::Display for Rect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{} {} {} {}]", self.x0, self.y0, self.x1, self.y1)
    }
}

impl From<IRect> for Rect {
    fn from(it: IRect) -> Rect {
        Rect {
            x0: it.x0 as f32,
            y0: it.y0 as f32,
            x1: it.x1 as f32,
            y1: it.y1 as f32,
        }
    }
}

impl From<Quad> for Rect {
    fn from(q: Quad) -> Rect {
        Rect {
            x0: q.ul.x.min(q.ur.x).min(q.ll.x).min(q.lr.x),
            y0: q.ul.y.min(q.ur.y).min(q.ll.y).min(q.lr.y),
            x1: q.ul.x.max(q.ur.x).max(q.ll.x).max(q.lr.x),
            y1: q.ul.y.max(q.ur.y).max(q.ll.y).max(q.lr.y),
        }
    }
}

impl From<fz_rect> for Rect {
    fn from(r: fz_rect) -> Rect {
        Rect {
            x0: r.x0,
            y0: r.y0,
            x1: r.x1,
            y1: r.y1,
        }
    }
}

impl From<Rect> for fz_rect {
    fn from(r: Rect) -> Self {
        fz_rect {
            x0: r.x0,
            y0: r.y0,
            x1: r.x1,
            y1: r.y1,
        }
    }
}

use std::fmt;

use mupdf_sys::*;

use crate::pdf::PdfObject;
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
            other
        } else if !other.is_valid() {
            *self
        } else {
            IRect {
                x0: self.x0.min(other.x0),
                y0: self.y0.min(other.y0),
                x1: self.x1.max(other.x1),
                y1: self.y1.max(other.y1),
            }
        }
    }

    pub fn intersect(&self, rect: &Self) -> Self {
        unsafe { fz_intersect_irect((*self).into(), (*rect).into()) }.into()
    }

    pub fn translate(&self, xoff: i32, yoff: i32) -> Self {
        unsafe { fz_translate_irect((*self).into(), xoff, yoff) }.into()
    }
}

impl fmt::Display for IRect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{} {} {} {}]", self.x0, self.y0, self.x1, self.y1)
    }
}

impl From<fz_irect> for IRect {
    fn from(r: fz_irect) -> IRect {
        let fz_irect { x0, y0, x1, y1 } = r;
        IRect { x0, y0, x1, y1 }
    }
}

impl From<IRect> for fz_irect {
    fn from(val: IRect) -> Self {
        let IRect { x0, y0, x1, y1 } = val;
        fz_irect { x0, y0, x1, y1 }
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

    pub fn translate(&self, xoff: f32, yoff: f32) -> Self {
        unsafe { fz_translate_rect((*self).into(), xoff, yoff) }.into()
    }

    pub fn encode_into(self, array: &mut PdfObject) -> Result<(), Error> {
        array.array_push(PdfObject::new_real(self.x0)?)?;
        array.array_push(PdfObject::new_real(self.y0)?)?;
        array.array_push(PdfObject::new_real(self.x1)?)?;
        array.array_push(PdfObject::new_real(self.y1)?)
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
        let Quad { ul, ur, ll, lr } = q;
        let x0 = ul.x.min(ur.x).min(ll.x).min(lr.x);
        let y0 = ul.y.min(ur.y).min(ll.y).min(lr.y);
        let x1 = ul.x.max(ur.x).max(ll.x).max(lr.x);
        let y1 = ul.y.max(ur.y).max(ll.y).max(lr.y);
        Rect { x0, y0, x1, y1 }
    }
}

impl From<fz_rect> for Rect {
    fn from(r: fz_rect) -> Rect {
        let fz_rect { x0, y0, x1, y1 } = r;
        Rect { x0, y0, x1, y1 }
    }
}

impl From<Rect> for fz_rect {
    fn from(val: Rect) -> Self {
        let Rect { x0, y0, x1, y1 } = val;
        fz_rect { x0, y0, x1, y1 }
    }
}

use std::fmt;

use mupdf_sys::*;

use crate::{context, Error, Matrix, Point, Quad, Size, StrokeState};

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
        x0: 1,
        y0: 1,
        x1: -1,
        y1: -1,
    };

    pub const fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        Self { x0, y0, x1, y1 }
    }

    pub fn is_empty(&self) -> bool {
        self.x0 == self.x1 || self.y0 == self.y1
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        if self.is_empty() {
            return false;
        }
        x >= self.x0 && x < self.x1 && y >= self.y0 && y < self.y1
    }

    pub fn width(&self) -> i32 {
        self.x1 - self.x0
    }

    pub fn height(&self) -> i32 {
        self.y1 - self.y0
    }

    pub fn origin(&self) -> Point {
        Point::new(self.x0 as f32, self.y0 as f32)
    }

    pub fn size(&self) -> Size {
        Size::new((self.x1 - self.x0) as f32, (self.y1 - self.y0) as f32)
    }

    pub fn r#union(&mut self, other: IRect) -> &mut Self {
        let IRect { x0, y0, x1, y1 } = other;
        if self.is_empty() {
            self.x0 = x0;
            self.y0 = y0;
            self.x1 = x1;
            self.y1 = y1;
        } else {
            if x0 < self.x0 {
                self.x0 = x0;
            }
            if y0 < self.y0 {
                self.y0 = y0;
            }
            if x1 > self.x1 {
                self.x1 = x1;
            }
            if y1 > self.y1 {
                self.y1 = y1;
            }
        }
        self
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

impl Into<fz_irect> for IRect {
    fn into(self) -> fz_irect {
        let IRect { x0, y0, x1, y1 } = self;
        fz_irect { x0, y0, x1, y1 }
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
        x0: 1.0,
        y0: 1.0,
        x1: -1.0,
        y1: -1.0,
    };

    pub const fn new(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        Self { x0, y0, x1, y1 }
    }

    pub fn is_empty(&self) -> bool {
        self.x0 == self.x1 || self.y0 == self.y1
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        if self.is_empty() {
            return false;
        }
        x >= self.x0 && x < self.x1 && y >= self.y0 && y < self.y1
    }

    pub fn width(&self) -> f32 {
        self.x1 - self.x0
    }

    pub fn height(&self) -> f32 {
        self.y1 - self.y0
    }

    pub fn origin(&self) -> Point {
        Point::new(self.x0, self.y0)
    }

    pub fn size(&self) -> Size {
        Size::new(self.x1 - self.x0, self.y1 - self.y0)
    }

    pub fn r#union(&mut self, other: Rect) -> &mut Self {
        let Rect { x0, y0, x1, y1 } = other;
        if self.is_empty() {
            self.x0 = x0;
            self.y0 = y0;
            self.x1 = x1;
            self.y1 = y1;
        } else {
            if x0 < self.x0 {
                self.x0 = x0;
            }
            if y0 < self.y0 {
                self.y0 = y0;
            }
            if x1 > self.x1 {
                self.x1 = x1;
            }
            if y1 > self.y1 {
                self.y1 = y1;
            }
        }
        self
    }

    pub fn adjust_for_stroke(&self, stroke: &StrokeState, ctm: &Matrix) -> Result<Self, Error> {
        let r = (*self).into();
        let new_rect = unsafe {
            ffi_try!(mupdf_adjust_rect_for_stroke(
                context(),
                r,
                stroke.inner,
                ctm.into()
            ))
        };
        Ok(new_rect.into())
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

impl Into<fz_rect> for Rect {
    fn into(self) -> fz_rect {
        let Rect { x0, y0, x1, y1 } = self;
        fz_rect { x0, y0, x1, y1 }
    }
}

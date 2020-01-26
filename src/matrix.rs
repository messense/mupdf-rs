use std::f32::consts::PI;

use mupdf_sys::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Matrix {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl Matrix {
    pub const IDENTITY: Matrix = Matrix {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    pub fn new(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) -> Self {
        Self { a, b, c, d, e, f }
    }

    pub fn new_scale(x: f32, y: f32) -> Self {
        Self::new(x, 0.0, 0.0, y, 0.0, 0.0)
    }

    pub fn new_translate(x: f32, y: f32) -> Self {
        Self::new(1.0, 0.0, 0.0, 1.0, x, y)
    }

    pub fn new_rotate(degrees: f32) -> Self {
        let mut degrees = degrees;
        while degrees < 0.0 {
            degrees += 360.0;
        }
        while degrees >= 360.0 {
            degrees -= 360.0
        }
        let (sin, cos) = if (0.0 - degrees).abs() < 0.0001 {
            (0.0, 1.0)
        } else if (90.0 - degrees).abs() < 0.0001 {
            (1.0, 0.0)
        } else if (180.0 - degrees).abs() < 0.0001 {
            (0.0, -1.0)
        } else if (270.0 - degrees).abs() < 0.0001 {
            (-1.0, 0.0)
        } else {
            ((degrees * PI / 180.0).sin(), (degrees * PI / 180.0).cos())
        };
        Self::new(cos, sin, -sin, cos, 0.0, 0.0)
    }

    pub fn concat(&mut self, m: Matrix) -> &mut Self {
        let a = self.a * m.a + self.b * m.c;
        let b = self.a * m.b + self.b * m.d;
        let c = self.c * m.a + self.d * m.c;
        let d = self.c * m.b + self.d * m.d;
        let e = self.e * m.a + self.f * m.c + m.e;
        let f = self.e * m.b + self.f * m.d + m.f;

        self.a = a;
        self.b = b;
        self.c = c;
        self.d = d;
        self.e = e;
        self.f = f;
        self
    }

    pub fn scale(&mut self, sx: f32, sy: f32) -> &mut Self {
        self.a *= sx;
        self.b *= sx;
        self.c *= sy;
        self.d *= sy;
        self
    }

    pub fn rotate(&mut self, degrees: f32) -> &mut Self {
        let mut degrees = degrees;
        while degrees < 0.0 {
            degrees += 360.0;
        }
        while degrees >= 360.0 {
            degrees -= 360.0
        }
        if (0.0 - degrees).abs() < 0.0001 {
            // Nothing to do
        } else if (90.0 - degrees).abs() < 0.0001 {
            let save_a = self.a;
            let save_b = self.b;
            self.a = self.c;
            self.b = self.d;
            self.c = -save_a;
            self.d = -save_b;
        } else if (180.0 - degrees).abs() < 0.0001 {
            self.a = -self.a;
            self.b = -self.b;
            self.c = -self.c;
            self.d = -self.d;
        } else if (270.0 - degrees).abs() < 0.0001 {
            let save_a = self.a;
            let save_b = self.b;
            self.a = self.c;
            self.b = self.d;
            self.c = save_a;
            self.d = save_b;
        } else {
            let sin = (degrees * PI / 180.0).sin();
            let cos = (degrees * PI / 180.0).cos();
            let save_a = self.a;
            let save_b = self.b;
            self.a = cos * save_a + sin * self.c;
            self.b = cos * save_b + sin * self.d;
            self.c = -sin * save_a + cos * self.c;
            self.d = -sin * save_b + cos * self.d;
        }
        self
    }
}

impl Default for Matrix {
    fn default() -> Self {
        Matrix {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }
}

impl Into<fz_matrix> for &Matrix {
    fn into(self) -> fz_matrix {
        let Matrix { a, b, c, d, e, f } = *self;
        fz_matrix { a, b, c, d, e, f }
    }
}

impl Into<fz_matrix> for Matrix {
    fn into(self) -> fz_matrix {
        let Matrix { a, b, c, d, e, f } = self;
        fz_matrix { a, b, c, d, e, f }
    }
}

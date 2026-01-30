use std::f32::consts::PI;

use mupdf_sys::*;

/// A row-major 3x3 matrix used for representing transformations of coordinates
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
        let degrees = degrees.rem_euclid(360.0);
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
            self.a = -self.c;
            self.b = -self.d;
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

    pub fn pre_translate(&mut self, x: f32, y: f32) -> &mut Self {
        self.e += x * self.a + y * self.c;
        self.f += x * self.b + y * self.d;
        self
    }

    pub fn pre_shear(&mut self, h: f32, v: f32) -> &mut Self {
        let a = self.a;
        let b = self.b;
        self.a += v * self.c;
        self.b += v * self.d;
        self.c += h * a;
        self.d += h * b;
        self
    }

    pub fn expansion(&self) -> f32 {
        (self.a * self.d - self.b * self.c).abs().sqrt()
    }

    /// Inverts this matrix.
    ///
    /// Returns the inverse matrix, or `None` if the matrix is singular
    /// (determinant is zero or near-zero).
    ///
    /// # MuPDF parity
    ///
    /// Ported from [`fz_invert_matrix`] in MuPDF ([source/fitz/geometry.c]).
    /// MuPDF returns a zero matrix `(0,0,0,0,0,0)` for singular matrices,
    /// this method returns `None` instead, letting the caller decide how to handle it.
    ///
    /// [source/fitz/geometry.c]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/fitz/geometry.c#L257
    #[inline]
    pub fn invert(&self) -> Option<Self> {
        // Use double precision for intermediate calculations (matching MuPDF)
        let sa = self.a as f64;
        let sb = self.b as f64;
        let sc = self.c as f64;
        let sd = self.d as f64;

        let det = sa * sd - sb * sc;

        // Check for singular matrix
        if det.abs() > f64::EPSILON {
            let det = 1.0 / det;
            let da = sd * det;
            let db = -sb * det;
            let dc = -sc * det;
            let dd = sa * det;
            let de = -(self.e as f64) * da - (self.f as f64) * dc;
            let df = -(self.e as f64) * db - (self.f as f64) * dd;

            return Some(Self {
                a: da as f32,
                b: db as f32,
                c: dc as f32,
                d: dd as f32,
                e: de as f32,
                f: df as f32,
            });
        }

        None // MuPDF returns zeros here
    }

    // Helper function, similar to `fz_transform_point_xy` from C. Performs bare math without checking for NaN.
    // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/fitz/geometry.c#L344
    #[inline(always)]
    pub fn transform_xy(&self, x: f32, y: f32) -> (f32, f32) {
        (
            x * self.a + y * self.c + self.e,
            x * self.b + y * self.d + self.f,
        )
    }
}

impl Default for Matrix {
    fn default() -> Self {
        Matrix::IDENTITY
    }
}

impl From<fz_matrix> for Matrix {
    fn from(m: fz_matrix) -> Self {
        let fz_matrix { a, b, c, d, e, f } = m;
        Self { a, b, c, d, e, f }
    }
}

impl From<&Matrix> for fz_matrix {
    fn from(val: &Matrix) -> Self {
        let Matrix { a, b, c, d, e, f } = *val;
        fz_matrix { a, b, c, d, e, f }
    }
}

impl From<Matrix> for fz_matrix {
    fn from(val: Matrix) -> Self {
        let Matrix { a, b, c, d, e, f } = val;
        fz_matrix { a, b, c, d, e, f }
    }
}

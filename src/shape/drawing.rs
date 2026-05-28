use super::operators::{format_g, util_hor_matrix};
use super::Shape;
use crate::{Error, Point, Quad, Rect};

const CURVE_KAPPA: f32 = 0.552_284_8;
const SQUIGGLE_CONTROL_SCALE: f32 = 2.414_213_7;

/// Radius specification for [`Shape::draw_rect_with_radius`].
///
/// ```
/// use mupdf::{pdf::PdfDocument, shape::{FinishOptions, RectRadius, Shape}, Rect, Size};
///
/// # fn main() -> Result<(), mupdf::Error> {
/// let mut doc = PdfDocument::new();
/// let mut page = doc.new_page(Size::A4)?;
/// let mut shape = Shape::new(&mut page)?;
/// shape
///     .draw_rect_with_radius(&Rect::new(72.0, 72.0, 180.0, 132.0), RectRadius::absolute(8.0))?
///     .finish(&FinishOptions::default())?
///     .commit(&mut doc, true)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RectRadius {
    /// A uniform radius in user-space units, clamped to half the shorter side.
    Absolute(f32),
    /// Per-axis radii in user-space units, clamped to half their corresponding sides.
    AbsoluteXY(f32, f32),
    /// Per-axis fractions of the rectangle width and height.
    Fractional(f32, f32),
}

impl RectRadius {
    /// Creates a uniform absolute radius.
    #[inline]
    pub const fn absolute(radius: f32) -> Self {
        Self::Absolute(radius)
    }

    /// Creates per-axis absolute radii.
    #[inline]
    pub const fn absolute_xy(rx: f32, ry: f32) -> Self {
        Self::AbsoluteXY(rx, ry)
    }

    /// Creates per-axis fractional radii.
    #[inline]
    pub const fn fractional(rx: f32, ry: f32) -> Self {
        Self::Fractional(rx, ry)
    }

    fn resolve(self, rect: Rect) -> (f32, f32) {
        let width = (rect.x1 - rect.x0).abs();
        let height = (rect.y1 - rect.y0).abs();
        let half_width = width / 2.0;
        let half_height = height / 2.0;
        let half_shortest = half_width.min(half_height);

        match self {
            Self::Absolute(radius) => {
                let radius = finite_abs(radius).min(half_shortest);
                (radius, radius)
            }
            Self::AbsoluteXY(rx, ry) => (
                finite_abs(rx).min(half_width),
                finite_abs(ry).min(half_height),
            ),
            Self::Fractional(rx, ry) => {
                let rx = finite_abs(rx);
                let ry = finite_abs(ry);
                let mut resolved_x = rx * width;
                let mut resolved_y = ry * height;
                if rx > 0.5 || ry > 0.5 {
                    resolved_x = resolved_x.min(half_shortest);
                    resolved_y = resolved_y.min(half_shortest);
                } else {
                    resolved_x = resolved_x.min(half_width);
                    resolved_y = resolved_y.min(half_height);
                }
                (resolved_x, resolved_y)
            }
        }
    }
}

impl From<f32> for RectRadius {
    fn from(radius: f32) -> Self {
        Self::Absolute(radius)
    }
}

impl From<(f32, f32)> for RectRadius {
    fn from((rx, ry): (f32, f32)) -> Self {
        if finite_abs(rx) <= 1.0 && finite_abs(ry) <= 1.0 {
            Self::Fractional(rx, ry)
        } else {
            Self::AbsoluteXY(rx, ry)
        }
    }
}

impl Shape<'_> {
    /// Draws a straight line from `p1` to `p2`.
    ///
    /// Equivalent of PyMuPDF `Shape.draw_line`.
    ///
    /// ```
    /// use mupdf::{pdf::PdfDocument, shape::{FinishOptions, Shape}, Point, Size};
    ///
    /// # fn main() -> Result<(), mupdf::Error> {
    /// let mut doc = PdfDocument::new();
    /// let mut page = doc.new_page(Size::A4)?;
    /// let mut shape = Shape::new(&mut page)?;
    /// shape
    ///     .draw_line(Point::new(72.0, 72.0), Point::new(180.0, 72.0))?
    ///     .finish(&FinishOptions::default())?
    ///     .commit(&mut doc, true)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn draw_line(&mut self, p1: Point, p2: Point) -> Result<&mut Self, Error> {
        self.move_to_if_needed(p1);
        self.line_to(p2);
        Ok(self)
    }

    /// Draws connected line segments through `points`.
    ///
    /// Inputs with fewer than two points are treated as a no-op: no operators
    /// are emitted and path bookkeeping is left unchanged.
    ///
    /// Equivalent of PyMuPDF `Shape.draw_polyline`.
    ///
    /// ```
    /// use mupdf::{pdf::PdfDocument, shape::{FinishOptions, Shape}, Point, Size};
    ///
    /// # fn main() -> Result<(), mupdf::Error> {
    /// let mut doc = PdfDocument::new();
    /// let mut page = doc.new_page(Size::A4)?;
    /// let mut shape = Shape::new(&mut page)?;
    /// shape
    ///     .draw_polyline(&[
    ///         Point::new(72.0, 72.0),
    ///         Point::new(120.0, 120.0),
    ///         Point::new(168.0, 72.0),
    ///     ])?
    ///     .finish(&FinishOptions::default())?
    ///     .commit(&mut doc, true)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn draw_polyline(&mut self, points: &[Point]) -> Result<&mut Self, Error> {
        let Some((first, rest)) = points.split_first() else {
            return Ok(self);
        };
        if rest.is_empty() {
            return Ok(self);
        }

        if self.needs_move_to(first) {
            let transformed = self.transform_point(*first);
            self.draw_cont.push_str(&format!(
                "{} {} m\n",
                format_g(transformed.x),
                format_g(transformed.y)
            ));
            self.set_last_point(*first);
        }
        self.update_rect(first);
        for point in rest {
            self.line_to(*point);
        }
        Ok(self)
    }

    /// Draws an axis-aligned rectangle using the PDF `re` operator.
    ///
    /// Equivalent of PyMuPDF `Shape.draw_rect` for the non-rounded case.
    ///
    /// ```
    /// use mupdf::{pdf::PdfDocument, shape::{FinishOptions, Shape}, Rect, Size};
    ///
    /// # fn main() -> Result<(), mupdf::Error> {
    /// let mut doc = PdfDocument::new();
    /// let mut page = doc.new_page(Size::A4)?;
    /// let mut shape = Shape::new(&mut page)?;
    /// shape
    ///     .draw_rect(&Rect::new(72.0, 72.0, 180.0, 132.0))?
    ///     .finish(&FinishOptions::default())?
    ///     .commit(&mut doc, true)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn draw_rect(&mut self, rect: &Rect) -> Result<&mut Self, Error> {
        let top_left = rect.tl();
        let bottom_right = rect.br();
        let transformed_top_left = self.transform_point(top_left);
        let transformed_bottom_right = self.transform_point(bottom_right);
        let x0 = transformed_top_left.x.min(transformed_bottom_right.x);
        let y0 = transformed_top_left.y.min(transformed_bottom_right.y);
        let x1 = transformed_top_left.x.max(transformed_bottom_right.x);
        let y1 = transformed_top_left.y.max(transformed_bottom_right.y);

        self.draw_cont.push_str(&format!(
            "{} {} {} {} re\n",
            format_g(x0),
            format_g(y0),
            format_g(x1 - x0),
            format_g(y1 - y0)
        ));
        self.update_rect_with_rect(*rect);
        self.set_last_point(rect.bl());
        Ok(self)
    }

    /// Draws an axis-aligned rectangle with rounded corners.
    ///
    /// The path is emitted as line segments plus cubic Bézier curves using the same
    /// κ approximation as [`Shape::draw_curve`]. Use [`Shape::draw_rect`] to keep
    /// the compact PDF `re` operator for non-rounded rectangles.
    ///
    /// ```
    /// use mupdf::{pdf::PdfDocument, shape::{FinishOptions, Shape}, Rect, Size};
    ///
    /// # fn main() -> Result<(), mupdf::Error> {
    /// let mut doc = PdfDocument::new();
    /// let mut page = doc.new_page(Size::A4)?;
    /// let mut shape = Shape::new(&mut page)?;
    /// shape
    ///     .draw_rect_with_radius(&Rect::new(72.0, 72.0, 180.0, 132.0), 10.0)?
    ///     .finish(&FinishOptions::default())?
    ///     .commit(&mut doc, true)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn draw_rect_with_radius<R>(&mut self, rect: &Rect, radius: R) -> Result<&mut Self, Error>
    where
        R: Into<RectRadius>,
    {
        let rect = normalize_drawing_rect(*rect);
        let (rx, ry) = radius.into().resolve(rect);
        if rx <= f32::EPSILON || ry <= f32::EPSILON {
            return self.draw_rect(&rect);
        }

        let tl = rect.tl();
        let tr = rect.tr();
        let bl = rect.bl();
        let br = rect.br();
        let start = Point::new(tl.x, tl.y + ry);

        self.draw_line(start, Point::new(bl.x, bl.y - ry))?;
        self.draw_curve(Point::new(bl.x, bl.y - ry), bl, Point::new(bl.x + rx, bl.y))?;
        self.draw_line(Point::new(bl.x + rx, bl.y), Point::new(br.x - rx, br.y))?;
        self.draw_curve(Point::new(br.x - rx, br.y), br, Point::new(br.x, br.y - ry))?;
        self.draw_line(Point::new(br.x, br.y - ry), Point::new(tr.x, tr.y + ry))?;
        self.draw_curve(Point::new(tr.x, tr.y + ry), tr, Point::new(tr.x - rx, tr.y))?;
        self.draw_line(Point::new(tr.x - rx, tr.y), Point::new(tl.x + rx, tl.y))?;
        self.draw_curve(Point::new(tl.x + rx, tl.y), tl, start)?;
        self.update_rect_with_rect(rect);
        self.set_last_point(start);
        Ok(self)
    }

    /// Draws a cubic Bézier curve from `p1` to `p4` using controls `p2` and `p3`.
    ///
    /// Equivalent of PyMuPDF `Shape.draw_bezier`.
    ///
    /// ```
    /// use mupdf::{pdf::PdfDocument, shape::{FinishOptions, Shape}, Point, Size};
    ///
    /// # fn main() -> Result<(), mupdf::Error> {
    /// let mut doc = PdfDocument::new();
    /// let mut page = doc.new_page(Size::A4)?;
    /// let mut shape = Shape::new(&mut page)?;
    /// shape
    ///     .draw_bezier(
    ///         Point::new(72.0, 120.0),
    ///         Point::new(96.0, 72.0),
    ///         Point::new(156.0, 168.0),
    ///         Point::new(180.0, 120.0),
    ///     )?
    ///     .finish(&FinishOptions::default())?
    ///     .commit(&mut doc, true)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn draw_bezier(
        &mut self,
        p1: Point,
        p2: Point,
        p3: Point,
        p4: Point,
    ) -> Result<&mut Self, Error> {
        self.move_to_if_needed(p1);
        let p2_transformed = self.transform_point(p2);
        let p3_transformed = self.transform_point(p3);
        let p4_transformed = self.transform_point(p4);
        self.draw_cont.push_str(&format!(
            "{} {} {} {} {} {} c\n",
            format_g(p2_transformed.x),
            format_g(p2_transformed.y),
            format_g(p3_transformed.x),
            format_g(p3_transformed.y),
            format_g(p4_transformed.x),
            format_g(p4_transformed.y)
        ));
        self.update_rect(&p1);
        self.update_rect(&p2);
        self.update_rect(&p3);
        self.update_rect(&p4);
        self.set_last_point(p4);
        Ok(self)
    }

    /// Draws a single-control curve from `p1` to `p3` through control point `p2`.
    ///
    /// The curve is converted to a cubic Bézier using κ = 0.55228474983.
    /// Equivalent of PyMuPDF `Shape.draw_curve`.
    ///
    /// ```
    /// use mupdf::{pdf::PdfDocument, shape::{FinishOptions, Shape}, Point, Size};
    ///
    /// # fn main() -> Result<(), mupdf::Error> {
    /// let mut doc = PdfDocument::new();
    /// let mut page = doc.new_page(Size::A4)?;
    /// let mut shape = Shape::new(&mut page)?;
    /// shape
    ///     .draw_curve(Point::new(72.0, 120.0), Point::new(126.0, 72.0), Point::new(180.0, 120.0))?
    ///     .finish(&FinishOptions::default())?
    ///     .commit(&mut doc, true)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn draw_curve(&mut self, p1: Point, p2: Point, p3: Point) -> Result<&mut Self, Error> {
        let k1 = point_between(p1, p2, CURVE_KAPPA);
        let k2 = point_between(p3, p2, CURVE_KAPPA);
        self.draw_bezier(p1, k1, k2, p3)
    }

    /// Draws a circular arc, optionally connecting both arc ends to `center`.
    ///
    /// `point` is the arc start point. Positive `beta` values follow PyMuPDF's clockwise
    /// convention. When `full_sector` is false this emits only the arc path.
    ///
    /// Equivalent of PyMuPDF `Shape.draw_sector`.
    ///
    /// ```
    /// use mupdf::{pdf::PdfDocument, shape::{FinishOptions, Shape}, Point, Size};
    ///
    /// # fn main() -> Result<(), mupdf::Error> {
    /// let mut doc = PdfDocument::new();
    /// let mut page = doc.new_page(Size::A4)?;
    /// let mut shape = Shape::new(&mut page)?;
    /// shape
    ///     .draw_sector(Point::new(126.0, 126.0), Point::new(180.0, 126.0), 120.0, true)?
    ///     .finish(&FinishOptions::default())?
    ///     .commit(&mut doc, true)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn draw_sector(
        &mut self,
        center: Point,
        point: Point,
        beta: f32,
        full_sector: bool,
    ) -> Result<&mut Self, Error> {
        let radius = (point - center).length();
        if !radius.is_finite() || radius <= f32::EPSILON {
            return Err(Error::InvalidArgument(
                "radius must be a positive finite value".to_owned(),
            ));
        }
        if !beta.is_finite() {
            return Err(Error::InvalidArgument(
                "beta must be a finite value".to_owned(),
            ));
        }

        self.move_to_if_needed(point);

        let mut sweep = f64::from(-beta).to_radians();
        while sweep.abs() > std::f64::consts::TAU {
            sweep -= sweep.signum() * std::f64::consts::TAU;
        }

        let mut current = point;
        let mut start_angle = horizontal_angle(center, point);
        let mut remaining = sweep;
        while remaining.abs() > std::f64::consts::FRAC_PI_2 + 1e-12 {
            let delta = remaining.signum() * std::f64::consts::FRAC_PI_2;
            current = self.arc_segment(center, radius, current, start_angle, delta);
            start_angle += delta;
            remaining -= delta;
        }
        if remaining.abs() > 1e-6 {
            current = self.arc_segment(center, radius, current, start_angle, remaining);
        }

        if full_sector {
            self.move_to(point);
            self.line_to(center);
            self.line_to(current);
        }

        Ok(self)
    }

    /// Draws a circle with `center` and `radius`.
    ///
    /// Equivalent of PyMuPDF `Shape.draw_circle`, implemented as a full `draw_sector` arc.
    ///
    /// ```
    /// use mupdf::{pdf::PdfDocument, shape::{FinishOptions, Shape}, Point, Size};
    ///
    /// # fn main() -> Result<(), mupdf::Error> {
    /// let mut doc = PdfDocument::new();
    /// let mut page = doc.new_page(Size::A4)?;
    /// let mut shape = Shape::new(&mut page)?;
    /// shape
    ///     .draw_circle(Point::new(126.0, 126.0), 54.0)?
    ///     .finish(&FinishOptions::default())?
    ///     .commit(&mut doc, true)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn draw_circle(&mut self, center: Point, radius: f32) -> Result<&mut Self, Error> {
        if !radius.is_finite() || radius <= f32::EPSILON {
            return Err(Error::InvalidArgument(
                "radius must be a positive finite value".to_owned(),
            ));
        }

        let point = Point::new(center.x - radius, center.y);
        self.draw_sector(center, point, 360.0, false)
    }

    /// Draws an oval inside a rectangle or quadrilateral.
    ///
    /// Equivalent of PyMuPDF `Shape.draw_oval`.
    ///
    /// ```
    /// use mupdf::{pdf::PdfDocument, shape::{FinishOptions, Shape}, Rect, Size};
    ///
    /// # fn main() -> Result<(), mupdf::Error> {
    /// let mut doc = PdfDocument::new();
    /// let mut page = doc.new_page(Size::A4)?;
    /// let mut shape = Shape::new(&mut page)?;
    /// shape
    ///     .draw_oval(Rect::new(72.0, 72.0, 180.0, 132.0))?
    ///     .finish(&FinishOptions::default())?
    ///     .commit(&mut doc, true)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn draw_oval(&mut self, tetra: impl Into<Quad>) -> Result<&mut Self, Error> {
        let quad = tetra.into();
        let middle_top = point_between(quad.ul, quad.ur, 0.5);
        let middle_right = point_between(quad.ur, quad.lr, 0.5);
        let middle_bottom = point_between(quad.ll, quad.lr, 0.5);
        let middle_left = point_between(quad.ul, quad.ll, 0.5);

        self.move_to_if_needed(middle_left);
        self.draw_curve(middle_left, quad.ll, middle_bottom)?;
        self.draw_curve(middle_bottom, quad.lr, middle_right)?;
        self.draw_curve(middle_right, quad.ur, middle_top)?;
        self.draw_curve(middle_top, quad.ul, middle_left)?;
        self.update_rect_with_rect(Rect::from(quad));
        self.set_last_point(middle_left);
        Ok(self)
    }

    /// Draws a quadrilateral outline.
    ///
    /// Equivalent of PyMuPDF `Shape.draw_quad`.
    ///
    /// ```
    /// use mupdf::{pdf::PdfDocument, shape::{FinishOptions, Shape}, Point, Quad, Size};
    ///
    /// # fn main() -> Result<(), mupdf::Error> {
    /// let mut doc = PdfDocument::new();
    /// let mut page = doc.new_page(Size::A4)?;
    /// let mut shape = Shape::new(&mut page)?;
    /// shape
    ///     .draw_quad(Quad::new(
    ///         Point::new(72.0, 72.0),
    ///         Point::new(180.0, 90.0),
    ///         Point::new(84.0, 150.0),
    ///         Point::new(192.0, 168.0),
    ///     ))?
    ///     .finish(&FinishOptions::default())?
    ///     .commit(&mut doc, true)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn draw_quad(&mut self, quad: Quad) -> Result<&mut Self, Error> {
        self.draw_polyline(&[quad.ul, quad.ll, quad.lr, quad.ur, quad.ul])
    }

    /// Draws a zigzag line from `p1` to `p2`.
    ///
    /// A `breadth` of zero degenerates to [`Shape::draw_line`] byte-for-byte.
    /// Equivalent of PyMuPDF `Shape.draw_zigzag`.
    ///
    /// ```
    /// use mupdf::{pdf::PdfDocument, shape::{FinishOptions, Shape}, Point, Size};
    ///
    /// # fn main() -> Result<(), mupdf::Error> {
    /// let mut doc = PdfDocument::new();
    /// let mut page = doc.new_page(Size::A4)?;
    /// let mut shape = Shape::new(&mut page)?;
    /// shape
    ///     .draw_zigzag(Point::new(72.0, 120.0), Point::new(220.0, 120.0), 6.0)?
    ///     .finish(&FinishOptions::default())?
    ///     .commit(&mut doc, true)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn draw_zigzag(&mut self, p1: Point, p2: Point, breadth: f32) -> Result<&mut Self, Error> {
        if breadth == 0.0 {
            return self.draw_line(p1, p2);
        }

        let breadth = checked_breadth(breadth)?;
        let distance = (p2 - p1).length();
        let phase_count = wave_phase_count(distance, breadth)?;
        let adjusted_breadth = distance / phase_count as f32;
        let inverse = util_hor_matrix(p1, p2)
            .invert()
            .ok_or(Error::NonInvertibleMatrix)?;

        let mut points = Vec::with_capacity(phase_count / 2 + 2);
        points.push(p1);
        for phase in 1..phase_count {
            let y = match phase % 4 {
                1 => -adjusted_breadth,
                3 => adjusted_breadth,
                _ => continue,
            };
            let x = phase as f32 * adjusted_breadth;
            points.push(Point::new(x, y).mul_matrix(&inverse));
        }
        points.push(p2);

        self.draw_polyline(&points)
    }

    /// Draws a squiggly line from `p1` to `p2`.
    ///
    /// A `breadth` of zero degenerates to [`Shape::draw_line`] byte-for-byte.
    /// Equivalent of PyMuPDF `Shape.draw_squiggle`.
    ///
    /// ```
    /// use mupdf::{pdf::PdfDocument, shape::{FinishOptions, Shape}, Point, Size};
    ///
    /// # fn main() -> Result<(), mupdf::Error> {
    /// let mut doc = PdfDocument::new();
    /// let mut page = doc.new_page(Size::A4)?;
    /// let mut shape = Shape::new(&mut page)?;
    /// shape
    ///     .draw_squiggle(Point::new(72.0, 120.0), Point::new(220.0, 120.0), 6.0)?
    ///     .finish(&FinishOptions::default())?
    ///     .commit(&mut doc, true)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn draw_squiggle(
        &mut self,
        p1: Point,
        p2: Point,
        breadth: f32,
    ) -> Result<&mut Self, Error> {
        if breadth == 0.0 {
            return self.draw_line(p1, p2);
        }

        let breadth = checked_breadth(breadth)?;
        let distance = (p2 - p1).length();
        let phase_count = wave_phase_count(distance, breadth)?;
        let adjusted_breadth = distance / phase_count as f32;
        let inverse = util_hor_matrix(p1, p2)
            .invert()
            .ok_or(Error::NonInvertibleMatrix)?;

        let mut points = Vec::with_capacity(phase_count + 1);
        points.push(p1);
        for phase in 1..phase_count {
            let y = match phase % 4 {
                1 => -SQUIGGLE_CONTROL_SCALE * adjusted_breadth,
                3 => SQUIGGLE_CONTROL_SCALE * adjusted_breadth,
                _ => 0.0,
            };
            let x = phase as f32 * adjusted_breadth;
            points.push(Point::new(x, y).mul_matrix(&inverse));
        }
        points.push(p2);

        let mut index = 0;
        while index + 2 < points.len() {
            self.draw_curve(points[index], points[index + 1], points[index + 2])?;
            index += 2;
        }

        Ok(self)
    }

    fn move_to(&mut self, point: Point) {
        let transformed = self.transform_point(point);
        self.draw_cont.push_str(&format!(
            "{} {} m\n",
            format_g(transformed.x),
            format_g(transformed.y)
        ));
        self.update_rect(&point);
        self.set_last_point(point);
    }

    fn move_to_if_needed(&mut self, point: Point) {
        if self.needs_move_to(&point) {
            self.move_to(point);
        }
    }

    fn line_to(&mut self, point: Point) {
        let transformed = self.transform_point(point);
        self.draw_cont.push_str(&format!(
            "{} {} l\n",
            format_g(transformed.x),
            format_g(transformed.y)
        ));
        self.update_rect(&point);
        self.set_last_point(point);
    }

    fn transform_point(&self, point: Point) -> Point {
        point.mul_matrix(&self.ipctm)
    }

    fn arc_segment(
        &mut self,
        center: Point,
        radius: f32,
        start: Point,
        start_angle: f64,
        delta: f64,
    ) -> Point {
        let radius = f64::from(radius);
        let end_angle = start_angle + delta;
        let kappa = (4.0 / 3.0) * (delta / 4.0).tan();
        let end = point_on_circle(center, radius, end_angle);
        let start_tangent = tangent_at(start_angle);
        let end_tangent = tangent_at(end_angle);
        let cp1 = Point::new(
            clean_f64(f64::from(start.x) + start_tangent.0 * kappa * radius),
            clean_f64(f64::from(start.y) + start_tangent.1 * kappa * radius),
        );
        let cp2 = Point::new(
            clean_f64(f64::from(end.x) - end_tangent.0 * kappa * radius),
            clean_f64(f64::from(end.y) - end_tangent.1 * kappa * radius),
        );

        let cp1_transformed = self.transform_point(cp1);
        let cp2_transformed = self.transform_point(cp2);
        let end_transformed = self.transform_point(end);
        self.draw_cont.push_str(&format!(
            "{} {} {} {} {} {} c\n",
            format_g(cp1_transformed.x),
            format_g(cp1_transformed.y),
            format_g(cp2_transformed.x),
            format_g(cp2_transformed.y),
            format_g(end_transformed.x),
            format_g(end_transformed.y)
        ));
        self.update_rect(&start);
        self.update_rect(&cp1);
        self.update_rect(&cp2);
        self.update_rect(&end);
        self.set_last_point(end);
        end
    }
}

fn point_between(start: Point, control: Point, scale: f32) -> Point {
    Point::new(
        start.x + (control.x - start.x) * scale,
        start.y + (control.y - start.y) * scale,
    )
}

fn finite_abs(value: f32) -> f32 {
    if value.is_finite() {
        value.abs()
    } else {
        0.0
    }
}

fn checked_breadth(breadth: f32) -> Result<f32, Error> {
    if breadth.is_finite() && breadth > 0.0 {
        Ok(breadth)
    } else {
        Err(Error::InvalidArgument(
            "wave breadth must be a positive finite number, or zero for a straight line".to_owned(),
        ))
    }
}

fn wave_phase_count(distance: f32, breadth: f32) -> Result<usize, Error> {
    if !distance.is_finite() || distance <= f32::EPSILON {
        return Err(Error::InvalidArgument(
            "wave endpoints are too close for the requested breadth".to_owned(),
        ));
    }

    let count = 4 * round_half_even(f64::from(distance) / (4.0 * f64::from(breadth))) as usize;
    if count == 0 {
        Err(Error::InvalidArgument(
            "wave endpoints are too close for the requested breadth".to_owned(),
        ))
    } else if count < 4 {
        Err(Error::InvalidArgument(
            "wave endpoints are too close for the requested breadth".to_owned(),
        ))
    } else {
        Ok(count)
    }
}

fn round_half_even(value: f64) -> u64 {
    let floor = value.floor();
    let fraction = value - floor;
    if (fraction - 0.5).abs() <= 1e-12 {
        let floor_int = floor as u64;
        if floor_int % 2 == 0 {
            floor_int
        } else {
            floor_int + 1
        }
    } else {
        value.round() as u64
    }
}

fn normalize_drawing_rect(rect: Rect) -> Rect {
    Rect::new(
        rect.x0.min(rect.x1),
        rect.y0.min(rect.y1),
        rect.x0.max(rect.x1),
        rect.y0.max(rect.y1),
    )
}

fn horizontal_angle(center: Point, point: Point) -> f64 {
    f64::from(point.y - center.y).atan2(f64::from(point.x - center.x))
}

fn point_on_circle(center: Point, radius: f64, angle: f64) -> Point {
    Point::new(
        clean_f64(f64::from(center.x) + angle.cos() * radius),
        clean_f64(f64::from(center.y) + angle.sin() * radius),
    )
}

fn tangent_at(angle: f64) -> (f64, f64) {
    (-angle.sin(), angle.cos())
}

fn clean_f64(value: f64) -> f32 {
    if value.abs() < 1e-10 {
        0.0
    } else {
        value as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::PdfDocument;
    use crate::{Matrix, Size};

    fn shape_with_identity_ctm<'a>(page: &'a mut crate::pdf::PdfPage) -> Shape<'a> {
        let mut shape = Shape::new(page).unwrap();
        shape.ipctm = Matrix::IDENTITY;
        shape
    }

    #[test]
    fn draw_sector_full_circle_operators() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = shape_with_identity_ctm(&mut page);

        shape
            .draw_sector(
                Point::new(100.0, 100.0),
                Point::new(150.0, 100.0),
                360.0,
                false,
            )
            .unwrap();

        assert_eq!(
            shape.draw_cont(),
            concat!(
                "150 100 m\n",
                "150 72.3858 127.614 50 100 50 c\n",
                "72.3858 50 50 72.3858 50 100 c\n",
                "50 127.614 72.3858 150 100 150 c\n",
                "127.614 150 150 127.614 150 100 c\n",
            )
        );
        assert_eq!(shape.last_point(), Some(Point::new(150.0, 100.0)));
        assert_eq!(shape.draw_cont().matches(" c\n").count(), 4);
        assert!(!shape.draw_cont().contains(" l\n"));
    }

    #[test]
    fn draw_sector_partial_with_full_sector_closes_to_center() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = shape_with_identity_ctm(&mut page);

        shape
            .draw_sector(
                Point::new(100.0, 100.0),
                Point::new(150.0, 100.0),
                90.0,
                true,
            )
            .unwrap();

        assert_eq!(
            shape.draw_cont(),
            concat!(
                "150 100 m\n",
                "150 72.3858 127.614 50 100 50 c\n",
                "150 100 m\n",
                "100 100 l\n",
                "100 50 l\n",
            )
        );
        assert_eq!(shape.draw_cont().matches(" c\n").count(), 1);
        assert_eq!(shape.draw_cont().matches(" m\n").count(), 2);
        assert_eq!(shape.draw_cont().matches(" l\n").count(), 2);
    }

    #[test]
    fn draw_sector_beta_360_equals_draw_circle() {
        let center = Point::new(100.0, 100.0);
        let point = Point::new(50.0, 100.0);

        let mut doc = PdfDocument::new();
        let mut sector_page = doc.new_page(Size::A4).unwrap();
        let sector_content = {
            let mut shape = shape_with_identity_ctm(&mut sector_page);
            shape.draw_sector(center, point, 360.0, false).unwrap();
            shape.draw_cont().to_owned()
        };

        let mut circle_page = doc.new_page(Size::A4).unwrap();
        let circle_content = {
            let mut shape = shape_with_identity_ctm(&mut circle_page);
            shape
                .draw_circle(center, (point - center).length())
                .unwrap();
            shape.draw_cont().to_owned()
        };

        assert_eq!(sector_content, circle_content);
    }

    #[test]
    fn draw_circle_rejects_non_positive_radius_without_appending_content() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = shape_with_identity_ctm(&mut page);

        let result = shape.draw_circle(Point::new(100.0, 100.0), 0.0);

        assert!(result.is_err());
        assert!(shape.draw_cont().is_empty());
        assert_eq!(shape.last_point(), None);
        assert_eq!(shape.rect(), None);
    }

    #[test]
    fn draw_oval_from_rect_operators() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = shape_with_identity_ctm(&mut page);

        shape.draw_oval(Rect::new(10.0, 10.0, 110.0, 60.0)).unwrap();

        assert_eq!(
            shape.draw_cont(),
            concat!(
                "10 35 m\n",
                "10 48.8071 32.3858 60 60 60 c\n",
                "87.6142 60 110 48.8071 110 35 c\n",
                "110 21.1929 87.6142 10 60 10 c\n",
                "32.3858 10 10 21.1929 10 35 c\n",
            )
        );
        assert_eq!(shape.last_point(), Some(Point::new(10.0, 35.0)));
        assert_eq!(shape.draw_cont().matches(" c\n").count(), 4);
    }

    #[test]
    fn draw_oval_from_quad_operators() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = shape_with_identity_ctm(&mut page);
        let quad = Quad::new(
            Point::new(0.0, 0.0),
            Point::new(100.0, 20.0),
            Point::new(10.0, 50.0),
            Point::new(110.0, 70.0),
        );

        shape.draw_oval(quad).unwrap();

        assert_eq!(
            shape.draw_cont(),
            concat!(
                "5 25 m\n",
                "7.76142 38.8071 32.3858 54.4772 60 60 c\n",
                "87.6142 65.5229 107.761 58.8071 105 45 c\n",
                "102.239 31.1929 77.6142 15.5228 50 10 c\n",
                "22.3858 4.47715 2.23858 11.1929 5 25 c\n",
            )
        );
        assert_eq!(shape.draw_cont().matches(" c\n").count(), 4);
    }

    #[test]
    fn draw_quad_emits_polyline() {
        let quad = Quad::new(
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(0.0, 50.0),
            Point::new(100.0, 50.0),
        );

        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let actual = {
            let mut shape = shape_with_identity_ctm(&mut page);
            shape.draw_quad(quad.clone()).unwrap();
            shape.draw_cont().to_owned()
        };

        let mut polyline_page = doc.new_page(Size::A4).unwrap();
        let expected = {
            let mut shape = shape_with_identity_ctm(&mut polyline_page);
            shape
                .draw_polyline(&[quad.ul, quad.ll, quad.lr, quad.ur, quad.ul])
                .unwrap();
            shape.draw_cont().to_owned()
        };

        assert_eq!(actual, expected);
        assert_eq!(actual, "0 0 m\n0 50 l\n100 50 l\n100 0 l\n0 0 l\n");
    }

    #[test]
    fn draw_rect_without_radius_uses_re_operator() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = shape_with_identity_ctm(&mut page);

        shape
            .draw_rect(&Rect::new(10.0, 20.0, 110.0, 80.0))
            .unwrap();

        assert_eq!(shape.draw_cont(), "10 20 100 60 re\n");
        assert!(!shape.draw_cont().contains(" m\n"));
        assert!(!shape.draw_cont().contains(" l\n"));
        assert!(!shape.draw_cont().contains(" c\n"));
        assert_eq!(
            shape.last_point(),
            Some(Rect::new(10.0, 20.0, 110.0, 80.0).bl())
        );
        assert_eq!(shape.rect(), Some(Rect::new(10.0, 20.0, 110.0, 80.0)));
    }

    #[test]
    fn draw_rect_with_radius_emits_compound_path() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = shape_with_identity_ctm(&mut page);

        shape
            .draw_rect_with_radius(&Rect::new(0.0, 0.0, 100.0, 50.0), (10.0, 10.0))
            .unwrap();

        assert_eq!(
            shape.draw_cont(),
            concat!(
                "0 10 m\n",
                "0 40 l\n",
                "0 45.5228 4.47715 50 10 50 c\n",
                "90 50 l\n",
                "95.5229 50 100 45.5228 100 40 c\n",
                "100 10 l\n",
                "100 4.47715 95.5229 0 90 0 c\n",
                "10 0 l\n",
                "4.47715 0 0 4.47715 0 10 c\n",
            )
        );
        assert!(!shape.draw_cont().contains(" re\n"));
        assert_eq!(shape.draw_cont().matches(" m\n").count(), 1);
        assert_eq!(shape.draw_cont().matches(" l\n").count(), 4);
        assert_eq!(shape.draw_cont().matches(" c\n").count(), 4);
    }

    #[test]
    fn draw_rect_radius_clamped_to_half_min_side() {
        let rect = Rect::new(0.0, 0.0, 40.0, 20.0);

        let mut doc = PdfDocument::new();
        let mut absolute_page = doc.new_page(Size::A4).unwrap();
        let absolute = {
            let mut shape = shape_with_identity_ctm(&mut absolute_page);
            shape.draw_rect_with_radius(&rect, 50.0).unwrap();
            shape.draw_cont().to_owned()
        };

        let mut fractional_page = doc.new_page(Size::A4).unwrap();
        let fractional = {
            let mut shape = shape_with_identity_ctm(&mut fractional_page);
            shape
                .draw_rect_with_radius(&rect, RectRadius::fractional(0.5, 1.0))
                .unwrap();
            shape.draw_cont().to_owned()
        };

        assert_eq!(absolute, fractional);
        assert_eq!(
            absolute,
            concat!(
                "0 10 m\n",
                "0 10 l\n",
                "0 15.5228 4.47715 20 10 20 c\n",
                "30 20 l\n",
                "35.5228 20 40 15.5228 40 10 c\n",
                "40 10 l\n",
                "40 4.47715 35.5228 0 30 0 c\n",
                "10 0 l\n",
                "4.47715 0 0 4.47715 0 10 c\n",
            )
        );
    }

    #[test]
    fn draw_rect_radius_fractional_form() {
        let rect = Rect::new(0.0, 0.0, 100.0, 40.0);

        let mut doc = PdfDocument::new();
        let mut fractional_page = doc.new_page(Size::A4).unwrap();
        let fractional = {
            let mut shape = shape_with_identity_ctm(&mut fractional_page);
            shape.draw_rect_with_radius(&rect, (0.25, 0.5)).unwrap();
            shape.draw_cont().to_owned()
        };

        let mut absolute_page = doc.new_page(Size::A4).unwrap();
        let absolute = {
            let mut shape = shape_with_identity_ctm(&mut absolute_page);
            shape
                .draw_rect_with_radius(&rect, RectRadius::absolute_xy(25.0, 20.0))
                .unwrap();
            shape.draw_cont().to_owned()
        };

        assert_eq!(fractional, absolute);
    }

    #[test]
    fn draw_zigzag_uses_util_hor_matrix() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = shape_with_identity_ctm(&mut page);

        shape
            .draw_zigzag(Point::new(0.0, 0.0), Point::new(16.0, 0.0), 2.0)
            .unwrap();

        assert_eq!(
            shape.draw_cont(),
            concat!(
                "0 0 m\n",
                "2 -2 l\n",
                "6 2 l\n",
                "10 -2 l\n",
                "14 2 l\n",
                "16 0 l\n",
            )
        );
        assert_eq!(shape.last_point(), Some(Point::new(16.0, 0.0)));
    }

    #[test]
    fn draw_squiggle_emits_curves_not_lines() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = shape_with_identity_ctm(&mut page);

        shape
            .draw_squiggle(Point::new(0.0, 0.0), Point::new(16.0, 0.0), 2.0)
            .unwrap();

        assert_eq!(
            shape.draw_cont(),
            concat!(
                "0 0 m\n",
                "1.10457 -2.66667 2.89543 -2.66667 4 0 c\n",
                "5.10457 2.66667 6.89543 2.66667 8 0 c\n",
                "9.10457 -2.66667 10.8954 -2.66667 12 0 c\n",
                "13.1046 2.66667 14.8954 2.66667 16 0 c\n",
            )
        );
        assert_eq!(shape.draw_cont().matches(" c\n").count(), 4);
        assert!(!shape.draw_cont().contains(" l\n"));
        assert_eq!(shape.last_point(), Some(Point::new(16.0, 0.0)));
    }

    #[test]
    fn draw_zigzag_squiggle_breadth_zero_degenerates_to_line() {
        let p1 = Point::new(3.0, 4.0);
        let p2 = Point::new(15.0, 20.0);

        let mut doc = PdfDocument::new();
        let mut line_page = doc.new_page(Size::A4).unwrap();
        let line = {
            let mut shape = shape_with_identity_ctm(&mut line_page);
            shape.draw_line(p1, p2).unwrap();
            shape.draw_cont().to_owned()
        };

        let mut zigzag_page = doc.new_page(Size::A4).unwrap();
        let zigzag = {
            let mut shape = shape_with_identity_ctm(&mut zigzag_page);
            shape.draw_zigzag(p1, p2, 0.0).unwrap();
            shape.draw_cont().to_owned()
        };

        let mut squiggle_page = doc.new_page(Size::A4).unwrap();
        let squiggle = {
            let mut shape = shape_with_identity_ctm(&mut squiggle_page);
            shape.draw_squiggle(p1, p2, 0.0).unwrap();
            shape.draw_cont().to_owned()
        };

        assert_eq!(zigzag, line);
        assert_eq!(squiggle, line);
    }

    #[test]
    fn draw_zigzag_diagonal_orientation() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(100.0, 100.0);
        let breadth = 4.0;

        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = shape_with_identity_ctm(&mut page);

        shape.draw_zigzag(p1, p2, breadth).unwrap();

        let matrix = util_hor_matrix(p1, p2);
        let vertices = shape
            .draw_cont()
            .lines()
            .filter_map(parse_line_to_vertex)
            .collect::<Vec<_>>();
        let internal_vertices = &vertices[..vertices.len() - 1];
        let phase_count = wave_phase_count((p2 - p1).length(), breadth).unwrap();
        let adjusted_breadth = (p2 - p1).length() / phase_count as f32;
        assert_eq!(internal_vertices.len(), phase_count / 2);

        for (index, vertex) in internal_vertices.iter().copied().enumerate() {
            let transformed = vertex.mul_matrix(&matrix);
            let expected = if index % 2 == 0 {
                -adjusted_breadth
            } else {
                adjusted_breadth
            };
            assert!(
                (transformed.y - expected).abs() <= 1e-4,
                "vertex {index}={vertex:?} transformed to {transformed:?}, expected y={expected}"
            );
        }

        let endpoint = vertices.last().copied().unwrap().mul_matrix(&matrix);
        assert!(endpoint.y.abs() <= 1e-4);
    }

    fn parse_line_to_vertex(line: &str) -> Option<Point> {
        let mut parts = line.split_whitespace();
        let x = parts.next()?.parse::<f32>().ok()?;
        let y = parts.next()?.parse::<f32>().ok()?;
        (parts.next()? == "l").then_some(Point::new(x, y))
    }
}

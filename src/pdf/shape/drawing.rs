use super::operators::format_g;
use super::Shape;
use crate::{Error, Point, Quad, Rect};

const CURVE_KAPPA: f32 = 0.552_284_8;

impl Shape<'_> {
    /// Draws a straight line from `p1` to `p2`.
    ///
    /// Equivalent of PyMuPDF `Shape.draw_line`.
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
    /// Rounded corners are intentionally not supported by this milestone.
    /// Equivalent of PyMuPDF `Shape.draw_rect` for the non-rounded case.
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
        self.set_last_point(top_left);
        Ok(self)
    }

    /// Draws a cubic Bézier curve from `p1` to `p4` using controls `p2` and `p3`.
    ///
    /// Equivalent of PyMuPDF `Shape.draw_bezier`.
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
    pub fn draw_sector(
        &mut self,
        center: Point,
        point: Point,
        beta: f32,
        full_sector: bool,
    ) -> Result<&mut Self, Error> {
        self.move_to_if_needed(point);

        let radius = (point - center).length();
        if radius <= f32::EPSILON {
            return Ok(self);
        }

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
            self.arc_segment(center, radius, current, start_angle, remaining);
        }

        if full_sector {
            self.line_to(center);
            self.line_to(point);
        }

        Ok(self)
    }

    /// Draws a circle with `center` and `radius`.
    ///
    /// Equivalent of PyMuPDF `Shape.draw_circle`, implemented as a full `draw_sector` arc.
    pub fn draw_circle(&mut self, center: Point, radius: f32) -> Result<&mut Self, Error> {
        let radius = radius.abs();
        let point = Point::new(center.x + radius, center.y);
        self.draw_sector(center, point, 360.0, false)
    }

    /// Draws an oval inside a rectangle or quadrilateral.
    ///
    /// Equivalent of PyMuPDF `Shape.draw_oval`.
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
    pub fn draw_quad(&mut self, quad: Quad) -> Result<&mut Self, Error> {
        self.draw_polyline(&[quad.ul, quad.ur, quad.lr, quad.ll, quad.ul])
    }

    fn move_to_if_needed(&mut self, point: Point) {
        if self.needs_move_to(&point) {
            let transformed = self.transform_point(point);
            self.draw_cont.push_str(&format!(
                "{} {} m\n",
                format_g(transformed.x),
                format_g(transformed.y)
            ));
            self.update_rect(&point);
            self.set_last_point(point);
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
                "100 100 l\n",
                "150 100 l\n",
            )
        );
        assert_eq!(shape.draw_cont().matches(" c\n").count(), 1);
        assert_eq!(shape.draw_cont().matches(" l\n").count(), 2);
    }

    #[test]
    fn draw_sector_beta_360_equals_draw_circle() {
        let center = Point::new(100.0, 100.0);
        let point = Point::new(150.0, 100.0);

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
    fn draw_circle_radius_zero_is_degenerate_point() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = shape_with_identity_ctm(&mut page);

        shape.draw_circle(Point::new(100.0, 100.0), 0.0).unwrap();

        assert_eq!(shape.draw_cont(), "100 100 m\n");
        assert_eq!(shape.last_point(), Some(Point::new(100.0, 100.0)));
        assert_eq!(shape.rect(), Some(Rect::new(100.0, 100.0, 100.0, 100.0)));
        shape.finish(&Default::default()).unwrap();
        assert!(!shape.total_cont().contains(" c\n"));
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
                .draw_polyline(&[quad.ul, quad.ur, quad.lr, quad.ll, quad.ul])
                .unwrap();
            shape.draw_cont().to_owned()
        };

        assert_eq!(actual, expected);
        assert_eq!(actual, "0 0 m\n100 0 l\n100 50 l\n0 50 l\n0 0 l\n");
    }
}

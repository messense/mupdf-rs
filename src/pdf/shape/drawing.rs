use super::operators::format_g;
use super::Shape;
use crate::{Error, Point, Rect};

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
}

fn point_between(start: Point, control: Point, scale: f32) -> Point {
    Point::new(
        start.x + (control.x - start.x) * scale,
        start.y + (control.y - start.y) * scale,
    )
}

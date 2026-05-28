use std::collections::HashMap;

use crate::pdf::{FontInfo, PdfPage};
use crate::{Error, Matrix, Point, Rect};

mod drawing;
mod finish;
#[allow(dead_code)]
mod operators;
mod options;
mod text;

pub use drawing::RectRadius;
pub use options::{FinishOptions, PdfColor, TextAlign, TextOptions, TextboxOptions};

/// Builder for accumulating drawing and text operations on a PDF page.
///
/// `Shape` owns a mutable borrow of its page for as long as the builder is alive. This
/// intentionally prevents other mutable page operations from aliasing the builder's page state:
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
///
/// ```compile_fail
/// use mupdf::{pdf::PdfDocument, shape::Shape, Size};
///
/// let mut doc = PdfDocument::new();
/// let mut page = doc.new_page(Size::A4).unwrap();
/// let shape = Shape::new(&mut page).unwrap();
/// page.set_rotation(90).unwrap();
/// drop(shape);
/// ```
#[derive(Debug)]
pub struct Shape<'a> {
    page: &'a mut PdfPage,
    width: f32,
    height: f32,
    x: f32,
    y: f32,
    pctm: Matrix,
    ipctm: Matrix,
    draw_cont: String,
    text_cont: String,
    total_cont: String,
    font_info_cache: HashMap<i32, FontInfo>,
    last_point: Option<Point>,
    rect: Option<Rect>,
}

impl<'a> Shape<'a> {
    /// Creates a new shape builder bound to `page`.
    ///
    /// The constructor caches the page crop-box geometry and current transformation
    /// matrix so later drawing methods can consistently transform coordinates.
    ///
    /// ```
    /// use mupdf::{pdf::PdfDocument, shape::Shape, Size};
    ///
    /// # fn main() -> Result<(), mupdf::Error> {
    /// let mut doc = PdfDocument::new();
    /// let mut page = doc.new_page(Size::A4)?;
    /// let shape = Shape::new(&mut page)?;
    /// assert!(shape.width() > 0.0);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(page: &'a mut PdfPage) -> Result<Self, Error> {
        let crop_box = page.crop_box()?;
        let pctm = page.ctm()?;
        let ipctm = pctm.invert().ok_or(Error::NonInvertibleMatrix)?;

        Ok(Self {
            page,
            width: crop_box.width(),
            height: crop_box.height(),
            x: crop_box.x0,
            y: crop_box.y0,
            pctm,
            ipctm,
            draw_cont: String::new(),
            text_cont: String::new(),
            total_cont: String::new(),
            font_info_cache: HashMap::new(),
            last_point: None,
            rect: None,
        })
    }

    /// Returns the cached page width.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Returns the cached page height.
    pub fn height(&self) -> f32 {
        self.height
    }

    /// Returns the cached crop-box x origin.
    pub fn x(&self) -> f32 {
        self.x
    }

    /// Returns the cached crop-box y origin.
    pub fn y(&self) -> f32 {
        self.y
    }

    /// Returns the cached page current transformation matrix.
    pub fn pctm(&self) -> &Matrix {
        &self.pctm
    }

    /// Returns the cached inverse page current transformation matrix.
    pub fn ipctm(&self) -> &Matrix {
        &self.ipctm
    }

    /// Returns the accumulated drawing content buffer.
    pub fn draw_cont(&self) -> &str {
        &self.draw_cont
    }

    /// Returns the accumulated text content buffer.
    pub fn text_cont(&self) -> &str {
        &self.text_cont
    }

    /// Returns the accumulated total content buffer.
    pub fn total_cont(&self) -> &str {
        &self.total_cont
    }

    /// Returns the last point in the current path, if any.
    pub fn last_point(&self) -> Option<Point> {
        self.last_point
    }

    /// Returns the current accumulated bounding rectangle, if any.
    pub fn rect(&self) -> Option<Rect> {
        self.rect
    }

    /// Returns the page this shape is bound to.
    pub fn page(&self) -> &PdfPage {
        self.page
    }

    /// Expands the accumulated bounding rectangle to contain `point`.
    #[allow(dead_code)]
    pub(crate) fn update_rect(&mut self, point: &Point) -> &mut Self {
        self.update_rect_with_rect(Rect::new(point.x, point.y, point.x, point.y))
    }

    /// Expands the accumulated bounding rectangle to contain `rect`.
    #[allow(dead_code)]
    pub(crate) fn update_rect_with_rect(&mut self, rect: Rect) -> &mut Self {
        let rect = normalize_rect(rect);
        self.rect = Some(match self.rect {
            Some(current) => union_rects(current, rect),
            None => rect,
        });
        self
    }

    /// Records the current path's last point.
    #[allow(dead_code)]
    pub(crate) fn set_last_point(&mut self, point: Point) -> &mut Self {
        self.last_point = Some(point);
        self
    }

    /// Clears path-state bookkeeping for a new drawing block.
    #[allow(dead_code)]
    pub(crate) fn clear_path_state(&mut self) -> &mut Self {
        self.last_point = None;
        self.rect = None;
        self
    }

    /// Returns whether drawing from `point` requires a PDF `m` operator.
    #[allow(dead_code)]
    pub(crate) fn needs_move_to(&self, point: &Point) -> bool {
        self.last_point != Some(*point)
    }
}

fn normalize_rect(rect: Rect) -> Rect {
    Rect::new(
        rect.x0.min(rect.x1),
        rect.y0.min(rect.y1),
        rect.x0.max(rect.x1),
        rect.y0.max(rect.y1),
    )
}

fn union_rects(lhs: Rect, rhs: Rect) -> Rect {
    let lhs = normalize_rect(lhs);
    let rhs = normalize_rect(rhs);
    Rect::new(
        lhs.x0.min(rhs.x0),
        lhs.y0.min(rhs.y0),
        lhs.x1.max(rhs.x1),
        lhs.y1.max(rhs.y1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::PdfDocument;
    use crate::Size;

    fn assert_matrix_near(actual: &Matrix, expected: &Matrix, epsilon: f32) {
        assert!(
            (actual.a - expected.a).abs() <= epsilon,
            "matrix a mismatch: actual={actual:?}, expected={expected:?}"
        );
        assert!(
            (actual.b - expected.b).abs() <= epsilon,
            "matrix b mismatch: actual={actual:?}, expected={expected:?}"
        );
        assert!(
            (actual.c - expected.c).abs() <= epsilon,
            "matrix c mismatch: actual={actual:?}, expected={expected:?}"
        );
        assert!(
            (actual.d - expected.d).abs() <= epsilon,
            "matrix d mismatch: actual={actual:?}, expected={expected:?}"
        );
        assert!(
            (actual.e - expected.e).abs() <= epsilon,
            "matrix e mismatch: actual={actual:?}, expected={expected:?}"
        );
        assert!(
            (actual.f - expected.f).abs() <= epsilon,
            "matrix f mismatch: actual={actual:?}, expected={expected:?}"
        );
    }

    #[test]
    fn new_caches_page_geometry() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let expected_ctm = page.ctm().unwrap();
        let expected_ictm = expected_ctm.invert().unwrap();

        let shape = Shape::new(&mut page).unwrap();

        assert_eq!(shape.width(), 595.0);
        assert_eq!(shape.height(), 842.0);
        assert_eq!(shape.x(), 0.0);
        assert_eq!(shape.y(), 0.0);
        assert_eq!(shape.pctm(), &expected_ctm);
        assert_eq!(shape.ipctm(), &expected_ictm);
    }

    #[test]
    fn new_rotated_page_pctm() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        page.set_rotation(90).unwrap();

        let shape = Shape::new(&mut page).unwrap();

        assert_eq!(shape.width(), 842.0);
        assert_eq!(shape.height(), 595.0);
        assert_eq!(shape.x(), 0.0);
        assert_eq!(shape.y(), 247.0);
        assert_ne!(shape.pctm(), &Matrix::IDENTITY);
        assert_matrix_near(&(shape.pctm() * shape.ipctm()), &Matrix::IDENTITY, 1e-5);
        assert_matrix_near(&(shape.ipctm() * shape.pctm()), &Matrix::IDENTITY, 1e-5);
    }

    #[test]
    fn new_initial_state() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();

        let shape = Shape::new(&mut page).unwrap();

        assert!(shape.draw_cont().is_empty());
        assert!(shape.text_cont().is_empty());
        assert!(shape.total_cont().is_empty());
        assert_eq!(shape.last_point(), None);
        assert_eq!(shape.rect(), None);
    }

    #[test]
    fn update_rect_points() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();

        shape.update_rect(&Point::new(10.0, 10.0));
        assert_eq!(shape.rect(), Some(Rect::new(10.0, 10.0, 10.0, 10.0)));

        shape.update_rect(&Point::new(50.0, 80.0));
        assert_eq!(shape.rect(), Some(Rect::new(10.0, 10.0, 50.0, 80.0)));

        shape.update_rect(&Point::new(30.0, 40.0));
        assert_eq!(shape.rect(), Some(Rect::new(10.0, 10.0, 50.0, 80.0)));
    }

    #[test]
    fn update_rect_rects() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();

        shape.update_rect_with_rect(Rect::new(0.0, 0.0, 30.0, 30.0));
        assert_eq!(shape.rect(), Some(Rect::new(0.0, 0.0, 30.0, 30.0)));

        shape.update_rect_with_rect(Rect::new(20.0, 40.0, 60.0, 50.0));
        assert_eq!(shape.rect(), Some(Rect::new(0.0, 0.0, 60.0, 50.0)));
    }

    #[test]
    fn path_state_helpers_track_last_point() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();

        assert!(shape.needs_move_to(&Point::new(1.0, 2.0)));
        shape.set_last_point(Point::new(1.0, 2.0));
        assert_eq!(shape.last_point(), Some(Point::new(1.0, 2.0)));
        assert!(!shape.needs_move_to(&Point::new(1.0, 2.0)));
        assert!(shape.needs_move_to(&Point::new(2.0, 3.0)));
        shape.clear_path_state();
        assert_eq!(shape.last_point(), None);
        assert_eq!(shape.rect(), None);
    }

    #[test]
    fn builder_chain_compiles() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();

        shape
            .update_rect(&Point::new(10.0, 10.0))
            .update_rect(&Point::new(50.0, 80.0))
            .set_last_point(Point::new(50.0, 80.0));

        assert_eq!(shape.rect(), Some(Rect::new(10.0, 10.0, 50.0, 80.0)));
        assert_eq!(shape.last_point(), Some(Point::new(50.0, 80.0)));
    }

    #[test]
    fn draw_line_basic_operators() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
        shape.ipctm = Matrix::IDENTITY;

        shape
            .draw_line(Point::new(10.0, 20.0), Point::new(30.0, 40.0))
            .unwrap();

        assert_eq!(shape.draw_cont(), "10 20 m\n30 40 l\n");
        assert_eq!(shape.last_point(), Some(Point::new(30.0, 40.0)));
        assert_eq!(shape.rect(), Some(Rect::new(10.0, 20.0, 30.0, 40.0)));
    }

    #[test]
    fn draw_line_chain_reuses_last_point() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
        shape.ipctm = Matrix::IDENTITY;

        shape
            .draw_line(Point::new(10.0, 20.0), Point::new(30.0, 40.0))
            .unwrap()
            .draw_line(Point::new(30.0, 40.0), Point::new(50.0, 60.0))
            .unwrap();

        assert_eq!(shape.draw_cont(), "10 20 m\n30 40 l\n50 60 l\n");
        assert_eq!(shape.last_point(), Some(Point::new(50.0, 60.0)));
    }

    #[test]
    fn draw_line_first_call_emits_m() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
        shape.ipctm = Matrix::IDENTITY;

        shape
            .draw_line(Point::new(10.0, 20.0), Point::new(30.0, 40.0))
            .unwrap();

        assert!(shape.draw_cont().starts_with("10 20 m\n"));
    }

    #[test]
    fn draw_polyline_two_points() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
        shape.ipctm = Matrix::IDENTITY;

        shape
            .draw_polyline(&[Point::new(10.0, 10.0), Point::new(20.0, 20.0)])
            .unwrap();

        assert_eq!(shape.draw_cont(), "10 10 m\n20 20 l\n");
        assert_eq!(shape.last_point(), Some(Point::new(20.0, 20.0)));
    }

    #[test]
    fn draw_polyline_many_points() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
        shape.ipctm = Matrix::IDENTITY;

        shape
            .draw_polyline(&[
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                Point::new(10.0, 10.0),
                Point::new(0.0, 10.0),
                Point::new(0.0, 0.0),
            ])
            .unwrap();

        assert_eq!(shape.draw_cont(), "0 0 m\n10 0 l\n10 10 l\n0 10 l\n0 0 l\n");
        assert_eq!(shape.rect(), Some(Rect::new(0.0, 0.0, 10.0, 10.0)));
    }

    #[test]
    fn draw_polyline_underflow_handled() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();

        shape.draw_polyline(&[]).unwrap();
        shape.draw_polyline(&[Point::new(1.0, 1.0)]).unwrap();

        assert!(shape.draw_cont().is_empty());
        assert_eq!(shape.last_point(), None);
        assert_eq!(shape.rect(), None);
    }

    #[test]
    fn draw_rect_basic_operators() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
        shape.ipctm = Matrix::IDENTITY;

        shape.draw_rect(&Rect::new(10.0, 20.0, 40.0, 60.0)).unwrap();

        assert_eq!(shape.draw_cont(), "10 20 30 40 re\n");
        assert_eq!(shape.last_point(), Some(Point::new(10.0, 60.0)));
        assert_eq!(shape.rect(), Some(Rect::new(10.0, 20.0, 40.0, 60.0)));
    }

    #[test]
    fn draw_bezier_basic_operators() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
        shape.ipctm = Matrix::IDENTITY;

        shape
            .draw_bezier(
                Point::new(0.0, 0.0),
                Point::new(0.0, 10.0),
                Point::new(10.0, 10.0),
                Point::new(10.0, 0.0),
            )
            .unwrap();

        assert_eq!(shape.draw_cont(), "0 0 m\n0 10 10 10 10 0 c\n");
        assert_eq!(shape.last_point(), Some(Point::new(10.0, 0.0)));
        assert_eq!(shape.rect(), Some(Rect::new(0.0, 0.0, 10.0, 10.0)));
    }

    #[test]
    fn draw_curve_uses_kappa() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
        shape.ipctm = Matrix::IDENTITY;

        shape
            .draw_curve(
                Point::new(0.0, 0.0),
                Point::new(0.0, 10.0),
                Point::new(10.0, 10.0),
            )
            .unwrap();

        assert_eq!(shape.draw_cont(), "0 0 m\n0 5.52285 4.47715 10 10 10 c\n");
    }

    #[test]
    fn draw_line_format_g_integration() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
        shape.ipctm = Matrix::IDENTITY;

        shape
            .draw_line(Point::new(1.234567, 0.0), Point::new(10.0, 0.5))
            .unwrap();

        assert_eq!(shape.draw_cont(), "1.23457 0 m\n10 0.5 l\n");
    }

    #[test]
    fn draw_applies_ipctm() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
        shape.ipctm = Matrix::new(2.0, 0.0, 0.0, 3.0, 5.0, 7.0);

        shape
            .draw_line(Point::new(10.0, 20.0), Point::new(30.0, 40.0))
            .unwrap();

        assert_eq!(shape.draw_cont(), "25 67 m\n65 127 l\n");
        assert_eq!(shape.last_point(), Some(Point::new(30.0, 40.0)));
    }

    #[test]
    fn drawing_builder_chain_compiles() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
        shape.ipctm = Matrix::IDENTITY;

        shape
            .draw_line(Point::new(10.0, 20.0), Point::new(30.0, 40.0))
            .unwrap()
            .draw_rect(&Rect::new(40.0, 50.0, 70.0, 90.0))
            .unwrap();

        assert_eq!(shape.draw_cont(), "10 20 m\n30 40 l\n40 50 30 40 re\n");
        assert_eq!(shape.last_point(), Some(Point::new(40.0, 90.0)));
    }
}

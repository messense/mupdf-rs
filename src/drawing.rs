use std::fmt;

use crate::{
    ColorParams, Colorspace, Error, Image, LineCap, LineJoin, Matrix, NativeDevice, Path,
    PathWalker, Point, Quad, Rect, Shade, StrokeState, Text,
};

const POINT_EPSILON: f32 = 1.0e-4;

/// The painting operation represented by a [`Drawing`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawingType {
    /// Fill-only path.
    Fill,
    /// Stroke-only path.
    Stroke,
    /// Combined fill and stroke path.
    FillStroke,
}

/// One path item in a [`Drawing`], matching PyMuPDF's `Page.get_drawings()` items.
#[derive(Debug, Clone, PartialEq)]
pub enum DrawingItem {
    /// A line segment from the first point to the second point.
    Line(Point, Point),
    /// A cubic Bézier curve from the first point to the fourth point.
    ///
    /// The second and third points are the control points.
    Curve(Point, Point, Point, Point),
    /// An axis-aligned rectangle and its orientation.
    ///
    /// The orientation is `1` for anti-clockwise rectangles and `-1` for clockwise rectangles,
    /// matching PyMuPDF's `("re", rect, orientation)` item.
    Rect { rect: Rect, orientation: i32 },
    /// A quadrilateral detected from four consecutive stroke line segments.
    Quad(Quad),
}

/// Stroke dash pattern information for a [`Drawing`].
#[derive(Debug, Clone, PartialEq)]
pub struct DrawingDashes {
    /// Dash lengths, scaled into page coordinates.
    pub dashes: Vec<f32>,
    /// Dash phase, scaled into page coordinates.
    pub phase: f32,
}

impl fmt::Display for DrawingDashes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.dashes.is_empty() {
            return write!(f, "[] {}", self.phase);
        }

        write!(f, "[")?;
        for (idx, dash) in self.dashes.iter().enumerate() {
            if idx > 0 {
                write!(f, " ")?;
            }
            write!(f, "{dash}")?;
        }
        write!(f, "] {}", self.phase)
    }
}

/// A vector drawing path extracted from a page.
///
/// This mirrors the default (`extended = false`) PyMuPDF `Page.get_drawings()` output: each
/// drawing contains path items plus the common fill / stroke properties used to paint them.
#[derive(Debug, Clone, PartialEq)]
pub struct Drawing {
    /// Path items: lines, curves, rectangles or quads.
    pub items: Vec<DrawingItem>,
    /// Whether this path fills, strokes, or does both.
    pub drawing_type: DrawingType,
    /// Bounding rectangle of the path geometry.
    pub rect: Rect,
    /// Stroke color converted to DeviceRGB, if this drawing is stroked.
    pub color: Option<[f32; 3]>,
    /// Fill color converted to DeviceRGB, if this drawing is filled.
    pub fill: Option<[f32; 3]>,
    /// Whether fill uses the even-odd rule, if this drawing is filled.
    pub even_odd: Option<bool>,
    /// PyMuPDF-compatible `closePath` flag for replaying the path.
    ///
    /// This is `false` when the extracted items already contain any required closing segment.
    pub close_path: Option<bool>,
    /// Stroke line width in page coordinates, if this drawing is stroked.
    pub width: Option<f32>,
    /// Stroke line caps `(start, dash, end)`, if this drawing is stroked.
    pub line_cap: Option<(LineCap, LineCap, LineCap)>,
    /// Stroke line join, if this drawing is stroked.
    pub line_join: Option<LineJoin>,
    /// Stroke dashes, if this drawing is stroked.
    pub dashes: Option<DrawingDashes>,
    /// Fill opacity, if this drawing is filled.
    pub fill_opacity: Option<f32>,
    /// Stroke opacity, if this drawing is stroked.
    pub stroke_opacity: Option<f32>,
    /// Sequence number in the page appearance stream.
    pub seqno: usize,
    /// Optional-content layer name active for this path.
    pub layer: Option<String>,
}

#[derive(Default)]
pub(crate) struct DrawingDevice {
    drawings: Vec<Drawing>,
    error: Option<Error>,
    seqno: usize,
    layer: Option<String>,
}

struct CollectedPath {
    items: Vec<DrawingItem>,
    rect: Rect,
    close_path: Option<bool>,
}

impl DrawingDevice {
    pub(crate) fn finish(&mut self) -> Result<Vec<Drawing>, Error> {
        if let Some(error) = self.error.take() {
            return Err(error);
        }
        Ok(std::mem::take(&mut self.drawings))
    }

    fn set_error(&mut self, error: Error) {
        if self.error.is_none() {
            self.error = Some(error);
        }
    }

    fn has_error(&self) -> bool {
        self.error.is_some()
    }

    fn collect_path(
        &mut self,
        path: &Path,
        ctm: &Matrix,
        drawing_type: DrawingType,
    ) -> Result<Option<CollectedPath>, Error> {
        let mut walker = DrawingPathWalker::new(ctm, drawing_type != DrawingType::Fill);
        path.walk(&mut walker)?;
        if walker.items.is_empty() {
            return Ok(None);
        }

        Ok(Some(CollectedPath {
            items: walker.items,
            rect: walker.path_rect.unwrap_or_default(),
            close_path: walker.close_path,
        }))
    }

    fn color_to_rgb(
        &mut self,
        color_space: &Colorspace,
        color: &[f32],
        cp: ColorParams,
    ) -> Option<[f32; 3]> {
        let rgb = Colorspace::device_rgb();
        match color_space.convert_color(color, &rgb, None, cp) {
            Ok(color) if color.len() >= 3 => Some([color[0], color[1], color[2]]),
            Ok(_) => {
                self.set_error(Error::InvalidArgument(
                    "color conversion returned fewer than 3 components".to_owned(),
                ));
                None
            }
            Err(error) => {
                self.set_error(error);
                None
            }
        }
    }

    fn append_or_merge(&mut self, drawing: Drawing) {
        if drawing.drawing_type == DrawingType::Stroke {
            if let Some(previous) = self.drawings.last_mut() {
                if previous.drawing_type == DrawingType::Fill && previous.items == drawing.items {
                    previous.drawing_type = DrawingType::FillStroke;
                    previous.color = drawing.color;
                    previous.width = drawing.width;
                    previous.line_cap = drawing.line_cap;
                    previous.line_join = drawing.line_join;
                    previous.dashes = drawing.dashes;
                    previous.stroke_opacity = drawing.stroke_opacity;
                    if previous.close_path.is_none() {
                        previous.close_path = drawing.close_path;
                    }
                    return;
                }
            }
        }

        self.drawings.push(drawing);
    }

    fn increase_seqno(&mut self) {
        self.seqno += 1;
    }
}

impl NativeDevice for DrawingDevice {
    fn fill_path(
        &mut self,
        path: &Path,
        even_odd: bool,
        ctm: Matrix,
        color_space: &Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) {
        if self.has_error() {
            return;
        }

        let path = match self.collect_path(path, &ctm, DrawingType::Fill) {
            Ok(Some(path)) => path,
            Ok(None) => return,
            Err(error) => {
                self.set_error(error);
                return;
            }
        };
        let Some(fill) = self.color_to_rgb(color_space, color, cp) else {
            return;
        };

        self.append_or_merge(Drawing {
            items: path.items,
            drawing_type: DrawingType::Fill,
            rect: path.rect,
            color: None,
            fill: Some(fill),
            even_odd: Some(even_odd),
            close_path: path.close_path,
            width: None,
            line_cap: None,
            line_join: None,
            dashes: None,
            fill_opacity: Some(alpha),
            stroke_opacity: None,
            seqno: self.seqno,
            layer: self.layer.clone(),
        });
        self.increase_seqno();
    }

    fn stroke_path(
        &mut self,
        path: &Path,
        stroke_state: &StrokeState,
        ctm: Matrix,
        color_space: &Colorspace,
        color: &[f32],
        alpha: f32,
        cp: ColorParams,
    ) {
        if self.has_error() {
            return;
        }

        let path = match self.collect_path(path, &ctm, DrawingType::Stroke) {
            Ok(Some(path)) => path,
            Ok(None) => return,
            Err(error) => {
                self.set_error(error);
                return;
            }
        };
        let Some(stroke_color) = self.color_to_rgb(color_space, color, cp) else {
            return;
        };
        let path_factor = (ctm.a * ctm.d - ctm.b * ctm.c).abs().sqrt();
        let dashes = stroke_state
            .dashes()
            .into_iter()
            .map(|dash| dash * path_factor)
            .collect();

        self.append_or_merge(Drawing {
            items: path.items,
            drawing_type: DrawingType::Stroke,
            rect: path.rect,
            color: Some(stroke_color),
            fill: None,
            even_odd: None,
            close_path: Some(path.close_path.unwrap_or(false)),
            width: Some(stroke_state.line_width() * path_factor),
            line_cap: Some((
                stroke_state.start_cap(),
                stroke_state.dash_cap(),
                stroke_state.end_cap(),
            )),
            line_join: Some(stroke_state.line_join()),
            dashes: Some(DrawingDashes {
                dashes,
                phase: stroke_state.dash_phase() * path_factor,
            }),
            fill_opacity: None,
            stroke_opacity: Some(alpha),
            seqno: self.seqno,
            layer: self.layer.clone(),
        });
        self.increase_seqno();
    }

    fn fill_text(
        &mut self,
        _text: &Text,
        _ctm: Matrix,
        _color_space: &Colorspace,
        _color: &[f32],
        _alpha: f32,
        _cp: ColorParams,
    ) {
        self.increase_seqno();
    }

    fn stroke_text(
        &mut self,
        _text: &Text,
        _stroke_state: &StrokeState,
        _ctm: Matrix,
        _color_space: &Colorspace,
        _color: &[f32],
        _alpha: f32,
        _cp: ColorParams,
    ) {
        self.increase_seqno();
    }

    fn ignore_text(&mut self, _text: &Text, _ctm: Matrix) {
        self.increase_seqno();
    }

    fn fill_shade(&mut self, _shade: &Shade, _ctm: Matrix, _alpha: f32, _cp: ColorParams) {
        self.increase_seqno();
    }

    fn fill_image(&mut self, _img: &Image, _ctm: Matrix, _alpha: f32, _cp: ColorParams) {
        self.increase_seqno();
    }

    fn fill_image_mask(
        &mut self,
        _img: &Image,
        _ctm: Matrix,
        _color_space: &Colorspace,
        _color: &[f32],
        _alpha: f32,
        _cp: ColorParams,
    ) {
        self.increase_seqno();
    }

    fn begin_layer(&mut self, name: &str) {
        self.layer = Some(name.to_owned());
    }

    fn end_layer(&mut self) {
        self.layer = None;
    }
}

struct DrawingPathWalker<'a> {
    ctm: &'a Matrix,
    detect_quads: bool,
    items: Vec<DrawingItem>,
    path_rect: Option<Rect>,
    last_point: Point,
    first_point: Point,
    have_move: bool,
    line_count: usize,
    close_path: Option<bool>,
}

impl<'a> DrawingPathWalker<'a> {
    fn new(ctm: &'a Matrix, detect_quads: bool) -> Self {
        Self {
            ctm,
            detect_quads,
            items: Vec::new(),
            path_rect: None,
            last_point: Point::new(0.0, 0.0),
            first_point: Point::new(0.0, 0.0),
            have_move: false,
            line_count: 0,
            close_path: None,
        }
    }

    fn transform_point(&self, x: f32, y: f32) -> Point {
        Point::new(x, y).transform(self.ctm)
    }

    fn include_point(&mut self, point: Point) {
        self.path_rect = Some(match self.path_rect {
            Some(rect) => Rect::new(
                rect.x0.min(point.x),
                rect.y0.min(point.y),
                rect.x1.max(point.x),
                rect.y1.max(point.y),
            ),
            None => Rect::new(point.x, point.y, point.x, point.y),
        });
    }

    fn check_quad(&mut self) -> bool {
        if self.items.len() < 4 {
            return false;
        }

        let len = self.items.len();
        let [DrawingItem::Line(ul, line0_end), DrawingItem::Line(ll, line1_end), DrawingItem::Line(lr, line2_end), DrawingItem::Line(ur, line3_end)] =
            &self.items[len - 4..]
        else {
            return false;
        };

        if !points_nearly_equal(*line0_end, *ll)
            || !points_nearly_equal(*line1_end, *lr)
            || !points_nearly_equal(*line2_end, *ur)
            || !points_nearly_equal(*line3_end, *ul)
        {
            return false;
        }

        let quad = Quad::new(*ul, *ur, *ll, *lr);
        self.items.truncate(len - 4);
        self.items.push(DrawingItem::Quad(quad));
        self.line_count = 0;
        true
    }

    fn check_rect(&mut self) -> bool {
        self.line_count = 0;
        if self.items.len() < 3 {
            return false;
        }

        let len = self.items.len();
        let (ll, lr, ur, ul) = match (&self.items[len - 3], &self.items[len - 1]) {
            (DrawingItem::Line(ll, lr), DrawingItem::Line(ur, ul)) => (*ll, *lr, *ur, *ul),
            _ => return false,
        };

        if !nearly_equal(ll.y, lr.y)
            || !nearly_equal(ll.x, ul.x)
            || !nearly_equal(ur.y, ul.y)
            || !nearly_equal(ur.x, lr.x)
        {
            return false;
        }

        let orientation = if ul.y < lr.y { 1 } else { -1 };
        let rect = Rect::new(
            ll.x.min(lr.x).min(ur.x).min(ul.x),
            ll.y.min(lr.y).min(ur.y).min(ul.y),
            ll.x.max(lr.x).max(ur.x).max(ul.x),
            ll.y.max(lr.y).max(ur.y).max(ul.y),
        );

        self.items.truncate(len - 3);
        self.items.push(DrawingItem::Rect { rect, orientation });
        true
    }
}

fn nearly_equal(a: f32, b: f32) -> bool {
    (a - b).abs() <= POINT_EPSILON
}

fn points_nearly_equal(a: Point, b: Point) -> bool {
    nearly_equal(a.x, b.x) && nearly_equal(a.y, b.y)
}

impl PathWalker for DrawingPathWalker<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        let point = self.transform_point(x, y);
        self.last_point = point;
        self.first_point = point;
        self.have_move = true;
        self.line_count = 0;
        self.include_point(point);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let point = self.transform_point(x, y);
        self.include_point(point);
        self.items.push(DrawingItem::Line(self.last_point, point));
        self.last_point = point;
        self.line_count += 1;
        if self.detect_quads && self.line_count == 4 {
            self.check_quad();
        }
    }

    fn curve_to(&mut self, cx1: f32, cy1: f32, cx2: f32, cy2: f32, ex: f32, ey: f32) {
        self.line_count = 0;
        let control1 = self.transform_point(cx1, cy1);
        let control2 = self.transform_point(cx2, cy2);
        let end = self.transform_point(ex, ey);
        self.include_point(control1);
        self.include_point(control2);
        self.include_point(end);
        self.items
            .push(DrawingItem::Curve(self.last_point, control1, control2, end));
        self.last_point = end;
    }

    fn close(&mut self) {
        if self.line_count == 3 && self.check_rect() {
            self.last_point = self.first_point;
            self.close_path = Some(false);
            self.have_move = false;
            return;
        }

        self.line_count = 0;
        if self.have_move {
            if !points_nearly_equal(self.first_point, self.last_point) {
                self.items
                    .push(DrawingItem::Line(self.last_point, self.first_point));
            }
            self.last_point = self.first_point;
            self.have_move = false;
            self.close_path = Some(false);
        } else {
            self.close_path = Some(true);
        }
    }
}

use super::operators::{color_code, format_g, ColorRole};
use super::{FinishOptions, Shape};
use crate::pdf::{DocOperation, PdfDocument, PdfPage};
use crate::{Error, Matrix, Point};

impl Shape<'_> {
    /// Finishes the currently accumulated drawing path and appends it to the total buffer.
    ///
    /// Equivalent of PyMuPDF `Shape.finish` for stroke/fill path painting options.
    ///
    /// ```
    /// use mupdf::{pdf::PdfDocument, shape::{FinishOptions, PdfColor, Shape}, Point, Size};
    ///
    /// # fn main() -> Result<(), mupdf::Error> {
    /// let mut doc = PdfDocument::new();
    /// let mut page = doc.new_page(Size::A4)?;
    /// let mut shape = Shape::new(&mut page)?;
    /// shape
    ///     .draw_line(Point::new(72.0, 72.0), Point::new(180.0, 72.0))?
    ///     .finish(&FinishOptions {
    ///         color: Some(PdfColor::rgb(1.0, 0.0, 0.0)),
    ///         width: 2.0,
    ///         ..Default::default()
    ///     })?
    ///     .commit(&mut doc, true)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn finish(&mut self, opts: &FinishOptions) -> Result<&mut Self, Error> {
        if self.draw_cont.is_empty() {
            return Ok(self);
        }

        validate_finish_scalars(opts)?;
        PdfPage::validate_opacity_pair(opts.stroke_opacity, opts.fill_opacity)?;
        if let Some(oc_xref) = opts.oc {
            let doc = self.page.document_handle()?;
            PdfPage::validate_optional_content_xref(&doc, oc_xref)?;
        }

        let (oc_name, opacity_name) = {
            let mut doc = self.page.document_handle()?;
            let oc_name = opts
                .oc
                .map(|oc_xref| self.page.register_optional_content(&mut doc, oc_xref))
                .transpose()?;
            let opacity_name =
                self.page
                    .register_ext_gstate(&mut doc, opts.stroke_opacity, opts.fill_opacity)?;
            (oc_name, opacity_name)
        };

        let mut block = String::new();
        block.push_str("q\n");
        if let Some((fixpoint, matrix)) = &opts.morph {
            if matrix != &Matrix::IDENTITY {
                block.push_str(&cm_operator(&morph_matrix(
                    *fixpoint,
                    matrix,
                    &self.pctm,
                    &self.ipctm,
                )));
            }
        }
        if let Some(oc_name) = &oc_name {
            block.push_str(&format!("/OC {oc_name} BDC\n"));
        }
        if let Some(opacity_name) = &opacity_name {
            block.push_str(&format!("{opacity_name} gs\n"));
        }
        block.push_str(&self.draw_cont);
        block.push_str(&format!("{} w\n", format_g(opts.width)));

        if let Some(line_cap) = opts.line_cap {
            block.push_str(&format!("{line_cap} J\n"));
        }
        if let Some(line_join) = opts.line_join {
            block.push_str(&format!("{line_join} j\n"));
        }
        if let Some(miter_limit) = opts.miter_limit {
            block.push_str(&format!("{} M\n", format_g(miter_limit)));
        }
        if let Some(dashes) = &opts.dashes {
            let dashes = dashes.trim();
            if !dashes.is_empty() {
                block.push_str(dashes);
                block.push_str(" d\n");
            }
        }
        if let Some(color) = effective_stroke_color(opts) {
            block.push_str(&color_code(color.components(), ColorRole::Stroke)?);
        }
        if let Some(fill) = &opts.fill {
            block.push_str(&color_code(fill.components(), ColorRole::Fill)?);
        }
        if opts.close_path {
            block.push_str("h\n");
        }
        block.push_str(paint_operator(opts));
        block.push('\n');
        if oc_name.is_some() {
            block.push_str("EMC\n");
        }
        block.push_str("Q\n");

        self.total_cont.push_str(&block);
        self.draw_cont.clear();
        self.clear_path_state();
        Ok(self)
    }

    /// Commits accumulated shape content to the bound page as a new `/Contents` stream.
    ///
    /// Equivalent of PyMuPDF `Shape.commit`. When `overlay` is true, existing page contents are
    /// wrapped in a balanced `q` / `Q` graphics-state pair before the shape stream is appended.
    /// When `overlay` is false, the shape stream is inserted before existing page contents.
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
    ///     .finish(&FinishOptions::default())?;
    /// shape.commit(&mut doc, true)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn commit(&mut self, doc: &mut PdfDocument, overlay: bool) -> Result<(), Error> {
        self.page.assert_document_owner(doc);

        if self.total_cont.is_empty() && self.text_cont.is_empty() {
            self.draw_cont.clear();
            self.text_cont.clear();
            self.clear_path_state();
            return Ok(());
        }

        let mut bytes = Vec::with_capacity(self.total_cont.len() + self.text_cont.len());
        bytes.extend_from_slice(self.total_cont.as_bytes());
        bytes.extend_from_slice(self.text_cont.as_bytes());

        let operation = DocOperation::begin(doc, "Shape commit")?;
        if overlay {
            self.page
                .insert_contents_in_operation(operation.doc, b"q\n", false)?;
            self.page
                .insert_contents_in_operation(operation.doc, b"Q\n", true)?;
        }
        self.page
            .insert_contents_in_operation(operation.doc, &bytes, overlay)?;
        operation.commit()?;

        self.draw_cont.clear();
        self.text_cont.clear();
        self.total_cont.clear();
        self.clear_path_state();

        Ok(())
    }
}

fn morph_matrix(fixpoint: Point, matrix: &Matrix, pctm: &Matrix, ipctm: &Matrix) -> Matrix {
    let user_morph = Matrix::new_translate(-fixpoint.x, -fixpoint.y)
        * matrix
        * Matrix::new_translate(fixpoint.x, fixpoint.y);
    pctm * user_morph * ipctm
}

fn cm_operator(matrix: &Matrix) -> String {
    format!(
        "{} {} {} {} {} {} cm\n",
        format_g(matrix.a),
        format_g(matrix.b),
        format_g(matrix.c),
        format_g(matrix.d),
        format_g(matrix.e),
        format_g(matrix.f)
    )
}

fn paint_operator(opts: &FinishOptions) -> &'static str {
    match (
        effective_stroke_color(opts).is_some(),
        opts.fill.is_some(),
        opts.even_odd,
    ) {
        (true, true, true) => "B*",
        (true, true, false) => "B",
        (true, false, _) => "S",
        (false, true, true) => "f*",
        (false, true, false) => "f",
        (false, false, _) => "n",
    }
}

fn effective_stroke_color(opts: &FinishOptions) -> Option<&super::PdfColor> {
    (opts.width > 0.0).then_some(opts.color.as_ref()).flatten()
}

fn validate_finish_scalars(opts: &FinishOptions) -> Result<(), Error> {
    if !opts.width.is_finite() || opts.width < 0.0 {
        return Err(Error::InvalidArgument(
            "width must be a non-negative finite value".to_owned(),
        ));
    }
    if let Some(color) = effective_stroke_color(opts) {
        color.validate()?;
    }
    if let Some(fill) = &opts.fill {
        fill.validate()?;
    }
    if let Some(miter_limit) = opts.miter_limit {
        if !miter_limit.is_finite() || miter_limit < 0.0 {
            return Err(Error::InvalidArgument(
                "miter_limit must be a non-negative finite value".to_owned(),
            ));
        }
    }
    if let Some((fixpoint, matrix)) = &opts.morph {
        let values = [
            fixpoint.x, fixpoint.y, matrix.a, matrix.b, matrix.c, matrix.d, matrix.e, matrix.f,
        ];
        if !values.into_iter().all(f32::is_finite) {
            return Err(Error::InvalidArgument(
                "morph fixpoint and matrix must contain finite values".to_owned(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::{FinishOptions, PdfColor, Shape, TextOptions};
    use crate::pdf::{PdfDocument, PdfObject, PdfPage};
    use crate::{Colorspace, Error, Image, ImageFormat, Matrix, Point, Quad, Rect, Size};
    use std::path::Path;
    use std::str;

    fn finished_line(opts: &FinishOptions) -> String {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
        shape.pctm = Matrix::IDENTITY;
        shape.ipctm = Matrix::IDENTITY;

        shape
            .draw_line(Point::new(10.0, 20.0), Point::new(30.0, 40.0))
            .unwrap()
            .finish(opts)
            .unwrap();

        shape.total_cont().to_owned()
    }

    fn finish_rect(opts: &FinishOptions) -> String {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
        shape.pctm = Matrix::IDENTITY;
        shape.ipctm = Matrix::IDENTITY;

        shape
            .draw_rect(&Rect::new(10.0, 20.0, 40.0, 60.0))
            .unwrap()
            .finish(opts)
            .unwrap();

        shape.total_cont().to_owned()
    }

    fn contents_stream_bytes(page: &PdfPage) -> Vec<Vec<u8>> {
        let contents = page.contents().unwrap().unwrap();
        assert!(contents.is_array().unwrap());
        (0..contents.len().unwrap())
            .map(|index| {
                contents
                    .get_array(index as i32)
                    .unwrap()
                    .unwrap()
                    .read_stream()
                    .unwrap()
            })
            .collect()
    }

    fn render_page(page: &PdfPage) -> crate::Pixmap {
        page.to_pixmap(
            &Matrix::new_scale(1.0, 1.0),
            &Colorspace::device_rgb(),
            false,
            true,
        )
        .unwrap()
    }

    fn assert_pixmaps_equal(actual: &crate::Pixmap, expected: &crate::Pixmap) {
        assert_eq!(actual.width(), expected.width());
        assert_eq!(actual.height(), expected.height());
        assert_eq!(actual.n(), expected.n());
        assert_eq!(actual.samples(), expected.samples());
    }

    fn assert_snapshot(snapshot: &str, rendered: &crate::Pixmap) {
        if std::env::var_os("UPDATE_SHAPE_SNAPSHOTS").is_some() {
            rendered.save_as(snapshot, ImageFormat::PNG).unwrap();
        }

        assert!(
            Path::new(snapshot).exists(),
            "missing snapshot {snapshot}; rerun with UPDATE_SHAPE_SNAPSHOTS=1"
        );
        let expected = Image::from_file(snapshot).unwrap().to_pixmap().unwrap();
        assert_pixmaps_equal(rendered, &expected);
    }

    fn page_ext_gstates(page: &PdfPage) -> Option<PdfObject> {
        page.object()
            .get_dict("Resources")
            .unwrap()
            .and_then(|resources| resources.get_dict("ExtGState").unwrap())
    }

    fn page_properties(page: &PdfPage) -> Option<PdfObject> {
        page.object()
            .get_dict("Resources")
            .unwrap()
            .and_then(|resources| resources.get_dict("Properties").unwrap())
    }

    fn properties_entries(page: &PdfPage) -> Vec<(String, PdfObject)> {
        let properties = page_properties(page).expect("Properties resource dict");
        (0..properties.dict_len().unwrap())
            .map(|index| {
                let key = properties.get_dict_key(index as i32).unwrap().unwrap();
                let key = str::from_utf8(key.as_name().unwrap()).unwrap().to_owned();
                let value = properties.get_dict_val(index as i32).unwrap().unwrap();
                (key, value)
            })
            .collect()
    }

    fn ext_gstate_entries(page: &PdfPage) -> Vec<(String, PdfObject)> {
        let ext_gstates = page_ext_gstates(page).expect("ExtGState resource dict");
        (0..ext_gstates.dict_len().unwrap())
            .map(|index| {
                let key = ext_gstates.get_dict_key(index as i32).unwrap().unwrap();
                let key = str::from_utf8(key.as_name().unwrap()).unwrap().to_owned();
                let value = ext_gstates.get_dict_val(index as i32).unwrap().unwrap();
                (key, value)
            })
            .collect()
    }

    fn fixpoint_matrix(fixpoint: Point, matrix: Matrix) -> Matrix {
        Matrix::new_translate(-fixpoint.x, -fixpoint.y)
            * matrix
            * Matrix::new_translate(fixpoint.x, fixpoint.y)
    }

    fn add_ocg(doc: &mut PdfDocument, name: &str) -> i32 {
        let mut ocg = doc.new_dict_with_capacity(2).unwrap();
        ocg.dict_put("Type", PdfObject::new_name("OCG").unwrap())
            .unwrap();
        ocg.dict_put("Name", PdfObject::new_string(name).unwrap())
            .unwrap();
        doc.add_object(&ocg).unwrap().as_indirect().unwrap()
    }

    fn add_indirect_string(doc: &mut PdfDocument, value: &str) -> i32 {
        let string = PdfObject::new_string(value).unwrap();
        doc.add_object(&string).unwrap().as_indirect().unwrap()
    }

    #[test]
    fn finish_default_stroke_wrapping() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
        shape.ipctm = Matrix::IDENTITY;

        shape
            .draw_line(Point::new(10.0, 20.0), Point::new(30.0, 40.0))
            .unwrap()
            .finish(&FinishOptions::default())
            .unwrap();

        assert_eq!(
            shape.total_cont(),
            "q\n10 20 m\n30 40 l\n1 w\n0 0 0 RG\nh\nS\nQ\n"
        );
        assert!(shape.draw_cont().is_empty());
        assert_eq!(shape.last_point(), None);
        assert_eq!(shape.rect(), None);
    }

    #[test]
    fn morph_identity_matrix_is_noop() {
        let without_morph = finish_rect(&FinishOptions::default());
        let with_identity_morph = finish_rect(&FinishOptions {
            morph: Some((Point::new(50.0, 50.0), Matrix::IDENTITY)),
            ..Default::default()
        });

        assert_eq!(with_identity_morph, without_morph);
        assert!(!with_identity_morph.contains(" cm\n"));
    }

    #[test]
    fn morph_non_identity_prepends_cm_operator() {
        let total_cont = finish_rect(&FinishOptions {
            morph: Some((Point::new(50.0, 50.0), Matrix::new_rotate(90.0))),
            ..Default::default()
        });

        assert_eq!(
            total_cont,
            "q\n0 1 -1 0 100 0 cm\n10 20 30 40 re\n1 w\n0 0 0 RG\nh\nS\nQ\n"
        );
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn morph_rotation_renders_rotated_shape() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let rendered = {
            let mut shape = Shape::new(&mut page).unwrap();
            shape
                .draw_rect(&Rect::new(80.0, 95.0, 120.0, 105.0))
                .unwrap()
                .finish(&FinishOptions {
                    color: None,
                    fill: Some(PdfColor::rgb(1.0, 0.0, 0.0)),
                    morph: Some((Point::new(100.0, 100.0), Matrix::new_rotate(90.0))),
                    ..Default::default()
                })
                .unwrap()
                .commit(&mut doc, true)
                .unwrap();
            render_page(shape.page())
        };

        assert_snapshot("tests/shape/snapshots/morph_rotated_rect.png", &rendered);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn morph_scale_about_fixpoint() {
        let morphed = {
            let mut doc = PdfDocument::new();
            let mut page = doc.new_page(Size::A4).unwrap();
            let mut shape = Shape::new(&mut page).unwrap();
            shape
                .draw_circle(Point::new(100.0, 100.0), 20.0)
                .unwrap()
                .finish(&FinishOptions {
                    color: None,
                    fill: Some(PdfColor::rgb(0.0, 0.0, 1.0)),
                    morph: Some((Point::new(100.0, 100.0), Matrix::new_scale(2.0, 2.0))),
                    ..Default::default()
                })
                .unwrap()
                .commit(&mut doc, true)
                .unwrap();
            render_page(shape.page())
        };

        let manual = {
            let mut doc = PdfDocument::new();
            let mut page = doc.new_page(Size::A4).unwrap();
            let mut shape = Shape::new(&mut page).unwrap();
            shape
                .draw_circle(Point::new(100.0, 100.0), 40.0)
                .unwrap()
                .finish(&FinishOptions {
                    color: None,
                    fill: Some(PdfColor::rgb(0.0, 0.0, 1.0)),
                    ..Default::default()
                })
                .unwrap()
                .commit(&mut doc, true)
                .unwrap();
            render_page(shape.page())
        };

        assert_pixmaps_equal(&morphed, &manual);
    }

    #[test]
    fn morph_does_not_leak_into_subsequent_finish() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
        shape.ipctm = Matrix::IDENTITY;

        shape
            .draw_rect(&Rect::new(80.0, 95.0, 120.0, 105.0))
            .unwrap()
            .finish(&FinishOptions {
                morph: Some((Point::new(100.0, 100.0), Matrix::new_rotate(90.0))),
                ..Default::default()
            })
            .unwrap()
            .draw_circle(Point::new(200.0, 200.0), 20.0)
            .unwrap()
            .finish(&FinishOptions::default())
            .unwrap();

        let total_cont = shape.total_cont();
        assert_eq!(total_cont.matches(" cm\n").count(), 1);

        let mut blocks = total_cont.split("Q\nq\n");
        assert!(blocks.next().unwrap().contains(" cm\n"));
        assert!(!blocks.next().unwrap().contains(" cm\n"));
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn morph_forty_five_degree_rect_matches_manual_transform() {
        let fixpoint = Point::new(100.0, 100.0);
        let rotation = Matrix::new_rotate(45.0);
        let morphed = {
            let mut doc = PdfDocument::new();
            let mut page = doc.new_page(Size::A4).unwrap();
            let mut shape = Shape::new(&mut page).unwrap();
            shape
                .draw_rect(&Rect::new(80.0, 80.0, 120.0, 120.0))
                .unwrap()
                .finish(&FinishOptions {
                    color: None,
                    fill: Some(PdfColor::rgb(0.0, 0.5, 0.0)),
                    morph: Some((fixpoint, rotation.clone())),
                    ..Default::default()
                })
                .unwrap()
                .commit(&mut doc, true)
                .unwrap();
            render_page(shape.page())
        };

        let manual = {
            let transform = fixpoint_matrix(fixpoint, rotation);
            let rect = Rect::new(80.0, 80.0, 120.0, 120.0);
            let mut doc = PdfDocument::new();
            let mut page = doc.new_page(Size::A4).unwrap();
            let mut shape = Shape::new(&mut page).unwrap();
            shape
                .draw_quad(Quad::new(
                    rect.tl().mul_matrix(&transform),
                    rect.tr().mul_matrix(&transform),
                    rect.bl().mul_matrix(&transform),
                    rect.br().mul_matrix(&transform),
                ))
                .unwrap()
                .finish(&FinishOptions {
                    color: None,
                    fill: Some(PdfColor::rgb(0.0, 0.5, 0.0)),
                    ..Default::default()
                })
                .unwrap()
                .commit(&mut doc, true)
                .unwrap();
            render_page(shape.page())
        };

        assert_pixmaps_equal(&morphed, &manual);
    }

    #[test]
    fn finish_paint_operator_matrix() {
        let cases = [
            (
                FinishOptions {
                    color: Some(PdfColor::rgb(1.0, 0.0, 0.0)),
                    fill: None,
                    ..Default::default()
                },
                "S\nQ\n",
            ),
            (
                FinishOptions {
                    color: None,
                    fill: Some(PdfColor::rgb(0.0, 1.0, 0.0)),
                    even_odd: false,
                    ..Default::default()
                },
                "f\nQ\n",
            ),
            (
                FinishOptions {
                    color: None,
                    fill: Some(PdfColor::rgb(0.0, 1.0, 0.0)),
                    even_odd: true,
                    ..Default::default()
                },
                "f*\nQ\n",
            ),
            (
                FinishOptions {
                    color: Some(PdfColor::rgb(1.0, 0.0, 0.0)),
                    fill: Some(PdfColor::rgb(0.0, 1.0, 0.0)),
                    even_odd: false,
                    ..Default::default()
                },
                "B\nQ\n",
            ),
            (
                FinishOptions {
                    color: Some(PdfColor::rgb(1.0, 0.0, 0.0)),
                    fill: Some(PdfColor::rgb(0.0, 1.0, 0.0)),
                    even_odd: true,
                    ..Default::default()
                },
                "B*\nQ\n",
            ),
            (
                FinishOptions {
                    color: None,
                    fill: None,
                    ..Default::default()
                },
                "n\nQ\n",
            ),
        ];

        for (opts, expected_tail) in cases {
            let total_cont = finish_rect(&opts);
            assert!(
                total_cont.ends_with(expected_tail),
                "content did not end with {expected_tail:?}: {total_cont:?}"
            );
        }
    }

    #[test]
    fn finish_close_path_toggle() {
        let closed = finished_line(&FinishOptions {
            close_path: true,
            ..Default::default()
        });
        assert!(closed.ends_with("h\nS\nQ\n"));

        let open = finished_line(&FinishOptions {
            close_path: false,
            ..Default::default()
        });
        assert!(open.ends_with("S\nQ\n"));
        assert!(!open.contains("h\nS\n"));
    }

    #[test]
    fn finish_emits_width_caps_joins_and_miter() {
        let total_cont = finished_line(&FinishOptions {
            width: 2.5,
            line_cap: Some(1),
            line_join: Some(2),
            miter_limit: Some(10.0),
            ..Default::default()
        });

        assert!(total_cont.contains("2.5 w\n"));
        assert!(total_cont.contains("1 J\n"));
        assert!(total_cont.contains("2 j\n"));
        assert!(total_cont.contains("10 M\n"));
    }

    #[test]
    fn finish_emits_dash_pattern() {
        let with_dashes = finished_line(&FinishOptions {
            dashes: Some("[3 2] 0".to_owned()),
            ..Default::default()
        });
        assert!(with_dashes.contains("[3 2] 0 d\n"));

        let without_dashes = finished_line(&FinishOptions {
            dashes: None,
            ..Default::default()
        });
        assert!(!without_dashes.contains(" d\n"));
    }

    #[test]
    fn finish_color_serialization_1_3_4_components() {
        let rgb = finished_line(&FinishOptions {
            color: Some(PdfColor::rgb(1.0, 0.0, 0.0)),
            fill: Some(PdfColor::rgb(0.0, 1.0, 0.0)),
            ..Default::default()
        });
        assert!(rgb.contains("1 0 0 RG\n"));
        assert!(rgb.contains("0 1 0 rg\n"));

        let gray = finished_line(&FinishOptions {
            color: Some(PdfColor::gray(0.5)),
            fill: Some(PdfColor::gray(0.25)),
            ..Default::default()
        });
        assert!(gray.contains("0.5 G\n"));
        assert!(gray.contains("0.25 g\n"));

        let cmyk = finished_line(&FinishOptions {
            color: Some(PdfColor::cmyk(0.1, 0.2, 0.3, 0.4)),
            fill: Some(PdfColor::cmyk(0.4, 0.3, 0.2, 0.1)),
            ..Default::default()
        });
        assert!(cmyk.contains("0.1 0.2 0.3 0.4 K\n"));
        assert!(cmyk.contains("0.4 0.3 0.2 0.1 k\n"));
    }

    #[test]
    fn finish_zero_width_disables_stroking() {
        let filled = finish_rect(&FinishOptions {
            color: Some(PdfColor::rgb(1.0, 0.0, 0.0)),
            fill: Some(PdfColor::rgb(0.0, 1.0, 0.0)),
            width: 0.0,
            ..Default::default()
        });

        assert!(filled.contains("0 w\n"));
        assert!(!filled.contains(" RG\n"));
        assert!(filled.ends_with("0 1 0 rg\nh\nf\nQ\n"));

        let stroke_only = finished_line(&FinishOptions {
            color: Some(PdfColor::rgb(1.0, 0.0, 0.0)),
            width: 0.0,
            ..Default::default()
        });
        assert!(!stroke_only.contains(" RG\n"));
        assert!(stroke_only.ends_with("0 w\nh\nn\nQ\n"));

        let ignored_invalid_stroke = finished_line(&FinishOptions {
            color: Some(PdfColor::rgb(2.0, 0.0, 0.0)),
            width: 0.0,
            ..Default::default()
        });
        assert!(ignored_invalid_stroke.ends_with("0 w\nh\nn\nQ\n"));
    }

    #[test]
    fn finish_rejects_invalid_colors() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();

        let err = shape
            .draw_line(Point::new(10.0, 20.0), Point::new(30.0, 40.0))
            .unwrap()
            .finish(&FinishOptions {
                color: Some(PdfColor::rgb(2.0, 0.0, 0.0)),
                ..Default::default()
            })
            .unwrap_err();

        assert!(matches!(err, Error::InvalidArgument(_)));
    }

    #[test]
    fn finish_without_draws_is_noop() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
        shape.ipctm = Matrix::IDENTITY;

        shape.finish(&FinishOptions::default()).unwrap();
        assert!(shape.draw_cont().is_empty());
        assert!(shape.total_cont().is_empty());

        shape
            .draw_line(Point::new(10.0, 20.0), Point::new(30.0, 40.0))
            .unwrap()
            .finish(&FinishOptions::default())
            .unwrap();
        assert_eq!(
            shape.total_cont(),
            "q\n10 20 m\n30 40 l\n1 w\n0 0 0 RG\nh\nS\nQ\n"
        );
    }

    #[test]
    fn finish_appends_not_replaces() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
        shape.ipctm = Matrix::IDENTITY;

        shape
            .draw_line(Point::new(10.0, 20.0), Point::new(30.0, 40.0))
            .unwrap()
            .finish(&FinishOptions::default())
            .unwrap()
            .draw_line(Point::new(50.0, 60.0), Point::new(70.0, 80.0))
            .unwrap()
            .finish(&FinishOptions::default())
            .unwrap();

        assert_eq!(
            shape.total_cont(),
            concat!(
                "q\n10 20 m\n30 40 l\n1 w\n0 0 0 RG\nh\nS\nQ\n",
                "q\n50 60 m\n70 80 l\n1 w\n0 0 0 RG\nh\nS\nQ\n"
            )
        );
    }

    mod oc {
        use super::*;

        fn array_contains_xref(array: &PdfObject, xref: i32) -> bool {
            (0..array.len().unwrap()).any(|index| {
                array
                    .get_array(index as i32)
                    .unwrap()
                    .is_some_and(|item| item.as_indirect().unwrap_or_default() == xref)
            })
        }

        #[test]
        fn finish_wraps_bdc_emc() {
            let mut doc = PdfDocument::new();
            let oc_xref = add_ocg(&mut doc, "Layer 1");
            let mut page = doc.new_page(Size::A4).unwrap();
            let mut shape = Shape::new(&mut page).unwrap();
            shape.ipctm = Matrix::IDENTITY;

            shape
                .draw_line(Point::new(10.0, 20.0), Point::new(30.0, 40.0))
                .unwrap()
                .finish(&FinishOptions {
                    oc: Some(oc_xref),
                    ..Default::default()
                })
                .unwrap();

            let entries = properties_entries(shape.page());
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].1.as_indirect().unwrap(), oc_xref);
            assert_eq!(
                shape.total_cont(),
                format!(
                    "q\n/OC /{} BDC\n10 20 m\n30 40 l\n1 w\n0 0 0 RG\nh\nS\nEMC\nQ\n",
                    entries[0].0
                )
            );
        }

        #[test]
        fn finish_registers_ocg_in_catalog_ocproperties() {
            let mut doc = PdfDocument::new();
            let oc_xref = add_ocg(&mut doc, "Catalog Layer");
            let mut page = doc.new_page(Size::A4).unwrap();
            let mut shape = Shape::new(&mut page).unwrap();
            shape.ipctm = Matrix::IDENTITY;

            shape
                .draw_line(Point::new(10.0, 20.0), Point::new(30.0, 40.0))
                .unwrap()
                .finish(&FinishOptions {
                    oc: Some(oc_xref),
                    ..Default::default()
                })
                .unwrap();

            let catalog = doc.catalog().unwrap();
            let oc_properties = catalog.get_dict("OCProperties").unwrap().unwrap();
            let ocgs = oc_properties.get_dict("OCGs").unwrap().unwrap();
            let default_config = oc_properties.get_dict("D").unwrap().unwrap();
            let on = default_config.get_dict("ON").unwrap().unwrap();
            let order = default_config.get_dict("Order").unwrap().unwrap();

            assert!(array_contains_xref(&ocgs, oc_xref));
            assert!(array_contains_xref(&on, oc_xref));
            assert!(array_contains_xref(&order, oc_xref));
        }

        #[test]
        fn finish_completes_existing_catalog_ocproperties() {
            let mut doc = PdfDocument::new();
            let oc_xref = add_ocg(&mut doc, "Partial Catalog Layer");
            let oc_ref = doc.new_indirect(oc_xref, 0).unwrap();
            let mut ocgs = doc.new_array().unwrap();
            ocgs.array_push_ref(&oc_ref).unwrap();
            let mut oc_properties = doc.new_dict().unwrap();
            oc_properties.dict_put_ref("OCGs", &ocgs).unwrap();
            doc.catalog()
                .unwrap()
                .dict_put_ref("OCProperties", &oc_properties)
                .unwrap();

            let mut page = doc.new_page(Size::A4).unwrap();
            let mut shape = Shape::new(&mut page).unwrap();
            shape.ipctm = Matrix::IDENTITY;

            shape
                .draw_line(Point::new(10.0, 20.0), Point::new(30.0, 40.0))
                .unwrap()
                .finish(&FinishOptions {
                    oc: Some(oc_xref),
                    ..Default::default()
                })
                .unwrap();

            let catalog = doc.catalog().unwrap();
            let oc_properties = catalog.get_dict("OCProperties").unwrap().unwrap();
            let ocgs = oc_properties.get_dict("OCGs").unwrap().unwrap();
            let default_config = oc_properties.get_dict("D").unwrap().unwrap();
            let on = default_config.get_dict("ON").unwrap().unwrap();
            let order = default_config.get_dict("Order").unwrap().unwrap();

            assert!(array_contains_xref(&ocgs, oc_xref));
            assert!(array_contains_xref(&on, oc_xref));
            assert!(array_contains_xref(&order, oc_xref));
        }

        #[test]
        fn text_wraps_bdc_emc() {
            let mut doc = PdfDocument::new();
            let oc_xref = add_ocg(&mut doc, "Text Layer");
            let mut page = doc.new_page(Size::new(600.0, 800.0)).unwrap();
            let mut shape = Shape::new(&mut page).unwrap();

            shape
                .insert_text(
                    Point::new(50.0, 100.0),
                    "Hi",
                    &TextOptions {
                        oc: Some(oc_xref),
                        ..Default::default()
                    },
                )
                .unwrap();

            let entries = properties_entries(shape.page());
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].1.as_indirect().unwrap(), oc_xref);
            assert_eq!(
                shape.text_cont(),
                format!(
                    "q\n/OC /{} BDC\nBT\n1 0 0 1 50 700 Tm\n/F0 11 Tf\n[<4869>] TJ\nET\nEMC\nQ\n",
                    entries[0].0
                )
            );
        }

        #[test]
        fn idempotent_properties_slot() {
            let mut doc = PdfDocument::new();
            let oc_xref = add_ocg(&mut doc, "Shared Layer");
            let mut page = doc.new_page(Size::A4).unwrap();
            let mut shape = Shape::new(&mut page).unwrap();
            shape.ipctm = Matrix::IDENTITY;
            let opts = FinishOptions {
                oc: Some(oc_xref),
                ..Default::default()
            };

            shape
                .draw_line(Point::new(10.0, 20.0), Point::new(30.0, 40.0))
                .unwrap()
                .finish(&opts)
                .unwrap()
                .draw_line(Point::new(50.0, 60.0), Point::new(70.0, 80.0))
                .unwrap()
                .finish(&opts)
                .unwrap();

            let entries = properties_entries(shape.page());
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].1.as_indirect().unwrap(), oc_xref);
            assert_eq!(
                shape
                    .total_cont()
                    .matches(&format!("/OC /{} BDC\n", entries[0].0))
                    .count(),
                2
            );
        }

        #[test]
        fn invalid_oc_xref_errors() {
            let mut doc = PdfDocument::new();
            let invalid_xref = add_indirect_string(&mut doc, "not an OCG dictionary");
            let mut page = doc.new_page(Size::A4).unwrap();
            let mut shape = Shape::new(&mut page).unwrap();
            shape.ipctm = Matrix::IDENTITY;
            shape
                .draw_line(Point::new(10.0, 20.0), Point::new(30.0, 40.0))
                .unwrap();
            let draw_before = shape.draw_cont().to_owned();

            let result = shape.finish(&FinishOptions {
                oc: Some(invalid_xref),
                ..Default::default()
            });

            assert!(result.is_err());
            assert_eq!(shape.draw_cont(), draw_before);
            assert!(shape.total_cont().is_empty());
            assert!(page_properties(shape.page()).is_none());

            let mut doc = PdfDocument::new();
            let invalid_xref = add_indirect_string(&mut doc, "not an OCG dictionary");
            let mut page = doc.new_page(Size::new(600.0, 800.0)).unwrap();
            let mut shape = Shape::new(&mut page).unwrap();

            let result = shape.insert_text(
                Point::new(50.0, 100.0),
                "Hi",
                &TextOptions {
                    oc: Some(invalid_xref),
                    ..Default::default()
                },
            );

            assert!(result.is_err());
            assert!(shape.text_cont().is_empty());
            assert!(page_properties(shape.page()).is_none());
        }
    }

    mod opacity {
        use super::*;

        #[test]
        fn stroke_opacity_registers_extgstate() {
            let mut doc = PdfDocument::new();
            let mut page = doc.new_page(Size::A4).unwrap();
            let mut shape = Shape::new(&mut page).unwrap();
            shape.ipctm = Matrix::IDENTITY;

            shape
                .draw_line(Point::new(10.0, 20.0), Point::new(30.0, 40.0))
                .unwrap()
                .finish(&FinishOptions {
                    stroke_opacity: Some(0.5),
                    ..Default::default()
                })
                .unwrap();

            let entries = ext_gstate_entries(shape.page());
            assert_eq!(entries.len(), 1);
            assert!(entries[0].0.starts_with('A'));
            assert_eq!(
                entries[0]
                    .1
                    .get_dict("CA")
                    .unwrap()
                    .unwrap()
                    .as_float()
                    .unwrap(),
                0.5
            );
            assert!(entries[0].1.get_dict("ca").unwrap().is_none());
            assert!(shape
                .total_cont()
                .contains(&format!("/{} gs\n", entries[0].0)));
        }

        #[test]
        fn fill_opacity_registers_extgstate() {
            let mut doc = PdfDocument::new();
            let mut page = doc.new_page(Size::A4).unwrap();
            let mut shape = Shape::new(&mut page).unwrap();
            shape.ipctm = Matrix::IDENTITY;

            shape
                .draw_rect(&Rect::new(10.0, 20.0, 40.0, 60.0))
                .unwrap()
                .finish(&FinishOptions {
                    color: None,
                    fill: Some(PdfColor::rgb(1.0, 0.0, 0.0)),
                    fill_opacity: Some(0.25),
                    ..Default::default()
                })
                .unwrap();

            let entries = ext_gstate_entries(shape.page());
            assert_eq!(entries.len(), 1);
            assert!(entries[0].1.get_dict("CA").unwrap().is_none());
            assert_eq!(
                entries[0]
                    .1
                    .get_dict("ca")
                    .unwrap()
                    .unwrap()
                    .as_float()
                    .unwrap(),
                0.25
            );
            assert!(shape
                .total_cont()
                .contains(&format!("/{} gs\n", entries[0].0)));
        }

        #[test]
        fn combined_opacity_single_extgstate() {
            let mut doc = PdfDocument::new();
            let mut page = doc.new_page(Size::A4).unwrap();
            let mut shape = Shape::new(&mut page).unwrap();
            shape.ipctm = Matrix::IDENTITY;

            shape
                .draw_rect(&Rect::new(10.0, 20.0, 40.0, 60.0))
                .unwrap()
                .finish(&FinishOptions {
                    fill: Some(PdfColor::rgb(0.0, 1.0, 0.0)),
                    stroke_opacity: Some(0.5),
                    fill_opacity: Some(0.5),
                    ..Default::default()
                })
                .unwrap();

            let entries = ext_gstate_entries(shape.page());
            assert_eq!(entries.len(), 1);
            assert_eq!(
                entries[0]
                    .1
                    .get_dict("CA")
                    .unwrap()
                    .unwrap()
                    .as_float()
                    .unwrap(),
                0.5
            );
            assert_eq!(
                entries[0]
                    .1
                    .get_dict("ca")
                    .unwrap()
                    .unwrap()
                    .as_float()
                    .unwrap(),
                0.5
            );
            assert_eq!(shape.total_cont().matches(" gs\n").count(), 1);
        }

        #[test]
        fn idempotent_extgstate_registration() {
            let mut doc = PdfDocument::new();
            let mut page = doc.new_page(Size::A4).unwrap();
            let mut shape = Shape::new(&mut page).unwrap();
            shape.ipctm = Matrix::IDENTITY;
            let opts = FinishOptions {
                stroke_opacity: Some(0.5),
                fill_opacity: Some(0.25),
                ..Default::default()
            };

            shape
                .draw_line(Point::new(10.0, 20.0), Point::new(30.0, 40.0))
                .unwrap()
                .finish(&opts)
                .unwrap()
                .draw_line(Point::new(50.0, 60.0), Point::new(70.0, 80.0))
                .unwrap()
                .finish(&opts)
                .unwrap();

            let entries = ext_gstate_entries(shape.page());
            assert_eq!(entries.len(), 1);
            assert_eq!(
                shape
                    .total_cont()
                    .matches(&format!("/{} gs\n", entries[0].0))
                    .count(),
                2
            );
        }

        #[test]
        fn opacity_out_of_range_errors() {
            for opts in [
                FinishOptions {
                    stroke_opacity: Some(1.5),
                    ..Default::default()
                },
                FinishOptions {
                    fill_opacity: Some(-0.1),
                    ..Default::default()
                },
                FinishOptions {
                    stroke_opacity: Some(f32::NAN),
                    ..Default::default()
                },
            ] {
                let mut doc = PdfDocument::new();
                let mut page = doc.new_page(Size::A4).unwrap();
                let mut shape = Shape::new(&mut page).unwrap();
                shape.ipctm = Matrix::IDENTITY;
                shape
                    .draw_line(Point::new(10.0, 20.0), Point::new(30.0, 40.0))
                    .unwrap();
                let draw_before = shape.draw_cont().to_owned();

                let result = shape.finish(&opts);

                assert!(result.is_err());
                assert_eq!(shape.draw_cont(), draw_before);
                assert!(shape.total_cont().is_empty());
                assert!(page_ext_gstates(shape.page()).is_none());
            }
        }

        #[test]
        fn text_opacity_registers_extgstate() {
            let mut doc = PdfDocument::new();
            let mut page = doc.new_page(Size::A4).unwrap();
            let mut shape = Shape::new(&mut page).unwrap();
            shape.ipctm = Matrix::IDENTITY;

            shape
                .draw_line(Point::new(10.0, 20.0), Point::new(30.0, 40.0))
                .unwrap()
                .finish(&FinishOptions {
                    fill: Some(PdfColor::rgb(1.0, 0.0, 0.0)),
                    fill_opacity: Some(0.5),
                    stroke_opacity: Some(0.5),
                    ..Default::default()
                })
                .unwrap();

            shape
                .insert_text(
                    Point::new(50.0, 100.0),
                    "Hi",
                    &TextOptions {
                        fill_opacity: Some(0.5),
                        stroke_opacity: Some(0.5),
                        ..Default::default()
                    },
                )
                .unwrap();

            let entries = ext_gstate_entries(shape.page());
            assert_eq!(entries.len(), 1);
            assert_eq!(
                entries[0]
                    .1
                    .get_dict("CA")
                    .unwrap()
                    .unwrap()
                    .as_float()
                    .unwrap(),
                0.5
            );
            assert_eq!(
                entries[0]
                    .1
                    .get_dict("ca")
                    .unwrap()
                    .unwrap()
                    .as_float()
                    .unwrap(),
                0.5
            );
            let gs = format!("/{} gs\n", entries[0].0);
            let text_cont = shape.text_cont();
            let gs_index = text_cont.find(&gs).expect("gs operator");
            let bt_index = text_cont.find("BT\n").expect("BT operator");
            let tf_index = text_cont.find(" Tf\n").expect("Tf operator");
            assert!(gs_index < bt_index);
            assert!(bt_index < tf_index);
        }
    }

    #[test]
    fn commit_overlay_appends_stream() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
        shape.ipctm = Matrix::IDENTITY;

        shape
            .draw_line(Point::new(10.0, 20.0), Point::new(30.0, 40.0))
            .unwrap()
            .finish(&FinishOptions::default())
            .unwrap();
        let expected = shape.total_cont().as_bytes().to_vec();

        shape.commit(&mut doc, true).unwrap();

        let stream_bytes = contents_stream_bytes(shape.page());
        assert_eq!(stream_bytes.last().unwrap(), &expected);
        assert!(shape.draw_cont().is_empty());
        assert!(shape.text_cont().is_empty());
        assert!(shape.total_cont().is_empty());
        assert_eq!(shape.last_point(), None);
        assert_eq!(shape.rect(), None);
    }

    #[test]
    fn commit_underlay_prepends_stream() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        page.insert_contents(&mut doc, b"original\n", true).unwrap();

        let mut shape = Shape::new(&mut page).unwrap();
        shape.ipctm = Matrix::IDENTITY;
        shape
            .draw_line(Point::new(10.0, 20.0), Point::new(30.0, 40.0))
            .unwrap()
            .finish(&FinishOptions::default())
            .unwrap();
        let expected = shape.total_cont().as_bytes().to_vec();

        shape.commit(&mut doc, false).unwrap();

        let stream_bytes = contents_stream_bytes(shape.page());
        assert_eq!(stream_bytes, vec![expected, b"original\n".to_vec()]);
    }

    #[test]
    fn commit_overlay_wraps_existing() {
        let mut doc =
            PdfDocument::from_bytes(include_bytes!("../../tests/files/dummy.pdf")).unwrap();
        let mut page = PdfPage::try_from(doc.load_page(0).unwrap()).unwrap();
        let original = page.contents().unwrap().unwrap();
        let original_bytes = original.read_stream().unwrap();

        let mut shape = Shape::new(&mut page).unwrap();
        shape.ipctm = Matrix::IDENTITY;
        shape
            .draw_line(Point::new(10.0, 20.0), Point::new(30.0, 40.0))
            .unwrap()
            .finish(&FinishOptions::default())
            .unwrap();
        let expected_shape = shape.total_cont().as_bytes().to_vec();

        shape.commit(&mut doc, true).unwrap();

        let stream_bytes = contents_stream_bytes(shape.page());
        assert_eq!(stream_bytes.len(), 4);
        assert_eq!(stream_bytes[0], b"q\n");
        assert_eq!(stream_bytes[1], original_bytes);
        assert_eq!(stream_bytes[2], b"Q\n");
        assert_eq!(stream_bytes[3], expected_shape);
    }

    #[test]
    fn commit_empty_is_noop() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
        shape.draw_cont.push_str("10 20 m\n30 40 l\n");
        shape.set_last_point(Point::new(30.0, 40.0));

        shape.commit(&mut doc, true).unwrap();

        assert!(shape.page().contents().unwrap().is_none());
        assert!(shape.draw_cont().is_empty());
        assert!(shape.text_cont().is_empty());
        assert!(shape.total_cont().is_empty());
    }

    #[test]
    fn commit_wrong_document_panics_before_mutating_buffers() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
        shape.total_cont.push_str("draw\n");
        shape.text_cont.push_str("text\n");
        let mut other_doc = PdfDocument::new();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            shape.commit(&mut other_doc, true).unwrap();
        }));

        assert!(result.is_err());
        assert_eq!(shape.total_cont(), "draw\n");
        assert_eq!(shape.text_cont(), "text\n");
    }

    #[test]
    fn commit_appends_text_content_before_writing() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
        shape.total_cont.push_str("draw\n");
        shape.text_cont.push_str("text\n");

        shape.commit(&mut doc, false).unwrap();

        let stream_bytes = contents_stream_bytes(shape.page());
        assert_eq!(stream_bytes, vec![b"draw\ntext\n".to_vec()]);
        assert!(shape.draw_cont().is_empty());
        assert!(shape.text_cont().is_empty());
        assert!(shape.total_cont().is_empty());
    }

    #[test]
    fn commit_repeated_appends_each_time() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
        shape.ipctm = Matrix::IDENTITY;

        shape
            .draw_line(Point::new(10.0, 20.0), Point::new(30.0, 40.0))
            .unwrap()
            .finish(&FinishOptions::default())
            .unwrap();
        let first = shape.total_cont().as_bytes().to_vec();
        shape.commit(&mut doc, true).unwrap();
        assert!(shape.total_cont().is_empty());

        shape
            .draw_line(Point::new(50.0, 60.0), Point::new(70.0, 80.0))
            .unwrap()
            .finish(&FinishOptions::default())
            .unwrap();
        let second = shape.total_cont().as_bytes().to_vec();
        shape.commit(&mut doc, true).unwrap();

        let stream_bytes = contents_stream_bytes(shape.page());
        let first_index = stream_bytes
            .iter()
            .position(|bytes| bytes == &first)
            .unwrap();
        let second_index = stream_bytes
            .iter()
            .position(|bytes| bytes == &second)
            .unwrap();
        assert!(first_index < second_index);
        assert_eq!(
            stream_bytes.iter().filter(|bytes| *bytes == &first).count(),
            1
        );
        assert_eq!(
            stream_bytes
                .iter()
                .filter(|bytes| *bytes == &second)
                .count(),
            1
        );
        assert!(shape.draw_cont().is_empty());
        assert!(shape.text_cont().is_empty());
        assert!(shape.total_cont().is_empty());
    }
}

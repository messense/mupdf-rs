use super::operators::{color_code, format_g, ColorRole};
use super::{FinishOptions, Shape};
use crate::pdf::PdfDocument;
use crate::{Buffer, Error, Matrix, Point};

impl Shape<'_> {
    /// Finishes the currently accumulated drawing path and appends it to the total buffer.
    ///
    /// Equivalent of PyMuPDF `Shape.finish` for stroke/fill path painting options.
    pub fn finish(&mut self, opts: &FinishOptions) -> Result<&mut Self, Error> {
        if self.draw_cont.is_empty() {
            return Ok(self);
        }

        let opacity_name = {
            let mut doc = self.page.document_handle()?;
            self.page
                .register_ext_gstate(&mut doc, opts.stroke_opacity, opts.fill_opacity)?
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
        if let Some(color) = &opts.color {
            block.push_str(&color_code(color.components(), ColorRole::Stroke));
        }
        if let Some(fill) = &opts.fill {
            block.push_str(&color_code(fill.components(), ColorRole::Fill));
        }
        if opts.close_path {
            block.push_str("h\n");
        }
        block.push_str(paint_operator(opts));
        block.push_str("\nQ\n");

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
    pub fn commit(&mut self, doc: &mut PdfDocument, overlay: bool) -> Result<(), Error> {
        if !self.text_cont.is_empty() {
            self.total_cont.push_str(&self.text_cont);
        }

        if self.total_cont.is_empty() {
            self.draw_cont.clear();
            self.text_cont.clear();
            self.clear_path_state();
            return Ok(());
        }

        let bytes = self.total_cont.as_bytes().to_vec();
        if overlay {
            self.page.wrap_contents(doc)?;
        }

        let xref = self.page.insert_contents(doc, &bytes, overlay)?;
        let mut stream = doc.new_indirect(xref, 0)?;
        let buffer = Buffer::from_bytes(&bytes)?;
        stream.write_stream_buffer(&buffer)?;

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
    match (opts.color.is_some(), opts.fill.is_some(), opts.even_odd) {
        (true, true, true) => "B*",
        (true, true, false) => "B",
        (true, false, _) => "S",
        (false, true, true) => "f*",
        (false, true, false) => "f",
        (false, false, _) => "n",
    }
}

#[cfg(test)]
mod tests {
    use super::super::{FinishOptions, PdfColor, Shape, TextOptions};
    use crate::pdf::{PdfDocument, PdfObject, PdfPage};
    use crate::{Colorspace, Image, ImageFormat, Matrix, Point, Quad, Rect, Size};
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
            "q\n10 20 m\n30 40 l\n1 w\n0 0 0 RG\nS\nQ\n"
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
            "q\n0 1 -1 0 100 0 cm\n10 20 30 40 re\n1 w\n0 0 0 RG\nS\nQ\n"
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

        assert_snapshot("tests/shape/snapshots/m3_morph_rotated_rect.png", &rendered);
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
            "q\n10 20 m\n30 40 l\n1 w\n0 0 0 RG\nS\nQ\n"
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
                "q\n10 20 m\n30 40 l\n1 w\n0 0 0 RG\nS\nQ\n",
                "q\n50 60 m\n70 80 l\n1 w\n0 0 0 RG\nS\nQ\n"
            )
        );
    }

    mod m5 {
        pub mod opacity {
            use super::super::*;

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
                assert!(bt_index < gs_index);
                assert!(gs_index < tf_index);
            }
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
            PdfDocument::from_bytes(include_bytes!("../../../tests/files/dummy.pdf")).unwrap();
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

use super::operators::{color_code, format_g, ColorRole};
use super::{FinishOptions, Shape};
use crate::Error;

impl Shape<'_> {
    /// Finishes the currently accumulated drawing path and appends it to the total buffer.
    ///
    /// Equivalent of PyMuPDF `Shape.finish` for stroke/fill path painting options.
    pub fn finish(&mut self, opts: &FinishOptions) -> Result<&mut Self, Error> {
        if self.draw_cont.is_empty() {
            return Ok(self);
        }

        let mut block = String::new();
        block.push_str("q\n");
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
    use super::super::{FinishOptions, PdfColor, Shape};
    use crate::pdf::PdfDocument;
    use crate::{Matrix, Point, Rect, Size};

    fn finished_line(opts: &FinishOptions) -> String {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::A4).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
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
        shape.ipctm = Matrix::IDENTITY;

        shape
            .draw_rect(&Rect::new(10.0, 20.0, 40.0, 60.0))
            .unwrap()
            .finish(opts)
            .unwrap();

        shape.total_cont().to_owned()
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
}

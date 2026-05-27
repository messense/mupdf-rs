use super::operators::{color_code, format_g, tj_str, ColorRole};
use super::{Shape, TextOptions};
use crate::pdf::InsertFontOptions;
use crate::{Error, Point};

#[derive(Clone, Copy, Debug, PartialEq)]
struct TextMatrix {
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
}

impl Shape<'_> {
    /// Inserts text at `point`.
    ///
    /// Equivalent of PyMuPDF `Shape.insert_text` for point text. Text is emitted as a
    /// single PDF text object using `Tm` and `TJ`; newline-separated input creates one
    /// `TJ` operation per line. Only text rotations of 0, 90, 180, and 270 degrees are
    /// supported.
    pub fn insert_text(
        &mut self,
        point: Point,
        text: &str,
        opts: &TextOptions,
    ) -> Result<&mut Self, Error> {
        if text.is_empty() {
            return Ok(self);
        }

        let rotate = normalize_rotate(opts.rotate)?;
        let lines = text.lines().collect::<Vec<_>>();
        if lines.is_empty() {
            return Ok(self);
        }

        let (font_name, font_info) = {
            let mut doc = self.page.document_handle()?;
            let font_opts = InsertFontOptions {
                name: opts.fontname.trim_start_matches('/'),
                fontfile: None,
                simple: opts.simple,
                encoding: opts.encoding,
                ..InsertFontOptions::new(opts.fontname.trim_start_matches('/'))
            };
            let (font_name, _xref, font_info) = self.page.insert_font(&mut doc, &font_opts)?;
            (font_name, font_info)
        };

        let origin = point.mul_matrix(&self.ipctm);
        let line_advance = opts.fontsize * opts.lineheight;
        let mut block = String::new();
        block.push_str("q\nBT\n");

        if opts.render_mode != 0 {
            block.push_str(&format!("{} Tr\n", opts.render_mode));
            block.push_str(&format!(
                "{} w\n",
                format_g(opts.border_width * opts.fontsize)
            ));
            if let Some(miter_limit) = opts.miter_limit {
                block.push_str(&format!("{} M\n", format_g(miter_limit)));
            }
        }

        if let Some(color) = &opts.color {
            block.push_str(&color_code(color.components(), ColorRole::Stroke));
        }
        let fill = opts.fill.as_ref().or(opts.color.as_ref());
        if let Some(fill) = fill {
            block.push_str(&color_code(fill.components(), ColorRole::Fill));
        }

        for (line_index, line) in lines.iter().enumerate() {
            let matrix = text_matrix(rotate, origin, line_advance * line_index as f32);
            block.push_str(&format!(
                "{} {} {} {} {} {} Tm\n",
                format_g(matrix.a),
                format_g(matrix.b),
                format_g(matrix.c),
                format_g(matrix.d),
                format_g(matrix.e),
                format_g(matrix.f)
            ));
            block.push_str(&format!("{font_name} {} Tf\n", format_g(opts.fontsize)));
            block.push_str(&tj_str(line, &font_info));
            block.push_str(" TJ\n");
        }

        block.push_str("ET\nQ\n");
        self.text_cont.push_str(&block);
        Ok(self)
    }
}

fn normalize_rotate(rotate: i32) -> Result<i32, Error> {
    if matches!(rotate, 0 | 90 | 180 | 270) {
        return Ok(rotate);
    }

    Err(Error::InvalidArgument(format!(
        "bad rotate value: {rotate}; expected one of 0, 90, 180, 270"
    )))
}

fn text_matrix(rotate: i32, origin: Point, line_offset: f32) -> TextMatrix {
    let (a, b, c, d) = match rotate {
        0 => (1.0, 0.0, 0.0, 1.0),
        90 => (0.0, 1.0, -1.0, 0.0),
        180 => (-1.0, 0.0, 0.0, -1.0),
        270 => (0.0, -1.0, 1.0, 0.0),
        _ => unreachable!("rotate was normalized before building text matrix"),
    };

    TextMatrix {
        a,
        b,
        c,
        d,
        e: origin.x - c * line_offset,
        f: origin.y - d * line_offset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::PdfDocument;
    use crate::{PdfColor, Size};

    fn text_cont_for(text: &str, opts: &TextOptions) -> String {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::new(600.0, 800.0)).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
        shape
            .insert_text(Point::new(50.0, 100.0), text, opts)
            .unwrap();
        shape.text_cont().to_owned()
    }

    #[test]
    fn insert_text_default_emits_bt_et_default_font_tm_and_tj() {
        let text_cont = text_cont_for("Hi", &TextOptions::default());

        assert_eq!(
            text_cont,
            "q\nBT\n1 0 0 1 50 700 Tm\n/F0 11 Tf\n[<4869>] TJ\nET\nQ\n"
        );
    }

    #[test]
    fn insert_text_multiline_emits_one_tj_per_line_with_lineheight_spacing() {
        let text_cont = text_cont_for(
            "line1\nline2",
            &TextOptions {
                fontsize: 10.0,
                lineheight: 1.5,
                ..Default::default()
            },
        );

        assert!(text_cont.contains("1 0 0 1 50 700 Tm\n/F0 10 Tf\n[<6c696e6531>] TJ\n"));
        assert!(text_cont.contains("1 0 0 1 50 685 Tm\n/F0 10 Tf\n[<6c696e6532>] TJ\n"));
        assert_eq!(text_cont.matches(" TJ\n").count(), 2);
    }

    #[test]
    fn insert_text_rotation_matrices_anchor_at_transformed_point() {
        for (rotate, expected_tm) in [
            (0, "1 0 0 1 50 700 Tm\n"),
            (90, "0 1 -1 0 50 700 Tm\n"),
            (180, "-1 0 0 -1 50 700 Tm\n"),
            (270, "0 -1 1 0 50 700 Tm\n"),
        ] {
            let text_cont = text_cont_for(
                "R",
                &TextOptions {
                    rotate,
                    ..Default::default()
                },
            );
            assert!(
                text_cont.contains(expected_tm),
                "rotate {rotate} text_cont:\n{text_cont}"
            );
        }
    }

    #[test]
    fn insert_text_rejects_non_right_angle_rotation_without_appending_content() {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::new(600.0, 800.0)).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
        let result = shape.insert_text(
            Point::new(50.0, 100.0),
            "bad",
            &TextOptions {
                rotate: 45,
                ..Default::default()
            },
        );

        assert!(result.is_err());
        assert!(shape.text_cont().is_empty());
    }

    #[test]
    fn insert_text_rejects_rotation_outside_supported_quadrants() {
        for rotate in [-90, 360, 450] {
            let mut doc = PdfDocument::new();
            let mut page = doc.new_page(Size::new(600.0, 800.0)).unwrap();
            let mut shape = Shape::new(&mut page).unwrap();

            let result = shape.insert_text(
                Point::new(50.0, 100.0),
                "bad",
                &TextOptions {
                    rotate,
                    ..Default::default()
                },
            );

            assert!(result.is_err(), "rotate {rotate} unexpectedly succeeded");
            assert!(
                shape.text_cont().is_empty(),
                "rotate {rotate} appended content"
            );
        }
    }

    #[test]
    fn insert_text_color_fill_render_mode_border_width_and_miter_limit() {
        let text_cont = text_cont_for(
            "Hi",
            &TextOptions {
                fontsize: 10.0,
                color: Some(PdfColor::rgb(1.0, 0.0, 0.0)),
                render_mode: 1,
                border_width: 0.2,
                miter_limit: Some(2.0),
                ..Default::default()
            },
        );

        assert!(text_cont.contains("1 Tr\n"));
        assert!(text_cont.contains("2 w\n"));
        assert!(text_cont.contains("2 M\n"));
        assert!(text_cont.contains("1 0 0 RG\n"));
        assert!(text_cont.contains("1 0 0 rg\n"));
    }

    #[test]
    fn insert_text_empty_input_is_noop() {
        let text_cont = text_cont_for("", &TextOptions::default());

        assert!(text_cont.is_empty());
    }

    #[test]
    fn insert_text_latin1_round_trips_through_tj_operand() {
        let text_cont = text_cont_for("café", &TextOptions::default());

        assert!(text_cont.contains("[<636166e9>] TJ"));
    }

    #[test]
    fn insert_text_accepts_very_small_fontsize_with_format_g() {
        let text_cont = text_cont_for(
            "tiny",
            &TextOptions {
                fontsize: 0.001,
                ..Default::default()
            },
        );

        assert!(text_cont.contains("/F0 0.001 Tf\n"));
    }
}

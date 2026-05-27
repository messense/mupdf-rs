use super::operators::{color_code, format_g, tj_str, ColorRole};
use super::{Shape, TextAlign, TextOptions, TextboxOptions};
use crate::pdf::{InsertFontOptions, PdfPage};
use crate::{Error, Font, Point, Rect};

#[derive(Clone, Copy, Debug, PartialEq)]
struct TextMatrix {
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TextboxLine {
    text: String,
    is_paragraph_last: bool,
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

        PdfPage::validate_opacity_pair(opts.stroke_opacity, opts.fill_opacity)?;

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
        let opacity_name = {
            let mut doc = self.page.document_handle()?;
            self.page
                .register_ext_gstate(&mut doc, opts.stroke_opacity, opts.fill_opacity)?
        };

        let origin = point.mul_matrix(&self.ipctm);
        let line_advance = opts.fontsize * opts.lineheight;
        let mut block = String::new();
        block.push_str("q\nBT\n");
        if let Some(opacity_name) = &opacity_name {
            block.push_str(&format!("{opacity_name} gs\n"));
        }

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

    /// Inserts word-wrapped text into `rect`.
    ///
    /// Equivalent of PyMuPDF `Shape.insert_textbox` for the M4 feature set. The text
    /// is wrapped by word into the supplied rectangle and emitted as one PDF text
    /// object. Left, center, right, and justified alignment are supported. Justified
    /// lines distribute extra width across inter-word gaps with PDF word spacing; the
    /// last line of each paragraph and single-word lines stay left-aligned. The return
    /// value is the unused height when all lines fit, or a negative deficit of
    /// `missing_lines * fontsize * lineheight` when text overflows.
    pub fn insert_textbox(
        &mut self,
        rect: Rect,
        text: &str,
        opts: &TextboxOptions,
    ) -> Result<f32, Error> {
        let rotate = normalize_rotate(opts.rotate)?;
        let rect = normalize_textbox_rect(rect);
        if ![rect.x0, rect.y0, rect.x1, rect.y1]
            .into_iter()
            .all(f32::is_finite)
            || rect.width() <= 0.0
            || rect.height() <= 0.0
        {
            return Err(Error::InvalidArgument(
                "text box must be finite and not empty".to_owned(),
            ));
        }

        if text.is_empty() {
            return Ok(rect.height());
        }
        if !opts.fontsize.is_finite() || opts.fontsize <= 0.0 {
            return Err(Error::InvalidArgument(
                "fontsize must be a positive finite value".to_owned(),
            ));
        }
        if !opts.lineheight.is_finite() || opts.lineheight <= 0.0 {
            return Err(Error::InvalidArgument(
                "lineheight must be a positive finite value".to_owned(),
            ));
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
        let font = Font::new(&font_info.name)?;

        let max_width = textbox_line_width(rect, rotate);
        let max_height = textbox_line_capacity_height(rect, rotate);
        let line_advance = opts.fontsize * opts.lineheight;
        let lines = wrap_textbox_lines(text, max_width, opts.fontsize, &font, &font_info)?;
        if lines.is_empty() {
            return Ok(rect.height());
        }

        let fitting_lines = ((max_height + f32::EPSILON) / line_advance)
            .floor()
            .max(0.0) as usize;
        let lines_to_emit = fitting_lines.min(lines.len());
        let missing_lines = lines.len().saturating_sub(fitting_lines);
        let deficit = if missing_lines > 0 {
            -(missing_lines as f32 * line_advance)
        } else {
            max_height - lines.len() as f32 * line_advance
        };

        if lines_to_emit == 0 {
            return Ok(deficit);
        }

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

        for (line_index, line) in lines.iter().take(lines_to_emit).enumerate() {
            let line_width = text_width(&line.text, opts.fontsize, &font, &font_info)?;
            let align_offset = textbox_align_offset(max_width, line_width, opts.align);
            let point = textbox_line_point(
                rect,
                rotate,
                opts.fontsize * font_info.ascender,
                line_advance,
                line_index,
                align_offset,
            );
            let origin = point.mul_matrix(&self.ipctm);
            let matrix = text_matrix(rotate, origin, 0.0);
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
            let word_spacing = if opts.align == TextAlign::Justify {
                textbox_justify_word_spacing(line, max_width, line_width)
            } else {
                None
            };
            if let Some(word_spacing) = word_spacing {
                block.push_str(&format!("{} Tw\n", format_g(word_spacing)));
            }
            block.push_str(&tj_str(&line.text, &font_info));
            block.push_str(" TJ\n");
            if word_spacing.is_some() {
                block.push_str("0 Tw\n");
            }
        }

        block.push_str("ET\nQ\n");
        self.text_cont.push_str(&block);
        self.update_rect_with_rect(rect);
        Ok(deficit)
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

fn normalize_textbox_rect(rect: Rect) -> Rect {
    Rect::new(
        rect.x0.min(rect.x1),
        rect.y0.min(rect.y1),
        rect.x0.max(rect.x1),
        rect.y0.max(rect.y1),
    )
}

fn textbox_line_width(rect: Rect, rotate: i32) -> f32 {
    if matches!(rotate, 90 | 270) {
        rect.height()
    } else {
        rect.width()
    }
}

fn textbox_line_capacity_height(rect: Rect, rotate: i32) -> f32 {
    if matches!(rotate, 90 | 270) {
        rect.width()
    } else {
        rect.height()
    }
}

fn textbox_align_offset(max_width: f32, line_width: f32, align: TextAlign) -> f32 {
    let remaining = max_width - line_width;
    match align {
        TextAlign::Left | TextAlign::Justify => 0.0,
        TextAlign::Center => remaining / 2.0,
        TextAlign::Right => remaining,
    }
}

fn textbox_justify_word_spacing(
    line: &TextboxLine,
    max_width: f32,
    line_width: f32,
) -> Option<f32> {
    if line.is_paragraph_last {
        return None;
    }

    let gaps = line.text.matches(' ').count();
    if gaps == 0 {
        return None;
    }

    let extra_width = (max_width - line_width).max(0.0);
    if extra_width <= f32::EPSILON {
        return None;
    }

    Some(extra_width / gaps as f32)
}

fn textbox_line_point(
    rect: Rect,
    rotate: i32,
    ascender_offset: f32,
    line_advance: f32,
    line_index: usize,
    align_offset: f32,
) -> Point {
    let line_offset = line_advance * line_index as f32;
    match rotate {
        0 => Point::new(
            rect.x0 + align_offset,
            rect.y0 + ascender_offset + line_offset,
        ),
        90 => Point::new(
            rect.x0 + ascender_offset + line_offset,
            rect.y1 - align_offset,
        ),
        180 => Point::new(
            rect.x1 - align_offset,
            rect.y1 - ascender_offset - line_offset,
        ),
        270 => Point::new(
            rect.x1 - ascender_offset - line_offset,
            rect.y0 + align_offset,
        ),
        _ => unreachable!("rotate was normalized before building textbox point"),
    }
}

fn wrap_textbox_lines(
    text: &str,
    max_width: f32,
    fontsize: f32,
    font: &Font,
    font_info: &crate::pdf::FontInfo,
) -> Result<Vec<TextboxLine>, Error> {
    let mut lines = Vec::new();

    for paragraph in text.split('\n') {
        let words = paragraph.split_whitespace().collect::<Vec<_>>();
        if words.is_empty() {
            lines.push(TextboxLine {
                text: String::new(),
                is_paragraph_last: true,
            });
            continue;
        }

        let mut current = String::new();
        for word in words {
            if current.is_empty() {
                current.push_str(word);
                continue;
            }

            let candidate = format!("{current} {word}");
            if text_width(&candidate, fontsize, font, font_info)? <= max_width {
                current = candidate;
            } else {
                lines.push(TextboxLine {
                    text: current,
                    is_paragraph_last: false,
                });
                current = word.to_owned();
            }
        }

        lines.push(TextboxLine {
            text: current,
            is_paragraph_last: true,
        });
    }

    Ok(lines)
}

fn text_width(
    text: &str,
    fontsize: f32,
    font: &Font,
    font_info: &crate::pdf::FontInfo,
) -> Result<f32, Error> {
    if font_info.ordering.is_some() {
        return Ok(text.chars().count() as f32 * fontsize);
    }

    let mut width = 0.0;
    for ch in text.chars() {
        let code = ch as u32;
        let glyph = font_info
            .glyphs
            .as_ref()
            .and_then(|glyphs| glyphs.get(&code))
            .copied()
            .or_else(|| {
                let code = if font_info.simple && code > 255 {
                    0xb7
                } else {
                    code
                };
                font.encode_character(code as i32).ok()
            })
            .unwrap_or(code as i32);
        width += font.advance_glyph(glyph)? * fontsize;
    }
    Ok(width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::{FontInfo, PdfDocument};
    use crate::{PdfColor, Rect, SimpleFontEncoding, Size, WriteMode};

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

    fn insert_textbox_on_test_page(
        rect: Rect,
        text: &str,
        opts: &TextboxOptions,
    ) -> (Result<f32, Error>, String) {
        let mut doc = PdfDocument::new();
        let mut page = doc.new_page(Size::new(600.0, 800.0)).unwrap();
        let mut shape = Shape::new(&mut page).unwrap();
        let result = shape.insert_textbox(rect, text, opts);
        (result, shape.text_cont().to_owned())
    }

    #[test]
    fn insert_textbox_empty_input_returns_full_rect_height_and_noops() {
        let rect = Rect::new(50.0, 100.0, 250.0, 160.0);
        let (result, text_cont) = insert_textbox_on_test_page(rect, "", &TextboxOptions::default());

        assert_eq!(result.unwrap(), rect.height());
        assert!(text_cont.is_empty());
    }

    #[test]
    fn insert_textbox_rejects_non_right_angle_rotation_without_appending_content() {
        let (result, text_cont) = insert_textbox_on_test_page(
            Rect::new(50.0, 100.0, 250.0, 160.0),
            "bad",
            &TextboxOptions {
                rotate: 45,
                ..Default::default()
            },
        );

        assert!(result.is_err());
        assert!(text_cont.is_empty());
    }

    #[test]
    fn insert_textbox_justify_align_succeeds_and_emits_text() {
        let (result, text_cont) = insert_textbox_on_test_page(
            Rect::new(50.0, 100.0, 250.0, 160.0),
            "lorem ipsum dolor sit amet",
            &TextboxOptions {
                align: TextAlign::Justify,
                ..Default::default()
            },
        );

        assert!(result.unwrap() >= 0.0);
        assert!(!text_cont.is_empty());
        assert!(text_cont.contains(" TJ\n"));
    }

    #[test]
    fn insert_textbox_justify_word_spacing_balances_non_last_lines() {
        let rect = Rect::new(50.0, 100.0, 175.0, 180.0);
        let opts = TextboxOptions {
            fontsize: 12.0,
            lineheight: 1.0,
            align: TextAlign::Justify,
            ..Default::default()
        };
        let (result, text_cont) =
            insert_textbox_on_test_page(rect, "lorem ipsum dolor sit amet consectetur", &opts);

        assert!(result.unwrap() >= 0.0);
        let entries = textbox_text_entries(&text_cont);
        assert!(entries.len() >= 2, "expected wrapped lines:\n{text_cont}");

        let (line, word_spacing) = entries
            .iter()
            .find(|(line, word_spacing)| line.contains(' ') && word_spacing.is_some())
            .expect("expected at least one justified multi-word line");
        let word_spacing = word_spacing.unwrap();
        let font_info = test_helvetica_font_info();
        let font = Font::new("Helvetica").unwrap();
        let line_width = text_width(line, opts.fontsize, &font, &font_info).unwrap();
        let gaps = line.matches(' ').count();

        assert!(gaps > 0);
        assert!(
            (line_width + gaps as f32 * word_spacing - rect.width()).abs() <= 0.001,
            "line={line:?}, width={line_width}, word_spacing={word_spacing}, box={}",
            rect.width()
        );

        let last_line = entries.last().expect("at least one emitted line");
        assert_eq!(last_line.1, None, "last line must not be justified");
    }

    #[test]
    fn insert_textbox_justify_single_word_line_does_not_emit_word_spacing() {
        let (result, text_cont) = insert_textbox_on_test_page(
            Rect::new(50.0, 100.0, 120.0, 180.0),
            "supercalifragilistic",
            &TextboxOptions {
                fontsize: 12.0,
                lineheight: 1.0,
                align: TextAlign::Justify,
                ..Default::default()
            },
        );

        assert!(result.unwrap() >= 0.0);
        assert!(!text_cont.contains("nan"));
        assert!(!text_cont.contains("inf"));
        assert_eq!(
            textbox_text_entries(&text_cont),
            vec![("supercalifragilistic".to_owned(), None)]
        );
    }

    mod m5 {
        pub mod justify {
            use super::super::*;

            #[test]
            fn accepts_justify_align() {
                let (result, text_cont) = insert_textbox_on_test_page(
                    Rect::new(50.0, 100.0, 250.0, 160.0),
                    "lorem ipsum dolor sit amet",
                    &TextboxOptions {
                        align: TextAlign::Justify,
                        ..Default::default()
                    },
                );

                assert!(result.unwrap() >= 0.0);
                assert!(!text_cont.is_empty());
                assert!(text_cont.contains(" TJ\n"));
            }

            #[test]
            fn word_spacing_balances_lines() {
                let rect = Rect::new(50.0, 100.0, 175.0, 180.0);
                let opts = TextboxOptions {
                    fontsize: 12.0,
                    lineheight: 1.0,
                    align: TextAlign::Justify,
                    ..Default::default()
                };
                let (result, text_cont) = insert_textbox_on_test_page(
                    rect,
                    "lorem ipsum dolor sit amet consectetur",
                    &opts,
                );

                assert!(result.unwrap() >= 0.0);
                let entries = textbox_text_entries(&text_cont);
                assert!(entries.len() >= 2, "expected wrapped lines:\n{text_cont}");

                let (line, word_spacing) = entries
                    .iter()
                    .find(|(line, word_spacing)| line.contains(' ') && word_spacing.is_some())
                    .expect("expected at least one justified multi-word line");
                let word_spacing = word_spacing.unwrap();
                let font_info = test_helvetica_font_info();
                let font = Font::new("Helvetica").unwrap();
                let line_width = text_width(line, opts.fontsize, &font, &font_info).unwrap();
                let gaps = line.matches(' ').count();

                assert!(gaps > 0);
                assert!(
                    (line_width + gaps as f32 * word_spacing - rect.width()).abs() <= 0.001,
                    "line={line:?}, width={line_width}, word_spacing={word_spacing}, box={}",
                    rect.width()
                );

                let last_line = entries.last().expect("at least one emitted line");
                assert_eq!(last_line.1, None, "last line must not be justified");
            }

            #[test]
            fn single_word_line_safe() {
                let (result, text_cont) = insert_textbox_on_test_page(
                    Rect::new(50.0, 100.0, 120.0, 180.0),
                    "supercalifragilistic",
                    &TextboxOptions {
                        fontsize: 12.0,
                        lineheight: 1.0,
                        align: TextAlign::Justify,
                        ..Default::default()
                    },
                );

                assert!(result.unwrap() >= 0.0);
                assert!(!text_cont.contains("nan"));
                assert!(!text_cont.contains("inf"));
                assert_eq!(
                    textbox_text_entries(&text_cont),
                    vec![("supercalifragilistic".to_owned(), None)]
                );
            }
        }
    }

    #[test]
    fn insert_textbox_overflow_returns_missing_line_deficit_and_emits_fit_lines() {
        let (result, text_cont) = insert_textbox_on_test_page(
            Rect::new(50.0, 100.0, 250.0, 125.0),
            "line1\nline2\nline3",
            &TextboxOptions {
                fontsize: 10.0,
                lineheight: 1.0,
                ..Default::default()
            },
        );

        assert_eq!(result.unwrap(), -10.0);
        assert!(text_cont.contains("[<6c696e6531>] TJ"));
        assert!(text_cont.contains("[<6c696e6532>] TJ"));
        assert!(!text_cont.contains("[<6c696e6533>] TJ"));
    }

    #[test]
    fn insert_textbox_oversized_single_word_gets_its_own_line() {
        let (result, text_cont) = insert_textbox_on_test_page(
            Rect::new(50.0, 100.0, 60.0, 130.0),
            "supercalifragilistic",
            &TextboxOptions {
                fontsize: 10.0,
                lineheight: 1.0,
                ..Default::default()
            },
        );

        assert!(result.unwrap() >= 0.0);
        assert_eq!(text_cont.matches(" TJ\n").count(), 1);
        assert!(text_cont.contains("[<737570657263616c6966726167696c6973746963>] TJ"));
    }

    fn textbox_text_entries(text_cont: &str) -> Vec<(String, Option<f32>)> {
        let mut entries = Vec::new();
        let mut pending_word_spacing = None;

        for line in text_cont.lines() {
            if let Some(value) = line.strip_suffix(" Tw") {
                pending_word_spacing = value.parse::<f32>().ok().filter(|value| value.abs() > 1e-6);
                continue;
            }

            if line.ends_with(" TJ") {
                entries.push((decode_tj_line(line), pending_word_spacing.take()));
            }
        }

        entries
    }

    fn decode_tj_line(line: &str) -> String {
        let hex = line
            .strip_prefix("[<")
            .and_then(|line| line.strip_suffix(">] TJ"))
            .expect("simple TJ line");
        let bytes = (0..hex.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).unwrap())
            .collect::<Vec<_>>();
        String::from_utf8(bytes).unwrap()
    }

    fn test_helvetica_font_info() -> FontInfo {
        FontInfo {
            ascender: 1.0,
            descender: -0.2,
            glyphs: None,
            simple: true,
            ordering: None,
            name: "Helvetica".to_owned(),
            encoding: SimpleFontEncoding::Latin,
            wmode: WriteMode::Horizontal,
            serif: false,
            fontfile_hash: None,
        }
    }
}

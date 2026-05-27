use std::fs;
use std::path::{Path, PathBuf};

use mupdf::pdf::{PdfDocument, PdfObject, PdfPage};
use mupdf::{
    Colorspace, FinishOptions, ImageFormat, Matrix, PdfColor, Point, Quad, Rect, RectRadius, Shape,
    Size, TextAlign, TextOptions, TextboxOptions,
};

const CUSTOM_FONT_BYTES: &[u8] = include_bytes!("../tests/files/custom.ttf");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = output_dir();
    fs::create_dir_all(&output_dir)?;

    let doc = build_shape_demo_document()?;
    let pdf_path = output_dir.join("shape_demo.pdf");
    doc.save(&pdf_path.to_string_lossy())?;

    render_saved_pages(&pdf_path, &output_dir)?;
    println!("wrote {}", pdf_path.display());
    Ok(())
}

fn output_dir() -> PathBuf {
    std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("SHAPE_DEMO_OUTPUT_DIR").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("target/shape_demo"))
}

fn build_shape_demo_document() -> Result<PdfDocument, mupdf::Error> {
    let mut doc = PdfDocument::new();
    let oc_xref = add_ocg(&mut doc, "Shape demo optional content")?;

    let mut drawing_page = doc.new_page(Size::A4)?;
    draw_primitives_page(&mut doc, &mut drawing_page)?;

    let mut text_page = doc.new_page(Size::A4)?;
    draw_text_page(&mut doc, &mut text_page)?;

    let mut polish_page = doc.new_page(Size::A4)?;
    draw_polish_page(&mut doc, &mut polish_page, oc_xref)?;

    Ok(doc)
}

fn draw_primitives_page(doc: &mut PdfDocument, page: &mut PdfPage) -> Result<(), mupdf::Error> {
    let mut shape = Shape::new(page)?;
    shape
        .draw_line(Point::new(40.0, 48.0), Point::new(555.0, 48.0))?
        .draw_polyline(&[
            Point::new(60.0, 110.0),
            Point::new(120.0, 70.0),
            Point::new(180.0, 110.0),
            Point::new(60.0, 110.0),
        ])?
        .draw_rect(&Rect::new(220.0, 70.0, 330.0, 125.0))?
        .draw_rect_with_radius(
            &Rect::new(370.0, 70.0, 515.0, 130.0),
            RectRadius::absolute(12.0),
        )?
        .finish(&FinishOptions {
            color: Some(PdfColor::rgb(0.0, 0.0, 0.0)),
            fill: Some(PdfColor::rgb(0.92, 0.96, 1.0)),
            width: 1.5,
            close_path: false,
            ..Default::default()
        })?
        .draw_bezier(
            Point::new(60.0, 230.0),
            Point::new(110.0, 155.0),
            Point::new(180.0, 305.0),
            Point::new(230.0, 230.0),
        )?
        .draw_curve(
            Point::new(280.0, 230.0),
            Point::new(340.0, 150.0),
            Point::new(400.0, 230.0),
        )?
        .draw_sector(
            Point::new(495.0, 230.0),
            Point::new(540.0, 230.0),
            130.0,
            true,
        )?
        .finish(&FinishOptions {
            color: Some(PdfColor::rgb(0.75, 0.0, 0.0)),
            fill: Some(PdfColor::rgb(1.0, 0.88, 0.82)),
            width: 2.0,
            ..Default::default()
        })?
        .draw_circle(Point::new(105.0, 380.0), 38.0)?
        .draw_oval(Rect::new(190.0, 342.0, 320.0, 418.0))?
        .draw_quad(Quad::new(
            Point::new(385.0, 340.0),
            Point::new(520.0, 370.0),
            Point::new(360.0, 435.0),
            Point::new(495.0, 465.0),
        ))?
        .finish(&FinishOptions {
            color: Some(PdfColor::rgb(0.0, 0.35, 0.0)),
            fill: Some(PdfColor::rgb(0.86, 1.0, 0.86)),
            width: 2.0,
            ..Default::default()
        })?
        .draw_zigzag(Point::new(60.0, 570.0), Point::new(250.0, 570.0), 7.0)?
        .draw_squiggle(Point::new(315.0, 560.0), Point::new(535.0, 610.0), 7.0)?
        .finish(&FinishOptions {
            color: Some(PdfColor::rgb(0.1, 0.1, 0.7)),
            width: 1.5,
            ..Default::default()
        })?
        .commit(doc, true)?;
    Ok(())
}

fn draw_text_page(doc: &mut PdfDocument, page: &mut PdfPage) -> Result<(), mupdf::Error> {
    let mut shape = Shape::new(page)?;
    shape.insert_text(
        Point::new(72.0, 96.0),
        "Shape text at a point\nwith multiple lines",
        &TextOptions {
            fontsize: 24.0,
            lineheight: 1.15,
            fill: Some(PdfColor::rgb(0.0, 0.0, 0.0)),
            ..Default::default()
        },
    )?;
    shape.insert_textbox(
        Rect::new(72.0, 185.0, 300.0, 320.0),
        "Left aligned textbox wraps text into a rectangle using built-in font metrics.",
        &TextboxOptions {
            fontsize: 15.0,
            lineheight: 1.1,
            fill: Some(PdfColor::rgb(0.0, 0.2, 0.65)),
            ..Default::default()
        },
    )?;
    shape.insert_textbox(
        Rect::new(330.0, 185.0, 535.0, 320.0),
        "Centered textbox text demonstrates alternate alignment and word wrapping.",
        &TextboxOptions {
            fontsize: 15.0,
            lineheight: 1.1,
            align: TextAlign::Center,
            fill: Some(PdfColor::rgb(0.55, 0.0, 0.0)),
            ..Default::default()
        },
    )?;
    shape
        .insert_text(
            Point::new(92.0, 500.0),
            "Rotated text",
            &TextOptions {
                fontsize: 20.0,
                rotate: 90,
                fill: Some(PdfColor::rgb(0.0, 0.45, 0.0)),
                ..Default::default()
            },
        )?
        .commit(doc, true)?;
    Ok(())
}

fn draw_polish_page(
    doc: &mut PdfDocument,
    page: &mut PdfPage,
    oc_xref: i32,
) -> Result<(), mupdf::Error> {
    let mut shape = Shape::new(page)?;
    shape
        .draw_circle(Point::new(145.0, 150.0), 70.0)?
        .finish(&FinishOptions {
            color: Some(PdfColor::rgb(0.0, 0.0, 0.0)),
            fill: Some(PdfColor::rgb(1.0, 0.0, 0.0)),
            width: 2.0,
            fill_opacity: Some(0.45),
            oc: Some(oc_xref),
            ..Default::default()
        })?
        .draw_rect_with_radius(
            &Rect::new(250.0, 85.0, 510.0, 215.0),
            RectRadius::fractional(0.12, 0.25),
        )?
        .finish(&FinishOptions {
            color: Some(PdfColor::rgb(0.0, 0.0, 0.0)),
            fill: Some(PdfColor::rgb(0.0, 0.25, 1.0)),
            width: 2.0,
            fill_opacity: Some(0.45),
            oc: Some(oc_xref),
            ..Default::default()
        })?
        .insert_text(
            Point::new(72.0, 310.0),
            "Custom font + opacity + optional content",
            &TextOptions {
                fontsize: 26.0,
                fontname: "ShapeDemoCustom".to_owned(),
                fontfile: Some(CUSTOM_FONT_BYTES),
                fill: Some(PdfColor::rgb(0.1, 0.1, 0.1)),
                fill_opacity: Some(0.8),
                oc: Some(oc_xref),
                ..Default::default()
            },
        )?;
    shape.insert_textbox(
        Rect::new(72.0, 390.0, 525.0, 515.0),
        "Justified text spreads words across each non-final line while the final line stays left aligned.",
        &TextboxOptions {
            fontsize: 16.0,
            lineheight: 1.15,
            align: TextAlign::Justify,
            fill: Some(PdfColor::rgb(0.0, 0.0, 0.0)),
            ..Default::default()
        },
    )?;
    shape.commit(doc, true)?;
    Ok(())
}

fn add_ocg(doc: &mut PdfDocument, name: &str) -> Result<i32, mupdf::Error> {
    let mut ocg = doc.new_dict_with_capacity(2)?;
    ocg.dict_put("Type", PdfObject::new_name("OCG")?)?;
    ocg.dict_put("Name", PdfObject::new_string(name)?)?;
    doc.add_object(&ocg)?.as_indirect()
}

fn render_saved_pages(
    pdf_path: &Path,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read(pdf_path)?;
    let doc = PdfDocument::from_bytes(&bytes)?;
    for page_index in 0..doc.page_count()? {
        let page = PdfPage::try_from(doc.load_page(page_index)?)?;
        let pixmap = page.to_pixmap(
            &Matrix::new_scale(1.0, 1.0),
            &Colorspace::device_rgb(),
            false,
            true,
        )?;
        let png_path = output_dir.join(format!("shape_demo_page_{}.png", page_index + 1));
        pixmap.save_as(&png_path.to_string_lossy(), ImageFormat::PNG)?;
    }
    Ok(())
}

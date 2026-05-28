use mupdf::pdf::{PdfDocument, PdfPage};
use mupdf::{Colorspace, Matrix, Point, Rect, Shape, Size};

fn render_page(page: &PdfPage) -> mupdf::Pixmap {
    page.to_pixmap(
        &Matrix::new_scale(1.0, 1.0),
        &Colorspace::device_rgb(),
        false,
        true,
    )
    .unwrap()
}

fn pixel_rgb(pixmap: &mupdf::Pixmap, x: u32, y: u32) -> [u8; 3] {
    let n = pixmap.n() as usize;
    assert_eq!(n, 3);
    let index = ((y * pixmap.width() + x) as usize) * n;
    let samples = pixmap.samples();
    [samples[index], samples[index + 1], samples[index + 2]]
}

fn assert_blue(pixel: [u8; 3]) {
    assert!(
        pixel[0] <= 8 && pixel[1] <= 8 && pixel[2].abs_diff(255) <= 8,
        "expected blue stroke pixel, got {pixel:?}"
    );
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn m2_basic_drawing_renders_line_triangle_and_rectangle() {
    let mut doc = PdfDocument::new();
    let mut page = doc.new_page(Size::A4).unwrap();

    let draw_cont = {
        let mut shape = Shape::new(&mut page).unwrap();
        shape
            .draw_line(Point::new(50.0, 50.0), Point::new(250.0, 50.0))
            .unwrap()
            .draw_rect(&Rect::new(50.0, 100.0, 200.0, 180.0))
            .unwrap()
            .draw_polyline(&[
                Point::new(250.0, 150.0),
                Point::new(350.0, 250.0),
                Point::new(450.0, 150.0),
                Point::new(250.0, 150.0),
            ])
            .unwrap();
        shape.draw_cont().to_owned()
    };

    let content = format!("q\n0 0 1 RG\n6 w\n{draw_cont}S\nQ\n");
    page.insert_contents(&mut doc, content.as_bytes(), true)
        .unwrap();

    let rendered = render_page(&page);

    assert_blue(pixel_rgb(&rendered, 150, 50));
    assert_blue(pixel_rgb(&rendered, 125, 100));
    assert_blue(pixel_rgb(&rendered, 350, 150));
}

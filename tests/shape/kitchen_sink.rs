#![cfg(not(target_arch = "wasm32"))]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use crate::support::{assert_snapshot, render_page};
use mupdf::pdf::{PdfDocument, PdfObject, PdfPage};
use mupdf::shape::{
    FinishOptions, PdfColor, RectRadius, Shape, TextAlign, TextOptions, TextboxOptions,
};
use mupdf::{Image, Point, Rect, Size, TextPageFlags};

const CUSTOM_FONT_BYTES: &[u8] = include_bytes!("../files/custom.ttf");
const SHAPE_DEMO_FIRST_PAGE_SNAPSHOT: &str = "tests/shape/snapshots/shape_demo.png";
const CROSS_KITCHEN_SINK_SNAPSHOT: &str = "tests/shape/snapshots/cross_kitchen_sink.png";
const MULTIPAGE_SNAPSHOTS: [&str; 3] = [
    "tests/shape/snapshots/multipage_p1.png",
    "tests/shape/snapshots/multipage_p2.png",
    "tests/shape/snapshots/multipage_p3.png",
];

static CARGO_COMMAND_LOCK: Mutex<()> = Mutex::new(());

fn fresh_output_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("shape-kitchen-sink")
        .join(format!("{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn cargo_command() -> Command {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);
    command.current_dir(env!("CARGO_MANIFEST_DIR"));
    command
}

fn run_shape_demo(output_dir: &Path) {
    let _guard = CARGO_COMMAND_LOCK.lock().unwrap();
    let status = cargo_command()
        .arg("run")
        .arg("--example")
        .arg("shape_demo")
        .arg("--")
        .arg(output_dir)
        .status()
        .unwrap();
    assert!(status.success(), "shape_demo example failed: {status}");
}

fn build_shape_demo_example(output_dir: &Path) {
    let _guard = CARGO_COMMAND_LOCK.lock().unwrap();
    let status = cargo_command()
        .arg("build")
        .arg("--example")
        .arg("shape_demo")
        .env("SHAPE_DEMO_OUTPUT_DIR", output_dir)
        .status()
        .unwrap();
    assert!(
        status.success(),
        "shape_demo example did not compile: {status}"
    );
}

fn render_pdf_bytes(bytes: &[u8]) -> Vec<mupdf::Pixmap> {
    let doc = PdfDocument::from_bytes(bytes).unwrap();
    assert_eq!(doc.page_count().unwrap(), 3);
    (0..3)
        .map(|page_index| {
            let page = PdfPage::try_from(doc.load_page(page_index).unwrap()).unwrap();
            render_page(&page)
        })
        .collect()
}

fn pixel_rgb(pixmap: &mupdf::Pixmap, x: u32, y: u32) -> [u8; 3] {
    let n = pixmap.n() as usize;
    assert_eq!(n, 3);
    let index = ((y * pixmap.width() + x) as usize) * n;
    let samples = pixmap.samples();
    [samples[index], samples[index + 1], samples[index + 2]]
}

fn find_dark_pixel(pixmap: &mupdf::Pixmap) -> (u32, u32, [u8; 3]) {
    for y in 0..pixmap.height() {
        for x in 0..pixmap.width() {
            let pixel = pixel_rgb(pixmap, x, y);
            if pixel.iter().all(|channel| *channel <= 8) {
                return (x, y, pixel);
            }
        }
    }
    panic!("no solid dark pixel found in baseline render");
}

fn find_white_pixel(pixmap: &mupdf::Pixmap) -> (u32, u32) {
    for y in 0..pixmap.height() {
        for x in 0..pixmap.width() {
            let pixel = pixel_rgb(pixmap, x, y);
            if pixel.iter().all(|channel| *channel >= 250) {
                return (x, y);
            }
        }
    }
    panic!("no white pixel found in baseline render");
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

fn add_ocg(doc: &mut PdfDocument, name: &str) -> i32 {
    let mut ocg = doc.new_dict_with_capacity(2).unwrap();
    ocg.dict_put("Type", PdfObject::new_name("OCG").unwrap())
        .unwrap();
    ocg.dict_put("Name", PdfObject::new_string(name).unwrap())
        .unwrap();
    doc.add_object(&ocg).unwrap().as_indirect().unwrap()
}

fn non_wrapper_streams(streams: &[Vec<u8>]) -> Vec<&[u8]> {
    streams
        .iter()
        .map(Vec::as_slice)
        .filter(|bytes| *bytes != b"q\n" && *bytes != b"Q\n")
        .collect()
}

fn build_cross_kitchen_sink() -> (Vec<u8>, mupdf::Pixmap, Vec<Vec<u8>>) {
    let mut doc = PdfDocument::new();
    let oc_xref = add_ocg(&mut doc, "Cross kitchen sink");
    let mut page = doc.new_page(Size::A4).unwrap();
    let (rendered, streams) = {
        let mut shape = Shape::new(&mut page).unwrap();
        shape
            .draw_circle(Point::new(160.0, 160.0), 70.0)
            .unwrap()
            .finish(&FinishOptions {
                color: Some(PdfColor::rgb(0.0, 0.0, 0.0)),
                fill: Some(PdfColor::rgb(1.0, 0.0, 0.0)),
                width: 2.0,
                fill_opacity: Some(0.5),
                oc: Some(oc_xref),
                ..Default::default()
            })
            .unwrap()
            .insert_text(
                Point::new(90.0, 285.0),
                "Cross milestone",
                &TextOptions {
                    fontsize: 28.0,
                    fill: Some(PdfColor::rgb(0.0, 0.0, 0.0)),
                    ..Default::default()
                },
            )
            .unwrap()
            .draw_rect_with_radius(
                &Rect::new(80.0, 345.0, 430.0, 465.0),
                RectRadius::fractional(0.12, 0.25),
            )
            .unwrap()
            .finish(&FinishOptions {
                color: Some(PdfColor::rgb(0.0, 0.0, 0.0)),
                fill: Some(PdfColor::rgb(0.0, 0.35, 1.0)),
                width: 2.0,
                fill_opacity: Some(0.5),
                oc: Some(oc_xref),
                ..Default::default()
            })
            .unwrap()
            .commit(&mut doc, true)
            .unwrap();
        (
            render_page(shape.page()),
            contents_stream_bytes(shape.page()),
        )
    };
    let mut bytes = Vec::new();
    doc.write_to(&mut bytes).unwrap();
    (bytes, rendered, streams)
}

fn assert_dict_values_present(dict: &PdfObject) {
    for idx in 0..dict.dict_len().unwrap() {
        let key = dict.get_dict_key(idx as i32).unwrap().unwrap();
        assert!(key.is_name().unwrap());
        let value = dict.get_dict_val(idx as i32).unwrap().unwrap();
        assert!(!value.is_null().unwrap(), "resource value must not be null");
    }
}

pub mod m5 {
    pub mod examples {
        use super::super::*;

        #[test]
        fn shape_demo_compiles() {
            build_shape_demo_example(&fresh_output_dir("compile"));
        }

        #[test]
        fn shape_demo_outputs_pdf_and_goldens() {
            let output_dir = fresh_output_dir("shape-demo-output");
            run_shape_demo(&output_dir);

            let pdf_path = output_dir.join("shape_demo.pdf");
            assert!(pdf_path.exists(), "shape_demo did not write output PDF");
            let bytes = fs::read(&pdf_path).unwrap();
            let pages = render_pdf_bytes(&bytes);

            assert_snapshot(SHAPE_DEMO_FIRST_PAGE_SNAPSHOT, &pages[0]);
            for (snapshot, rendered) in MULTIPAGE_SNAPSHOTS.iter().zip(pages.iter()) {
                assert_snapshot(snapshot, rendered);
            }

            for (page_index, rendered) in pages.iter().enumerate() {
                let output_png = output_dir.join(format!("shape_demo_page_{}.png", page_index + 1));
                assert!(output_png.exists(), "missing example PNG {output_png:?}");
                let output_pixmap = Image::from_file(&output_png.to_string_lossy())
                    .unwrap()
                    .to_pixmap()
                    .unwrap();
                assert_eq!(output_pixmap.samples(), rendered.samples());
            }
        }
    }

    pub mod docs {
        use super::super::*;

        #[test]
        fn public_types_impl_debug() {
            fn assert_debug<T: std::fmt::Debug>() {}
            fn assert_shape_debug<'a>()
            where
                Shape<'a>: std::fmt::Debug,
            {
            }

            assert_shape_debug();
            assert_debug::<FinishOptions>();
            assert_debug::<TextOptions<'static>>();
            assert_debug::<TextboxOptions<'static>>();
            assert_debug::<TextAlign>();
            assert_debug::<PdfColor>();
            assert_debug::<RectRadius>();
        }
    }
}

pub mod cross {
    pub mod determinism {
        use super::super::*;

        #[test]
        fn same_process_repeatable() {
            let (_first_pdf, first, _) = build_cross_kitchen_sink();
            let (_second_pdf, second, _) = build_cross_kitchen_sink();
            assert_eq!(first.width(), second.width());
            assert_eq!(first.height(), second.height());
            assert_eq!(first.n(), second.n());
            assert_eq!(first.samples(), second.samples());
        }

        #[test]
        fn cross_process_repeatable() {
            let first_dir = fresh_output_dir("cross-process-a");
            let second_dir = fresh_output_dir("cross-process-b");
            run_shape_demo(&first_dir);
            run_shape_demo(&second_dir);

            for page_index in 1..=3 {
                let first =
                    fs::read(first_dir.join(format!("shape_demo_page_{page_index}.png"))).unwrap();
                let second =
                    fs::read(second_dir.join(format!("shape_demo_page_{page_index}.png"))).unwrap();
                assert_eq!(
                    first, second,
                    "page {page_index} PNG differed across processes"
                );
            }
        }
    }

    pub mod flows {
        use super::super::*;

        #[test]
        fn overlay_preserves_existing_content() {
            let original_doc =
                PdfDocument::from_bytes(include_bytes!("../files/dummy.pdf")).unwrap();
            let original_page = PdfPage::try_from(original_doc.load_page(0).unwrap()).unwrap();
            let original_text = original_page
                .to_text_page(TextPageFlags::empty())
                .unwrap()
                .to_text()
                .unwrap();
            assert!(!original_text.trim().is_empty());

            let mut doc = PdfDocument::from_bytes(include_bytes!("../files/dummy.pdf")).unwrap();
            let mut page = PdfPage::try_from(doc.load_page(0).unwrap()).unwrap();
            {
                let mut shape = Shape::new(&mut page).unwrap();
                shape
                    .draw_rect(&Rect::new(35.0, 360.0, 560.0, 455.0))
                    .unwrap()
                    .finish(&FinishOptions {
                        color: Some(PdfColor::rgb(1.0, 0.0, 0.0)),
                        fill: Some(PdfColor::rgb(1.0, 0.93, 0.93)),
                        width: 3.0,
                        fill_opacity: Some(0.35),
                        ..Default::default()
                    })
                    .unwrap()
                    .insert_text(
                        Point::new(50.0, 340.0),
                        "Overlay note",
                        &TextOptions {
                            fontsize: 18.0,
                            fill: Some(PdfColor::rgb(0.0, 0.0, 1.0)),
                            ..Default::default()
                        },
                    )
                    .unwrap()
                    .commit(&mut doc, true)
                    .unwrap();
            }
            let mut bytes = Vec::new();
            doc.write_to(&mut bytes).unwrap();
            let reopened = PdfDocument::from_bytes(&bytes).unwrap();
            let reopened_page = PdfPage::try_from(reopened.load_page(0).unwrap()).unwrap();
            let reopened_text = reopened_page
                .to_text_page(TextPageFlags::empty())
                .unwrap()
                .to_text()
                .unwrap();
            assert!(reopened_text.contains(original_text.trim()));
        }

        #[test]
        fn underlay_renders_behind() {
            let mut doc = PdfDocument::from_bytes(include_bytes!("../files/dummy.pdf")).unwrap();
            let mut page = PdfPage::try_from(doc.load_page(0).unwrap()).unwrap();
            let baseline = render_page(&page);
            let (dark_x, dark_y, dark_pixel) = find_dark_pixel(&baseline);
            let (white_x, white_y) = find_white_pixel(&baseline);

            let rendered = {
                let mut shape = Shape::new(&mut page).unwrap();
                shape
                    .draw_rect(&Rect::new(0.0, 0.0, 595.0, 842.0))
                    .unwrap()
                    .finish(&FinishOptions {
                        color: None,
                        fill: Some(PdfColor::rgb(0.0, 0.0, 1.0)),
                        fill_opacity: Some(1.0),
                        ..Default::default()
                    })
                    .unwrap()
                    .commit(&mut doc, false)
                    .unwrap();
                render_page(shape.page())
            };

            let dark_after = pixel_rgb(&rendered, dark_x, dark_y);
            assert!(
                dark_after
                    .iter()
                    .zip(dark_pixel)
                    .all(|(actual, expected)| actual.abs_diff(expected) <= 4),
                "underlay should remain behind existing glyph at ({dark_x}, {dark_y}); before={dark_pixel:?} after={dark_after:?}"
            );
            let white_after = pixel_rgb(&rendered, white_x, white_y);
            assert!(
                white_after[0] <= 4 && white_after[1] <= 4 && white_after[2] >= 250,
                "underlay fill should color blank page area blue, got {white_after:?}"
            );
        }

        #[test]
        fn resources_accumulate_correctly() {
            let mut doc = PdfDocument::new();
            let oc_xref = add_ocg(&mut doc, "Resource accumulation");
            let mut page = doc.new_page(Size::A4).unwrap();
            {
                let mut shape = Shape::new(&mut page).unwrap();
                shape
                    .draw_circle(Point::new(120.0, 120.0), 45.0)
                    .unwrap()
                    .finish(&FinishOptions {
                        color: Some(PdfColor::rgb(0.0, 0.0, 0.0)),
                        fill: Some(PdfColor::rgb(1.0, 0.0, 0.0)),
                        fill_opacity: Some(0.5),
                        oc: Some(oc_xref),
                        ..Default::default()
                    })
                    .unwrap()
                    .insert_text(
                        Point::new(72.0, 220.0),
                        "Base font",
                        &TextOptions {
                            fontsize: 18.0,
                            fill: Some(PdfColor::rgb(0.0, 0.0, 0.0)),
                            ..Default::default()
                        },
                    )
                    .unwrap()
                    .insert_text(
                        Point::new(72.0, 265.0),
                        "Custom font",
                        &TextOptions {
                            fontsize: 18.0,
                            fontname: "KitchenSinkCustom".to_owned(),
                            fontfile: Some(CUSTOM_FONT_BYTES),
                            fill: Some(PdfColor::rgb(0.0, 0.0, 0.0)),
                            ..Default::default()
                        },
                    )
                    .unwrap()
                    .draw_rect_with_radius(
                        &Rect::new(70.0, 320.0, 260.0, 410.0),
                        RectRadius::absolute(14.0),
                    )
                    .unwrap()
                    .finish(&FinishOptions {
                        color: Some(PdfColor::rgb(0.0, 0.0, 0.0)),
                        fill: Some(PdfColor::rgb(0.0, 0.0, 1.0)),
                        fill_opacity: Some(0.5),
                        oc: Some(oc_xref),
                        ..Default::default()
                    })
                    .unwrap()
                    .commit(&mut doc, true)
                    .unwrap();
            }

            let resources = page.resources().unwrap();
            let fonts = resources.get_dict("Font").unwrap().unwrap();
            let ext_gstates = resources.get_dict("ExtGState").unwrap().unwrap();
            let properties = resources.get_dict("Properties").unwrap().unwrap();

            assert_eq!(fonts.dict_len().unwrap(), 2);
            assert_eq!(ext_gstates.dict_len().unwrap(), 1);
            assert_eq!(properties.dict_len().unwrap(), 1);
            assert_dict_values_present(&fonts);
            assert_dict_values_present(&ext_gstates);
            assert_dict_values_present(&properties);
        }

        #[test]
        fn repeated_commit_clears_buffers() {
            let mut doc = PdfDocument::new();
            let mut page = doc.new_page(Size::A4).unwrap();
            let mut shape = Shape::new(&mut page).unwrap();

            shape
                .draw_line(Point::new(40.0, 40.0), Point::new(300.0, 40.0))
                .unwrap()
                .finish(&FinishOptions::default())
                .unwrap()
                .commit(&mut doc, true)
                .unwrap();
            assert!(shape.draw_cont().is_empty());
            assert!(shape.text_cont().is_empty());
            assert!(shape.total_cont().is_empty());

            shape
                .draw_rect(&Rect::new(60.0, 80.0, 220.0, 140.0))
                .unwrap()
                .finish(&FinishOptions {
                    fill: Some(PdfColor::rgb(1.0, 1.0, 0.0)),
                    ..Default::default()
                })
                .unwrap()
                .insert_text(
                    Point::new(70.0, 180.0),
                    "Second commit",
                    &TextOptions {
                        fontsize: 18.0,
                        fill: Some(PdfColor::rgb(0.0, 0.0, 0.0)),
                        ..Default::default()
                    },
                )
                .unwrap()
                .commit(&mut doc, true)
                .unwrap();

            let streams = contents_stream_bytes(shape.page());
            let painted = non_wrapper_streams(&streams);
            assert_eq!(painted.len(), 2);
            assert!(painted[0].windows(2).any(|window| window == b"l\n"));
            assert!(painted[1].windows(3).any(|window| window == b"re\n"));
            assert!(painted[1].windows(3).any(|window| window == b"TJ\n"));
        }

        #[test]
        fn borrow_pattern_compiles() {
            let mut doc = PdfDocument::new();
            let mut page = doc.new_page(Size::A4).unwrap();
            {
                let mut shape = Shape::new(&mut page).unwrap();
                shape
                    .draw_line(Point::new(20.0, 20.0), Point::new(120.0, 20.0))
                    .unwrap()
                    .finish(&FinishOptions::default())
                    .unwrap()
                    .commit(&mut doc, true)
                    .unwrap();
            }
            {
                let mut shape = Shape::new(&mut page).unwrap();
                shape
                    .draw_circle(Point::new(80.0, 80.0), 25.0)
                    .unwrap()
                    .finish(&FinishOptions::default())
                    .unwrap()
                    .commit(&mut doc, true)
                    .unwrap();
            }
        }
    }

    pub mod coverage {
        use super::super::*;

        #[test]
        fn all_public_shape_methods_exercised() {
            let public_methods = public_shape_methods();
            let exercised = BTreeSet::from([
                "commit",
                "draw_bezier",
                "draw_circle",
                "draw_curve",
                "draw_line",
                "draw_oval",
                "draw_polyline",
                "draw_quad",
                "draw_rect",
                "draw_rect_with_radius",
                "draw_sector",
                "draw_squiggle",
                "draw_zigzag",
                "finish",
                "insert_text",
                "insert_textbox",
            ]);

            assert_eq!(public_methods, exercised);
        }

        fn public_shape_methods() -> BTreeSet<&'static str> {
            let source_files = [
                include_str!("../../src/shape/drawing.rs"),
                include_str!("../../src/shape/finish.rs"),
                include_str!("../../src/shape/text.rs"),
            ];
            source_files
                .into_iter()
                .flat_map(|source| source.lines())
                .filter_map(|line| {
                    let trimmed = line.trim_start();
                    let rest = trimmed.strip_prefix("pub fn ")?;
                    rest.split_once('(')
                        .map(|(name, _)| name.split_once('<').map_or(name, |(base, _)| base))
                })
                .collect()
        }
    }

    #[test]
    fn kitchen_sink_snapshot_and_round_trip() {
        use super::*;

        let (bytes, rendered, streams) = build_cross_kitchen_sink();
        let reopened = PdfDocument::from_bytes(&bytes).unwrap();
        assert_eq!(reopened.page_count().unwrap(), 1);
        let painted = non_wrapper_streams(&streams);
        assert_eq!(painted.len(), 1);
        assert_snapshot(CROSS_KITCHEN_SINK_SNAPSHOT, &rendered);
    }
}

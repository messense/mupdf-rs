use std::error;
use std::fmt;
use std::{ffi::CStr, ptr::NonNull};

use super::*;
use crate::pdf::{PdfDocument, PdfObject, PdfPage};
use crate::{context, DestinationKind, Error, Rect, Size};

struct TestError {
    message: String,
    source: Option<Box<dyn error::Error + 'static>>,
}

impl TestError {
    fn new<E>(context: &str, source: E) -> Self
    where
        E: Into<Box<dyn error::Error + 'static>>,
    {
        Self {
            message: context.to_owned(),
            source: Some(source.into()),
        }
    }

    fn msg(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }
}

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl fmt::Debug for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)?;
        if let Some(source) = &self.source {
            write!(f, "\n  Caused by: {:?}", source)?;
        }
        Ok(())
    }
}

impl error::Error for TestError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.source.as_deref()
    }
}

const PAGE_SIZE: Size = Size::A4;
const PAGE_HEIGHT: f32 = PAGE_SIZE.height;

/// Generate a non-overlapping rectangle for link at given index.
/// These coordinates are in Fitz space (top-left origin).
fn test_link_rect(index: usize) -> Rect {
    let y = PAGE_HEIGHT - 50.0 - (index as f32 * 15.0);
    Rect {
        x0: 50.0,
        y0: y - 10.0,
        x1: 250.0,
        y1: y,
    }
}

trait LinksCreator {
    fn create_links(self) -> Vec<PdfLink>;
}

impl<T: IntoIterator<Item = PdfAction>> LinksCreator for T {
    fn create_links(self) -> Vec<PdfLink> {
        let create_link = |(idx, action)| PdfLink {
            bounds: test_link_rect(idx),
            action,
        };
        self.into_iter().enumerate().map(create_link).collect()
    }
}

impl PdfPage {
    /// Equivalent to [`PdfPage::pdf_links`] collected into a `Vec`, but relies exclusively
    /// on the Rust-side [`parse_external_link`] implementation for testing purposes.
    fn pdf_links_rust_parsed(&self) -> Result<Vec<PdfLink>, Error> {
        use mupdf_sys::*;

        struct LinksGuard {
            links_head: *mut fz_link,
        }

        impl Drop for LinksGuard {
            fn drop(&mut self) {
                if !self.links_head.is_null() {
                    unsafe {
                        fz_drop_link(context(), self.links_head);
                    }
                }
            }
        }

        let links_head =
            unsafe { ffi_try!(mupdf_load_links(context(), self.inner.as_ptr().cast())) }?;

        let doc_ptr =
            NonNull::new(unsafe { (*self.inner.as_ptr()).doc }).ok_or(Error::UnexpectedNullPtr)?;

        let doc = unsafe { PdfDocument::from_raw(pdf_keep_document(context(), doc_ptr.as_ptr())) };

        let mut output = Vec::new();

        let guard = LinksGuard { links_head };

        let mut next = guard.links_head;

        while !next.is_null() {
            let node = next;
            unsafe {
                next = (*node).next;
                let uri = CStr::from_ptr((*node).uri);

                let action = match parse_external_link(uri.to_string_lossy().as_ref()) {
                    Some(action) => match action {
                        PdfAction::GoTo(PdfDestination::Named(_)) => {
                            let dest = ffi_try!(mupdf_resolve_link_dest(
                                context(),
                                doc.inner,
                                uri.as_ptr()
                            ))?;
                            if dest.loc.page < 0 {
                                continue;
                            }
                            PdfAction::GoTo(PdfDestination::Page {
                                page: dest.loc.page as u32,
                                kind: dest.into(),
                            })
                        }
                        action => action,
                    },
                    None => continue,
                };
                let bounds = (*node).rect.into();
                output.push(PdfLink { bounds, action });
            }
        }

        Ok(output)
    }
}

/// Create a PDF document with the specified number of pages and add links on page 0.
/// Returns the PDF as bytes.
fn create_pdf_with_links(page_count: usize, links: &[PdfLink]) -> PdfDocument {
    assert!(page_count > 0, "page_count must be > 0");

    let mut doc = PdfDocument::new();
    for _ in 0..page_count {
        doc.new_page(PAGE_SIZE).unwrap();
    }

    if !links.is_empty() {
        let mut page = doc.load_pdf_page(0).unwrap();
        page.add_links(&mut doc, links).unwrap();
    }

    doc
}

fn add_named_destinations(doc: &mut PdfDocument, names: &[&str]) -> Result<(), crate::Error> {
    if names.is_empty() {
        return Ok(());
    }

    let page_obj = doc.find_page(0)?;
    let mut names_array = doc.new_array_with_capacity((names.len() * 2) as i32)?;

    for &name in names {
        let mut dest = doc.new_array_with_capacity(6)?;
        dest.array_push_ref(&page_obj)?;
        DestinationKind::FitV { left: Some(200.0) }.encode_into(&mut dest)?;

        names_array.array_push(PdfObject::new_string(name)?)?;
        names_array.array_push(dest)?;
    }

    let mut dests = doc.new_dict_with_capacity(1)?;
    dests.dict_put("Names", names_array)?;

    let mut names_dict = doc.new_dict_with_capacity(1)?;
    names_dict.dict_put("Dests", dests)?;

    doc.catalog()?.dict_put("Names", names_dict)?;
    Ok(())
}

/// Extract links from page 0 of a PDF using standard `pdf_links`.
fn extract_links(pdf: &PdfDocument) -> Result<Vec<PdfLink>, TestError> {
    let page = pdf
        .load_pdf_page(0)
        .map_err(|e| TestError::new("[pdf_links] Page 0 load failed", e))?;

    page.pdf_links()
        .map_err(|e| TestError::new("[pdf_links] Iterator init failed", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| TestError::new("[pdf_links] Collection failed", e))
}

/// Extract links from page 0 using Rust parsing logic ([`parse_external_link`])
fn extract_rust_parsed_links(pdf: &PdfDocument) -> Result<Vec<PdfLink>, TestError> {
    let page = pdf
        .load_pdf_page(0)
        .map_err(|e| TestError::new("[rust_parsed] Page 0 load failed", e))?;

    page.pdf_links_rust_parsed()
        .map_err(|e| TestError::new("[rust_parsed] Extraction failed", e))
}

/// Extract raw URI strings from page 0.
fn extract_raw_links(pdf: &PdfDocument) -> Result<Vec<String>, TestError> {
    let page = pdf
        .load_pdf_page(0)
        .map_err(|e| TestError::new("[raw_links] Page 0 load failed", e))?;

    page.links()
        .map_err(|e| TestError::new("[raw_links] Iterator init failed", e))
        .map(|iter| iter.map(|link| link.uri).collect())
}

fn assert_links_match(pdf: &PdfDocument, expected_vec: &[PdfLink]) -> Result<(), TestError> {
    let verify = |extracted: &[PdfLink], label: &str| -> Result<(), TestError> {
        if extracted.len() != expected_vec.len() {
            return Err(TestError::msg(format!(
                "{}Link count mismatch: extracted {}, expected {}",
                label,
                extracted.len(),
                expected_vec.len()
            )));
        }
        for (i, (extract, expect)) in extracted.iter().zip(expected_vec.iter()).enumerate() {
            if extract != expect {
                return Err(TestError::msg(format!(
                    "{}Link '{}' mismatch:\n  extracted: {:?}\n  expected:  {:?}",
                    label, i, extract, expect
                )));
            }
        }
        Ok(())
    };

    verify(&extract_links(pdf)?, "[pdf_links] ")?;
    verify(&extract_rust_parsed_links(pdf)?, "[rust_parsed] ")
}

fn assert_raw_uri_matches(pdf: &PdfDocument, expected_vec: &[PdfLink]) -> Result<(), TestError> {
    let extracted = extract_raw_links(pdf)?;
    if extracted.len() != expected_vec.len() {
        return Err(TestError::msg(format!(
            "Raw URI count mismatch: extracted {}, expected {}",
            extracted.len(),
            expected_vec.len(),
        )));
    }
    for (i, (extract, expect)) in extracted.iter().zip(expected_vec.iter()).enumerate() {
        let expected = expect.action.to_string();
        if *extract != expected {
            return Err(TestError::msg(format!(
                "Link '{}' raw uri mismatch:\n  extracted: '{}'\n  expected:  '{}'",
                i, extract, expected
            )));
        }
    }
    Ok(())
}

#[test]
fn test_url_and_gotor_url() {
    let cases = [
        "https://example.com/hello%20world",
        "https://example.com/test%2Fpath",
        "https://example.com/name%3Dvalue",
        "https://example.com/%%25",
        "https://example.com/%E4%B8%AD",
        "https://example.com/%E3%81%82",
        "https://example.com/hello%E4%B8%AD%E6%96%87",
        "https://example.com/%FF%FE",
        "https://example.com/100%%",
        "http://example.com/page",
        "https://example.com/secure",
        "ftp://ftp.example.com/file.txt",
        "custom://resource/path",
    ];
    let links = cases.map(|l| PdfAction::Uri(l.to_owned())).create_links();

    let pdf = create_pdf_with_links(1, &links);
    assert_links_match(&pdf, &links).unwrap();
    assert_raw_uri_matches(&pdf, &links).unwrap();

    let links = cases
        .into_iter()
        .enumerate()
        .map(|(i, url)| ((i % 6) as u32, FileSpec::Url(format!("{url}.pdf"))))
        .map(|(page, file)| PdfAction::GoToR {
            file,
            dest: PdfDestination::Page {
                page,
                kind: DestinationKind::default(),
            },
        })
        .create_links();

    let pdf = create_pdf_with_links(7, &links);
    assert_links_match(&pdf, &links).unwrap();
    assert_raw_uri_matches(&pdf, &links).unwrap();
}

#[test]
fn test_uri_links() {
    let links = [
        PdfAction::Uri("http://example.com/page".into()),
        PdfAction::Uri("https://example.com/secure".into()),
        PdfAction::Uri("mailto:user@example.com".into()),
        PdfAction::Uri("ftp://ftp.example.com/file.txt".into()),
        PdfAction::Uri("tel:+1-555-123-4567".into()),
        PdfAction::Uri("HTTP://EXAMPLE.COM".into()),
        PdfAction::Uri("custom://resource/path".into()),
        PdfAction::Uri("cmd://goto-page/12".into()),
    ];
    let links = links.create_links();

    let pdf = create_pdf_with_links(1, &links);
    assert_links_match(&pdf, &links).unwrap();
    assert_raw_uri_matches(&pdf, &links).unwrap();
}

#[test]
fn test_goto_and_gotor_links() {
    let kinds = [
        DestinationKind::Fit,
        DestinationKind::FitB,
        DestinationKind::FitH { top: Some(500.0) },
        DestinationKind::FitH { top: None },
        DestinationKind::FitV { left: Some(100.0) },
        DestinationKind::FitV { left: None },
        DestinationKind::FitBH { top: Some(300.0) },
        DestinationKind::FitBH { top: None },
        DestinationKind::FitBV { left: Some(50.0) },
        DestinationKind::FitBV { left: None },
        DestinationKind::XYZ {
            left: None,
            top: None,
            zoom: None,
        },
        DestinationKind::XYZ {
            left: Some(0.0),
            top: Some(0.0),
            zoom: Some(50.0),
        },
        DestinationKind::XYZ {
            left: Some(100.0),
            top: None,
            zoom: None,
        },
        DestinationKind::XYZ {
            left: None,
            top: Some(250.0),
            zoom: None,
        },
        DestinationKind::XYZ {
            left: None,
            top: None,
            zoom: Some(200.0),
        },
        DestinationKind::XYZ {
            left: Some(100.0),
            top: Some(600.0),
            zoom: Some(150.0),
        },
        DestinationKind::XYZ {
            left: Some(50.0),
            top: Some(700.0),
            zoom: None,
        },
        DestinationKind::XYZ {
            left: Some(50.0),
            top: None,
            zoom: Some(300.0),
        },
        DestinationKind::XYZ {
            left: Some(200.0),
            top: Some(PAGE_HEIGHT),
            zoom: Some(100.0),
        },
        DestinationKind::XYZ {
            left: None,
            top: Some(500.0),
            zoom: Some(75.0),
        },
    ];
    let links = kinds
        .into_iter()
        .enumerate()
        .map(|(i, kind)| ((i % 6) as u32, kind))
        .map(|(page, kind)| PdfAction::GoTo(PdfDestination::Page { page, kind }))
        .create_links();

    let pdf = create_pdf_with_links(7, &links);
    assert_links_match(&pdf, &links).unwrap();
    assert_raw_uri_matches(&pdf, &links).unwrap();

    let links = kinds
        .into_iter()
        .enumerate()
        .map(|(i, kind)| ((i % 6) as u32, kind))
        .map(|(page, kind)| PdfAction::GoToR {
            file: if page % 2 == 0 {
                FileSpec::Path("page_destinations.pdf".into())
            } else {
                FileSpec::Url("https://example.com/document.pdf".into())
            },
            dest: PdfDestination::Page { page, kind },
        })
        .create_links();

    let pdf = create_pdf_with_links(7, &links);
    assert_links_match(&pdf, &links).unwrap();
    assert_raw_uri_matches(&pdf, &links).unwrap();
}

#[test]
fn test_fitr_goto_and_gotor_links() {
    let dests = [PdfDestination::Page {
        page: 1,
        kind: DestinationKind::FitR {
            left: 50.0,
            bottom: 100.0,
            right: 200.0,
            top: 300.0,
        },
    }];

    let links = dests.iter().cloned().map(PdfAction::GoTo).create_links();
    let pdf = create_pdf_with_links(3, &links);
    assert_links_match(&pdf, &links).unwrap();
    assert_raw_uri_matches(&pdf, &links).unwrap();

    let links = dests
        .map(|dest| PdfAction::GoToR {
            file: FileSpec::Path("page_destinations.pdf".into()),
            dest,
        })
        .create_links();
    let pdf = create_pdf_with_links(3, &links);
    assert_links_match(&pdf, &links).unwrap();
    assert_raw_uri_matches(&pdf, &links).unwrap();
}

#[test]
fn test_rotated_target_page() {
    // This test combines the CTM checks for landing pages.
    // We check:
    // 1. Exact coordinate hits (full XYZ).
    // 2. Single-axis snapping (FitH, FitV, etc.) taking into account data loss at 90/270 degrees.
    // 3. Partial XYZ coordinates (Some/None) and their behavior when rotated.

    let mut pdf = PdfDocument::new();
    let rotations = [0, 90, 180, 270];

    {
        for (idx, &rotation) in rotations.iter().enumerate() {
            pdf.new_page(PAGE_SIZE).unwrap();
            let mut page = pdf.load_pdf_page(idx as i32).unwrap();
            page.set_rotation(rotation).unwrap();
        }
    }

    for (target_page_idx, &rotation) in rotations.iter().enumerate() {
        let target_page_idx = target_page_idx as u32;

        pdf.new_page(PAGE_SIZE).unwrap();
        let source_page_idx = pdf.page_count().unwrap() as i32 - 1;

        let context_msg = format!(
            "Failed at rotation: {}°, target_page: {}",
            rotation, target_page_idx
        );

        let mut actions = Vec::new();

        let bounds = {
            let page = pdf.load_pdf_page(target_page_idx as i32).unwrap();
            page.bounds().unwrap()
        };
        // Calculate key points
        let mid_x = (bounds.x0 + bounds.x1) * 0.5;
        let mid_y = (bounds.y0 + bounds.y1) * 0.5;

        // Full XYZ Points
        let inset = 1.25;
        let points = [
            (bounds.x0 + inset, bounds.y0 + inset), // near top-left
            (mid_x, mid_y),                         // center
            (bounds.x1 - inset, bounds.y1 - inset), // near bottom-right
        ];

        for (left, top) in points {
            actions.push(PdfAction::GoTo(PdfDestination::Page {
                page: target_page_idx,
                kind: DestinationKind::XYZ {
                    left: Some(left),
                    top: Some(top),
                    zoom: Some(100.0),
                },
            }));
        }

        let is_orthogonal = rotation == 90 || rotation == 270;

        // Single Axis
        let target_top = if is_orthogonal { None } else { Some(mid_y) };
        let target_left = if is_orthogonal { None } else { Some(mid_x) };

        actions.push(PdfAction::GoTo(PdfDestination::Page {
            page: target_page_idx,
            kind: DestinationKind::FitH { top: target_top },
        }));
        actions.push(PdfAction::GoTo(PdfDestination::Page {
            page: target_page_idx,
            kind: DestinationKind::FitBH { top: target_top },
        }));
        actions.push(PdfAction::GoTo(PdfDestination::Page {
            page: target_page_idx,
            kind: DestinationKind::FitV { left: target_left },
        }));
        actions.push(PdfAction::GoTo(PdfDestination::Page {
            page: target_page_idx,
            kind: DestinationKind::FitBV { left: target_left },
        }));

        // Partial XYZ
        // These combinations only work correctly at 0/180 due to the peculiarities of MuPDF
        if !is_orthogonal {
            actions.push(PdfAction::GoTo(PdfDestination::Page {
                page: target_page_idx,
                kind: DestinationKind::XYZ {
                    left: Some(mid_x),
                    top: None,
                    zoom: None,
                },
            }));
            actions.push(PdfAction::GoTo(PdfDestination::Page {
                page: target_page_idx,
                kind: DestinationKind::XYZ {
                    left: None,
                    top: Some(mid_y),
                    zoom: None,
                },
            }));
        }
        // These always work (Zoom Only or Empty)
        actions.push(PdfAction::GoTo(PdfDestination::Page {
            page: target_page_idx,
            kind: DestinationKind::XYZ {
                left: None,
                top: None,
                zoom: Some(100.0),
            },
        }));
        actions.push(PdfAction::GoTo(PdfDestination::Page {
            page: target_page_idx,
            kind: DestinationKind::XYZ {
                left: None,
                top: None,
                zoom: None,
            },
        }));

        let links = actions.create_links();

        {
            let mut source_page = pdf.load_pdf_page(source_page_idx).unwrap();
            source_page.add_links(&mut pdf, &links).unwrap();
        }

        let extracted = {
            let page = pdf
                .load_pdf_page(source_page_idx)
                .expect("Failed to load source page");
            page.pdf_links()
                .expect("Failed to get link iterator")
                .collect::<Result<Vec<_>, _>>()
                .expect("Failed to extract links")
        };

        assert_eq!(
            extracted.len(),
            links.len(),
            "{}: Count mismatch",
            context_msg
        );

        for (i, (act, exp)) in extracted.iter().zip(links.iter()).enumerate() {
            assert_eq!(act, exp, "{}: Link index {} mismatch", context_msg, i);
        }

        let raw_links: Vec<String> = {
            let page = pdf.load_pdf_page(source_page_idx).unwrap();
            page.links().unwrap().map(|l| l.uri).collect()
        };
        for (i, (org, raw)) in links.iter().zip(raw_links.iter()).enumerate() {
            assert_eq!(
                org.action.to_string(),
                *raw,
                "{}: Raw URI index {} mismatch",
                context_msg,
                i
            );
        }
    }
}

#[test]
fn test_fitr_rotated_target_page() {
    let mut pdf = PdfDocument::new();
    let rotations = [0, 90, 180, 270];

    {
        for (idx, &rotation) in rotations.iter().enumerate() {
            pdf.new_page(PAGE_SIZE).unwrap();
            let mut page = pdf.load_pdf_page(idx as i32).unwrap();
            page.set_rotation(rotation).unwrap();
        }
    }

    for (target_page_idx, &rotation) in rotations.iter().enumerate() {
        let target_page_idx = target_page_idx as u32;

        pdf.new_page(PAGE_SIZE).unwrap();
        let source_page_idx = pdf.page_count().unwrap() as i32 - 1;

        let context_msg = format!(
            "Failed at rotation: {}°, target_page: {}",
            rotation, target_page_idx
        );

        let mut actions = Vec::new();

        let bounds = {
            let page = pdf.load_pdf_page(target_page_idx as i32).unwrap();
            page.bounds().unwrap()
        };

        let inset = 10.0;
        let min_x = bounds.x0 + inset;
        let min_y = bounds.y0 + inset;
        let max_x = bounds.x1 - inset;
        let max_y = bounds.y1 - inset;
        let mid_x = (min_x + max_x) * 0.5;
        let mid_y = (min_y + max_y) * 0.5;
        let quarter_w = (max_x - min_x) * 0.25;
        let quarter_h = (max_y - min_y) * 0.25;

        // Small rect near top-left
        actions.push(PdfAction::GoTo(PdfDestination::Page {
            page: target_page_idx,
            kind: DestinationKind::FitR {
                left: min_x,
                bottom: min_y,
                right: min_x + quarter_w,
                top: min_y + quarter_h,
            },
        }));
        // Rect at center
        actions.push(PdfAction::GoTo(PdfDestination::Page {
            page: target_page_idx,
            kind: DestinationKind::FitR {
                left: mid_x - quarter_w * 0.5,
                bottom: mid_y - quarter_h * 0.5,
                right: mid_x + quarter_w * 0.5,
                top: mid_y + quarter_h * 0.5,
            },
        }));
        // Rect near bottom-right
        actions.push(PdfAction::GoTo(PdfDestination::Page {
            page: target_page_idx,
            kind: DestinationKind::FitR {
                left: max_x - quarter_w,
                bottom: max_y - quarter_h,
                right: max_x,
                top: max_y,
            },
        }));

        let links = actions.create_links();

        {
            let mut source_page = pdf.load_pdf_page(source_page_idx).unwrap();
            source_page.add_links(&mut pdf, &links).unwrap();
        }

        let extracted = {
            let page = pdf
                .load_pdf_page(source_page_idx)
                .expect("Failed to load source page");
            page.pdf_links()
                .expect("Failed to get link iterator")
                .collect::<Result<Vec<_>, _>>()
                .expect("Failed to extract links")
        };

        assert_eq!(
            extracted.len(),
            links.len(),
            "{}: Count mismatch",
            context_msg
        );

        for (i, (act, exp)) in extracted.iter().zip(links.iter()).enumerate() {
            assert_eq!(act, exp, "{}: Link index {} mismatch", context_msg, i);
        }

        let raw_links: Vec<String> = {
            let page = pdf.load_pdf_page(source_page_idx).unwrap();
            page.links().unwrap().map(|l| l.uri).collect()
        };

        assert_eq!(
            raw_links.len(),
            links.len(),
            "{}: Count mismatch",
            context_msg
        );

        for (i, (org, raw)) in links.iter().zip(raw_links.iter()).enumerate() {
            let org = org.action.to_string();
            assert_eq!(&org, raw, "{}: Link index {} mismatch", context_msg, i);
        }
    }
}

#[test]
fn test_rotated_source_page() {
    for source_rotation in [90, 180, 270] {
        let mut pdf = PdfDocument::new();
        pdf.new_page(PAGE_SIZE).unwrap();
        pdf.new_page(PAGE_SIZE).unwrap();

        let link_bounds;
        {
            let mut page0 = pdf.load_pdf_page(0).unwrap();
            page0.set_rotation(source_rotation).unwrap();
            let bounds = page0.bounds().unwrap();

            let inset = 20.0;
            link_bounds = Rect {
                x0: bounds.x0 + inset,
                y0: bounds.y0 + inset,
                x1: bounds.x0 + inset + 200.0,
                y1: bounds.y0 + inset + 10.0,
            };
        }

        let links = vec![
            PdfLink {
                bounds: link_bounds,
                action: PdfAction::GoTo(PdfDestination::Page {
                    page: 1,
                    kind: DestinationKind::XYZ {
                        left: Some(50.0),
                        top: Some(400.0),
                        zoom: Some(100.0),
                    },
                }),
            },
            PdfLink {
                bounds: Rect {
                    x0: link_bounds.x0,
                    y0: link_bounds.y1 + 5.0,
                    x1: link_bounds.x1,
                    y1: link_bounds.y1 + 15.0,
                },
                action: PdfAction::GoTo(PdfDestination::Page {
                    page: 1,
                    kind: DestinationKind::Fit,
                }),
            },
            PdfLink {
                bounds: Rect {
                    x0: link_bounds.x0,
                    y0: link_bounds.y1 + 20.0,
                    x1: link_bounds.x1,
                    y1: link_bounds.y1 + 30.0,
                },
                action: PdfAction::GoTo(PdfDestination::Page {
                    page: 1,
                    kind: DestinationKind::FitH { top: Some(300.0) },
                }),
            },
            PdfLink {
                bounds: Rect {
                    x0: link_bounds.x0,
                    y0: link_bounds.y1 + 35.0,
                    x1: link_bounds.x1,
                    y1: link_bounds.y1 + 45.0,
                },
                action: PdfAction::Uri("https://example.com".into()),
            },
        ];

        {
            let mut page0 = pdf.load_pdf_page(0).unwrap();
            page0.add_links(&mut pdf, &links).unwrap();
        }
        assert_links_match(&pdf, &links).unwrap();
        assert_raw_uri_matches(&pdf, &links).unwrap();
    }
}

#[test]
fn test_named_goto_and_gotor_links() {
    let cases = [
        "Chapter1",
        "G11.2063217",
        "section_1.1.b",
        "anchor-42",
        "Reference:Index",
        // Names that imitate parameters
        "page=10",
        "zoom=200,10,10",
        "view=FitH,100",
        "nameddest=True",
        "12345",
        "Name With Spaces",
        // Special characters and punctuation in PDF
        "Name/With/Slashes",
        "Name(With)Parens",
        "Name[With]Brackets",
        "Quote's",
        "Double\"Quotes",
        // Complete set of ASCII
        "~!@#$%^&*()_+`-={}|[]\\:\";'<>?,./",
        // Unicode and complex encodings
        "Заголовок_Кириллица",
        "章节",
        "😁_Emoji_Dest",
        "A\u{00A0}B",          // Non-breaking space
        "C\u{2003}D",          // Em-space
        "Z\u{200D}W\u{200D}J", // Zero-width joiners
        // Extreme length
        &"A".repeat(1024),
        // Normal percent-encoding
        "hello%20world",
        "test%2Fpath",
        "name%3Dvalue",
        "SimpleName",
        // Unicode (UTF-8) percent-encoding
        "%E4%B8%AD",
        "%E3%81%82",
        "hello%E4%B8%AD%E6%96%87",
        // Invalid percent-encoding
        "%FF%FE",
        "100%%",
        "bad%2",
    ];

    let links = cases
        .iter()
        .map(|&name| PdfAction::GoTo(PdfDestination::Named(name.to_string())))
        .create_links();

    let mut pdf = create_pdf_with_links(1, &links);
    add_named_destinations(&mut pdf, &cases).unwrap();

    let resolved_links: Vec<PdfLink> = links
        .iter()
        .map(|link| PdfLink {
            bounds: link.bounds,
            action: PdfAction::GoTo(PdfDestination::Page {
                page: 0,
                kind: DestinationKind::FitV { left: Some(200.0) },
            }),
        })
        .collect();

    assert_links_match(&pdf, &resolved_links).unwrap();
    assert_raw_uri_matches(&pdf, &links).unwrap();

    let links = cases
        .iter()
        .map(|&name| PdfDestination::Named(name.to_string()))
        .enumerate()
        .map(|(idx, dest)| PdfAction::GoToR {
            file: if idx % 2 == 0 {
                FileSpec::Path("page_destinations.pdf".into())
            } else {
                FileSpec::Url("https://example.com/document.pdf".into())
            },
            dest,
        })
        .create_links();

    let pdf = create_pdf_with_links(1, &links);

    assert_links_match(&pdf, &links).unwrap();
    assert_raw_uri_matches(&pdf, &links).unwrap();
}

#[test]
fn test_path_links_with_launch_and_gotor() {
    let paths = [
        // Simple file
        "readme",
        // Absolute path
        "/path/to/document",
        // Relative paths
        "other/path/document",
        "another/path/document",
        "../path/document",
        // UNC / Server path
        "/server/share/doc",
        // Edge cases with dots
        "",
        ".",
        "../report",
        "a/..",
        // Normal percent-encoding
        "/path/to/hello%20world",
        "/path/to/test%2Fpath",
        "/path/to/name%3Dvalue",
        "/path/to/SimpleName",
        "../to/hello%20world",
        "../to/test%2Fpath",
        "../to/name%3Dvalue",
        "../to/SimpleName",
        // Unicode (UTF-8) percent-encoding
        "/path/to/%E4%B8%AD",
        "/path/to/%E3%81%82",
        "/path/to/hello%E4%B8%AD%E6%96%87",
        "../to/%E4%B8%AD",
        "../to/%E3%81%82",
        "../to/hello%E4%B8%AD%E6%96%87",
        // Invalid percent-encoding
        "/path/to/%FF%FE",
        "/path/to/100%%",
        "/path/to/bad%2",
        "../to/%FF%FE",
        "../to/100%%",
        "../to/bad%2",
        // Unicode
        "文件",
        "документ",
    ];

    // 1. Test Launch actions

    let links = paths
        .iter()
        .map(|path| PdfAction::Launch(FileSpec::Path(format!("{path}.docx"))))
        .create_links();

    let pdf = create_pdf_with_links(1, &links);

    assert_links_match(&pdf, &links).unwrap();
    assert_raw_uri_matches(&pdf, &links).unwrap();

    // 2. Test GoToR actions (same paths)
    let links = paths
        .iter()
        .enumerate()
        .map(|(idx, path)| PdfAction::GoToR {
            file: if idx % 2 == 0 || !path.is_ascii() {
                FileSpec::Path(format!("{path}.pdf"))
            } else {
                FileSpec::Url(format!("https://example.com/{path}.pdf"))
            },
            dest: PdfDestination::default(),
        })
        .create_links();

    let pdf = create_pdf_with_links(1, &links);

    assert_links_match(&pdf, &links).unwrap();
    assert_raw_uri_matches(&pdf, &links).unwrap();

    // 3. Test Path Normalization

    let paths = [
        // Path for paths normalization
        "a/../b",
        "a/../../b",
        "/a//b/./c",
        "/../a",
        "/a/../../b",
    ];

    let clean_paths = ["b", "../b", "/a/b/c", "/a", "/b"];

    // 3a. Launch Normalization

    let links = paths
        .iter()
        .map(|path| PdfAction::Launch(FileSpec::Path(format!("{path}.docx"))))
        .create_links();

    let pdf = create_pdf_with_links(1, &links);

    let links_with_normalized_paths = clean_paths
        .iter()
        .map(|path| PdfAction::Launch(FileSpec::Path(format!("{path}.docx"))))
        .create_links();

    assert_links_match(&pdf, &links_with_normalized_paths).unwrap();
    assert_raw_uri_matches(&pdf, &links_with_normalized_paths).unwrap();

    // 3b. GoToR Normalization

    let links = paths
        .iter()
        .map(|path| PdfAction::GoToR {
            file: FileSpec::Path(format!("{path}.pdf")),
            dest: PdfDestination::default(),
        })
        .create_links();

    let pdf = create_pdf_with_links(1, &links);

    let links_with_normalized_paths = clean_paths
        .iter()
        .map(|path| PdfAction::GoToR {
            file: FileSpec::Path(format!("{path}.pdf")),
            dest: PdfDestination::default(),
        })
        .create_links();

    assert_links_match(&pdf, &links_with_normalized_paths).unwrap();
    assert_raw_uri_matches(&pdf, &links_with_normalized_paths).unwrap();
}

#[test]
fn test_mixed_links() {
    let links = [
        PdfAction::Uri("https://example.com".into()),
        PdfAction::GoTo(PdfDestination::Page {
            page: 2,
            kind: DestinationKind::Fit,
        }),
        PdfAction::GoTo(PdfDestination::Page {
            page: 3,
            kind: DestinationKind::XYZ {
                left: Some(100.0),
                top: Some(500.0),
                zoom: Some(150.0),
            },
        }),
        PdfAction::Launch(FileSpec::Path("readme.txt".into())),
        PdfAction::GoToR {
            file: FileSpec::Path("other.pdf".into()),
            dest: PdfDestination::Page {
                page: 0,
                kind: DestinationKind::Fit,
            },
        },
    ];
    let links = links.create_links();

    let pdf = create_pdf_with_links(5, &links);
    assert_links_match(&pdf, &links).unwrap();
    assert_raw_uri_matches(&pdf, &links).unwrap();
}

/// Test large coordinates handling.
///
/// Note: MuPDF clamps coordinates to page bounds during link resolution.
/// This is expected behavior - coordinates outside the page are clamped.
///
/// We don't use [`extract_rust_parsed_links`], since the coordinate clamping
/// logic is not implemented on the Rust side.
#[test]
fn test_large_coordinates() {
    let links = vec![PdfLink {
        bounds: test_link_rect(0),
        action: PdfAction::GoTo(PdfDestination::Page {
            page: 1,
            kind: DestinationKind::XYZ {
                left: Some(10000.0),
                top: Some(10000.0),
                zoom: Some(500.0),
            },
        }),
    }];

    let pdf = create_pdf_with_links(2, &links);
    let extracted = extract_links(&pdf).unwrap();

    assert_eq!(extracted.len(), 1);

    match &extracted[0].action {
        PdfAction::GoTo(PdfDestination::Page { page, kind }) => {
            assert_eq!(*page, 1);
            match kind {
                DestinationKind::XYZ { left, top, zoom } => {
                    // MuPDF clamps coordinates to page size.
                    assert_eq!(
                        *left,
                        Some(PAGE_SIZE.width),
                        "left should be clamped to page width"
                    );
                    assert_eq!(
                        *top,
                        Some(PAGE_SIZE.height),
                        "top should be clamped to page height"
                    );
                    assert_eq!(*zoom, Some(500.0), "zoom should be preserved");
                }
                other => panic!("Expected XYZ, got {:?}", other),
            }
        }
        other => panic!("Expected GoTo(Page), got {:?}", other),
    }
}

#[test]
fn test_no_links() {
    let links: Vec<PdfLink> = vec![];

    let pdf = create_pdf_with_links(1, &links);

    assert_links_match(&pdf, &links).unwrap();
}

#[test]
fn test_many_links() {
    // Create 20 links on a single page
    let links = (0..20)
        .map(|i| PdfAction::Uri(format!("https://example.com/page{}", i)))
        .create_links();

    let pdf = create_pdf_with_links(1, &links);
    assert_links_match(&pdf, &links).unwrap();
    assert_raw_uri_matches(&pdf, &links).unwrap();
}

#[test]
fn test_gotor_url_with_existing_fragment() {
    let links = [
        PdfAction::GoToR {
            file: FileSpec::Url("http://example.org/doc.pdf#pagemode=bookmarks".into()),
            dest: PdfDestination::Page {
                page: 1,
                kind: DestinationKind::Fit,
            },
        },
        PdfAction::GoToR {
            file: FileSpec::Url("http://example.org/doc.pdf#pagemode=none".into()),
            dest: PdfDestination::Named("Chapter1".into()),
        },
    ];
    let links = links.create_links();

    let pdf = create_pdf_with_links(3, &links);
    assert_raw_uri_matches(&pdf, &links).unwrap();

    let expected_links = [
        PdfAction::Uri("http://example.org/doc.pdf#pagemode=bookmarks&page=2&view=Fit".into()),
        PdfAction::Uri("http://example.org/doc.pdf#pagemode=none&nameddest=Chapter1".into()),
    ];
    let expected_links = expected_links.create_links();

    assert_links_match(&pdf, &expected_links).unwrap();
}

#[test]
#[should_panic(
    expected = "PdfPage ownership mismatch: the page is not attached to the provided PdfDocument"
)]
fn test_add_links_with_wrong_document_context() {
    let mut doc_owner = PdfDocument::new();
    doc_owner.new_page(PAGE_SIZE).unwrap();

    let mut page = doc_owner.load_pdf_page(0).unwrap();

    let mut doc_alien = PdfDocument::new();

    let links = [PdfAction::Uri("https://fail.com".into())].create_links();
    page.add_links(&mut doc_alien, &links).unwrap();
}

use std::error;
use std::fmt;
use std::iter::repeat_n;
use std::slice;
use std::sync::LazyLock;
use std::{ffi::CStr, ptr::NonNull};

use super::*;
use crate::pdf::{PdfDocument, PdfObject, PdfPage};
use crate::{context, DestinationKind, Error, Rect, Size};

struct TestError {
    message: String,
    source: Option<Box<dyn error::Error + 'static>>,
}

impl TestError {
    fn new<M: Into<String>, E>(message: M, source: E) -> Self
    where
        E: Into<Box<dyn error::Error + 'static>>,
    {
        Self {
            message: message.into(),
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
            write!(f, "\nCaused by: {:?}", source)?;
        }
        Ok(())
    }
}

impl error::Error for TestError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.source.as_deref()
    }
}

type TestResult<T> = Result<T, TestError>;

trait TestContext<T> {
    fn context<M: Into<String>>(self, msg: M) -> TestResult<T>;
}

impl<T, E: error::Error + 'static> TestContext<T> for Result<T, E> {
    fn context<M: Into<String>>(self, msg: M) -> TestResult<T> {
        self.map_err(|e| TestError::new(msg, e))
    }
}

pub(super) const PAGE_SIZE: Size = Size::A4;
pub(super) const PAGE_HEIGHT: f32 = PAGE_SIZE.height;
pub(super) const PAGE_WIDTH: f32 = PAGE_SIZE.width;

/// The [`DestinationKind`] used by [`add_named_destinations`] for all named destinations.
const NAMED_DEST_KIND: DestinationKind = DestinationKind::FitV { left: Some(200.0) };

/// The resolved [`PdfAction`] expected when a named destination is resolved.
/// All named destinations point to page 0 with [`NAMED_DEST_KIND`].
const NAMED_DEST_RESOLVED: PdfAction = PdfAction::GoTo(PdfDestination::Page {
    page: 0,
    kind: NAMED_DEST_KIND,
});

/// Generate a non-overlapping rectangle for link at given index.
/// These coordinates are in Fitz space (top-left origin).
pub(super) fn test_link_rect(index: usize) -> Rect {
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

impl<T: IntoIterator<Item = U>, U: Into<LinkAction>> LinksCreator for T {
    fn create_links(self) -> Vec<PdfLink> {
        let create_link = |(idx, action): (usize, U)| PdfLink {
            bounds: test_link_rect(idx),
            action: action.into(),
        };
        self.into_iter().enumerate().map(create_link).collect()
    }
}

impl PdfPage {
    /// Equivalent to [`PdfPage::resolved_links`] collected into a `Vec`, but relies exclusively
    /// on the Rust-side [`parse_external_link`] implementation for testing purposes.
    fn resolved_links_rust_parsed(&self) -> Result<Vec<PdfLink>, Error> {
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
                output.push(PdfLink {
                    bounds,
                    action: action.into(),
                });
            }
        }

        Ok(output)
    }
}

struct PdfTester {
    doc: PdfDocument,
    page: usize,
}

impl PdfTester {
    /// Creates a new PDF document with the specified number of blank pages.
    /// The focus page defaults to 0
    fn new(page_count: u32) -> Self {
        assert!(page_count > 0, "page_count must be > 0");
        let mut doc = PdfDocument::new();
        for _ in 0..page_count {
            doc.new_page(PAGE_SIZE).unwrap();
        }
        Self { doc, page: 0 }
    }

    /// Creates a new PDF and adds links on page 0
    fn with_links(page_count: u32, links: &[PdfLink]) -> Self {
        let mut test_pdf = Self::new(page_count);
        test_pdf.add_links(links).unwrap();
        test_pdf
    }

    /// Creates a new PDF with pages of the specified sizes
    fn with_sizes(sizes: &[Size]) -> Self {
        assert!(!sizes.is_empty(), "must provide at least one size");
        let mut doc = PdfDocument::new();
        for &size in sizes {
            doc.new_page(size).unwrap();
        }
        Self { doc, page: 0 }
    }

    /// Sets the focus page for subsequent operations
    fn on_page(&mut self, page: usize) -> &mut Self {
        self.page = page;
        self
    }

    /// Adds a new page with the given size
    fn new_page(&mut self, size: Size) -> &mut Self {
        self.doc.new_page(size).unwrap();
        self
    }

    fn page_count(&self) -> usize {
        self.doc.page_count().unwrap() as usize
    }

    /// Loads the current focus page
    fn load_page(&self) -> TestResult<PdfPage> {
        self.doc
            .load_pdf_page(self.page as i32)
            .context(format!("Page {} load failed", self.page))
    }

    /// Adds links on the current focus page
    fn add_links(&mut self, links: &[PdfLink]) -> TestResult<&mut Self> {
        self.load_page()?
            .add_links(&mut self.doc, links)
            .context(format!("add_links: Failed on page {}", self.page))?;
        Ok(self)
    }

    /// Sets rotation on the current focus page
    fn set_rotation(&mut self, rotation: i32) -> TestResult<&mut Self> {
        self.load_page()?
            .set_rotation(rotation)
            .context(format!("set_rotation: Failed on page {}", self.page))?;
        Ok(self)
    }

    /// Adds named destinations pointing to page 0 with `FitV { left: 200 }`
    fn add_named_destinations<T: AsRef<str>>(&mut self, names: &[T]) -> TestResult<&mut Self> {
        add_named_destinations(&mut self.doc, names).context("add named destinations failed")?;
        Ok(self)
    }

    fn bounds(&self) -> TestResult<Rect> {
        self.load_page()?
            .bounds()
            .context(format!("bounds: Failed on page {}", self.page))
    }

    fn extract_links(&self) -> TestResult<Vec<PdfLink>> {
        self.load_page()?
            .resolved_links()
            .context("[resolved_links] Iterator init failed")?
            .collect::<Result<Vec<_>, _>>()
            .context("[resolved_links] Collection failed")
    }

    fn extract_rust_parsed_links(&self) -> TestResult<Vec<PdfLink>> {
        self.load_page()?
            .resolved_links_rust_parsed()
            .context("[rust_parsed] Extraction failed")
    }

    fn extract_links_from_annotations(&self) -> TestResult<Vec<PdfLink>> {
        self.load_page()?
            .links_from_annotations_lossy()
            .context("[link_annotations] Extraction failed")
    }

    fn extract_raw_uri_links(&self) -> TestResult<Vec<String>> {
        self.load_page()?
            .links()
            .context("[raw_uri_links] Iterator init failed")
            .map(|iter| iter.map(|link| link.uri).collect())
    }

    /// Saves to memory and reloads, preserving the focus page
    fn reload(&self) -> TestResult<Self> {
        let mut buf = Vec::new();
        self.doc.write_to(&mut buf).context("Failed to write PDF")?;
        Ok(Self {
            doc: PdfDocument::from_bytes(&buf).context("Failed to load PDF from buffer")?,
            page: self.page,
        })
    }

    fn assert_links(&self, expected: &[PdfLink]) -> TestResult<()> {
        self.assert_links_split(expected, expected, expected)
    }

    fn assert_links_split(
        &self,
        expected_resolved: &[PdfLink],
        expected_annots: &[PdfLink],
        expected_uris: &[PdfLink],
    ) -> TestResult<()> {
        self.assert_links_split_impl(expected_resolved, expected_annots, expected_uris)
            .context(format!("[Original PDF, focus page {}]", self.page))?;
        self.reload()?
            .assert_links_split_impl(expected_resolved, expected_annots, expected_uris)
            .context(format!("[Reloaded PDF, focus page {}]", self.page))
    }

    fn assert_links_split_impl(
        &self,
        expected_resolved: &[PdfLink],
        expected_annots: &[PdfLink],
        expected_uris: &[PdfLink],
    ) -> TestResult<()> {
        let resolved = self.extract_links()?;
        assert_slice_eq(&resolved, expected_resolved, "[resolved_links]", "Link")?;

        let rust_parsed = self.extract_rust_parsed_links()?;
        assert_slice_eq(&rust_parsed, expected_resolved, "[rust_parsed]", "Link")?;

        let annots = self.extract_links_from_annotations()?;
        assert_slice_eq(&annots, expected_annots, "[link_annotations]", "Link")?;

        let extracted = self.extract_raw_uri_links()?;
        let expected_uris: Vec<_> = expected_uris.iter().map(|l| l.action.to_string()).collect();
        assert_slice_eq(&extracted, &expected_uris, "[Raw URI match]", "Raw URI")
    }
}

fn add_named_destinations<T>(doc: &mut PdfDocument, names: &[T]) -> Result<(), crate::Error>
where
    T: AsRef<str>,
{
    if names.is_empty() {
        return Ok(());
    }

    let page_obj = doc.find_page(0)?;
    let mut names_array = doc.new_array_with_capacity((names.len() * 2) as i32)?;

    for name in names {
        let mut dest = doc.new_array_with_capacity(6)?;
        dest.array_push_ref(&page_obj)?;
        NAMED_DEST_KIND.encode_into(&mut dest)?;

        names_array.array_push(PdfObject::new_string(name.as_ref())?)?;
        names_array.array_push(dest)?;
    }

    let mut dests = doc.new_dict_with_capacity(1)?;
    dests.dict_put("Names", names_array)?;

    let mut names_dict = doc.new_dict_with_capacity(1)?;
    names_dict.dict_put("Dests", dests)?;

    doc.catalog()?.dict_put("Names", names_dict)?;
    Ok(())
}

fn assert_slice_eq<T>(actual: &[T], expected: &[T], label: &str, item_name: &str) -> TestResult<()>
where
    T: PartialEq + fmt::Debug,
{
    if actual.len() != expected.len() {
        return Err(TestError::msg(format!(
            "{label} {item_name} count mismatch: extracted {}, expected {}",
            actual.len(),
            expected.len()
        )));
    }
    for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
        if a != e {
            return Err(TestError::msg(format!(
                "{label} {item_name} No '{i}' mismatch:\n  extracted: {a:?}\n  expected:  {e:?}"
            )));
        }
    }
    Ok(())
}

/// Convert [`DestinationKind`] into [`PdfDestination`] actions, cycling page indices.
fn page_dests(
    page_count: u32,
    kinds: impl IntoIterator<Item = DestinationKind>,
) -> impl Iterator<Item = PdfDestination> {
    kinds
        .into_iter()
        .enumerate()
        .map(move |(i, kind)| PdfDestination::Page {
            page: (i as u32) % page_count,
            kind,
        })
}

/// Convert DestinationKinds into GoToR actions, cycling page indices
/// and alternating between `path` and `url` file specs.
fn gotor_actions(
    dests: impl IntoIterator<Item = PdfDestination>,
    path: &'static str,
    url: &'static str,
) -> impl Iterator<Item = PdfAction> {
    dests
        .into_iter()
        .enumerate()
        .map(move |(i, dest)| PdfAction::GoToR {
            file: if i % 2 == 0 {
                FileSpec::Path(path.to_owned())
            } else {
                FileSpec::Url(url.to_owned())
            },
            dest,
        })
}

pub(super) const fn xyz_dest(
    left: Option<f32>,
    top: Option<f32>,
    zoom: Option<f32>,
) -> DestinationKind {
    DestinationKind::XYZ { left, top, zoom }
}

pub(super) const fn fit_r_dest(left: f32, bottom: f32, right: f32, top: f32) -> DestinationKind {
    DestinationKind::FitR {
        left,
        bottom,
        right,
        top,
    }
}

#[test]
fn test_url_and_gotor_url() {
    let page_count = 7;
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

    let tester = PdfTester::with_links(1, &links);
    tester.assert_links(&links).unwrap();

    let links = cases
        .into_iter()
        .enumerate()
        .map(|(i, url)| (i as u32 % page_count, FileSpec::Url(format!("{url}.pdf"))))
        .map(|(page, file)| PdfAction::GoToR {
            file,
            dest: PdfDestination::Page {
                page,
                kind: DestinationKind::default(),
            },
        })
        .create_links();

    let tester = PdfTester::with_links(page_count, &links);
    tester.assert_links(&links).unwrap();
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
    let tester = PdfTester::with_links(1, &links);
    tester.assert_links(&links).unwrap();
}

pub(super) const EXPLICIT_DESTS: [DestinationKind; 20] = [
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
    xyz_dest(None, None, None),
    xyz_dest(Some(0.0), Some(0.0), Some(50.0)),
    xyz_dest(Some(100.0), None, None),
    xyz_dest(None, Some(250.0), None),
    xyz_dest(None, None, Some(200.0)),
    xyz_dest(Some(100.0), Some(600.0), Some(150.0)),
    xyz_dest(Some(50.0), Some(700.0), None),
    xyz_dest(Some(50.0), None, Some(300.0)),
    xyz_dest(Some(200.0), Some(PAGE_HEIGHT), Some(100.0)),
    xyz_dest(None, Some(500.0), Some(75.0)),
];

pub(super) fn get_named_str(dest: &PdfDestination) -> &str {
    match dest {
        PdfDestination::Named(name) => name.as_str(),
        _ => unreachable!("Expected PdfDestination::Named"),
    }
}

pub(super) static NAMED_DESTS: LazyLock<Vec<PdfDestination>> = LazyLock::new(|| {
    let mut names: Vec<_> = [
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
    ]
    .into_iter()
    .map(String::from)
    .collect();

    names.push("A".repeat(1024));
    names.into_iter().map(PdfDestination::Named).collect()
});

#[test]
fn test_goto_and_gotor_links() {
    let page_count = 7;

    let links = page_dests(page_count, EXPLICIT_DESTS)
        .map(PdfAction::GoTo)
        .create_links();
    let tester = PdfTester::with_links(page_count, &links);
    tester.assert_links(&links).unwrap();

    let links = gotor_actions(
        page_dests(page_count, EXPLICIT_DESTS),
        "page_destinations.pdf",
        "https://example.com/document.pdf",
    );
    let links = links.create_links();
    let tester = PdfTester::with_links(page_count, &links);
    tester.assert_links(&links).unwrap();
}

#[test]
fn test_fitr_goto_and_gotor_links() {
    let page_count = 3;
    let dests = [PdfDestination::Page {
        page: 1,
        kind: fit_r_dest(50.0, 100.0, 200.0, 300.0),
    }];

    let links = dests.iter().cloned().map(PdfAction::GoTo).create_links();
    let tester = PdfTester::with_links(page_count, &links);
    tester.assert_links(&links).unwrap();

    let links = dests
        .map(|dest| PdfAction::GoToR {
            file: FileSpec::Path("page_destinations.pdf".into()),
            dest,
        })
        .create_links();
    let tester = PdfTester::with_links(page_count, &links);
    tester.assert_links(&links).unwrap();
}

#[test]
fn test_rotated_target_page() {
    // This test combines the CTM checks for landing pages.
    // We check:
    // 1. Exact coordinate hits (full XYZ).
    // 2. Single-axis snapping (FitH, FitV, etc.) taking into account data loss at 90/270 degrees.
    // 3. Partial XYZ coordinates (Some/None) and their behavior when rotated.

    let rotations = [0, 90, 180, 270];
    let mut tester = PdfTester::new(rotations.len() as u32);

    for (idx, &rotation) in rotations.iter().enumerate() {
        tester.on_page(idx).set_rotation(rotation).unwrap();
    }

    for (target_page_idx, &rotation) in rotations.iter().enumerate() {
        tester.new_page(PAGE_SIZE);
        let source_page_idx = tester.page_count() - 1;

        let context_msg = format!(
            "Failed at rotation: {}°, target_page: {}",
            rotation, target_page_idx
        );

        let mut actions = Vec::new();

        let bounds = tester.on_page(target_page_idx).bounds().unwrap();
        let target_page_idx = target_page_idx as u32;
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
                kind: xyz_dest(Some(left), Some(top), Some(100.0)),
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
                kind: xyz_dest(Some(mid_x), None, None),
            }));
            actions.push(PdfAction::GoTo(PdfDestination::Page {
                page: target_page_idx,
                kind: xyz_dest(None, Some(mid_y), None),
            }));
        }
        // These always work (Zoom Only or Empty)
        actions.push(PdfAction::GoTo(PdfDestination::Page {
            page: target_page_idx,
            kind: xyz_dest(None, None, Some(100.0)),
        }));
        actions.push(PdfAction::GoTo(PdfDestination::Page {
            page: target_page_idx,
            kind: xyz_dest(None, None, None),
        }));

        let links = actions.create_links();
        tester.on_page(source_page_idx).add_links(&links).unwrap();
        tester.assert_links(&links).context(context_msg).unwrap();
    }
}

#[test]
fn test_fitr_rotated_target_page() {
    let rotations = [0, 90, 180, 270];
    let mut tester = PdfTester::new(rotations.len() as u32);

    for (idx, &rotation) in rotations.iter().enumerate() {
        tester.on_page(idx).set_rotation(rotation).unwrap();
    }

    for (target_page_idx, &rotation) in rotations.iter().enumerate() {
        tester.new_page(PAGE_SIZE);
        let source_page_idx = tester.page_count() - 1;

        let context_msg = format!(
            "Failed at rotation: {}°, target_page: {}",
            rotation, target_page_idx
        );

        let mut actions = Vec::new();

        let bounds = tester.on_page(target_page_idx).bounds().unwrap();

        let inset = 10.0;
        let min_x = bounds.x0 + inset;
        let min_y = bounds.y0 + inset;
        let max_x = bounds.x1 - inset;
        let max_y = bounds.y1 - inset;
        let mid_x = (min_x + max_x) * 0.5;
        let mid_y = (min_y + max_y) * 0.5;
        let quarter_w = (max_x - min_x) * 0.25;
        let quarter_h = (max_y - min_y) * 0.25;
        let target_page_idx = target_page_idx as u32;
        // Small rect near top-left
        actions.push(PdfAction::GoTo(PdfDestination::Page {
            page: target_page_idx,
            kind: fit_r_dest(min_x, min_y, min_x + quarter_w, min_y + quarter_h),
        }));
        // Rect at center
        actions.push(PdfAction::GoTo(PdfDestination::Page {
            page: target_page_idx,
            kind: fit_r_dest(
                mid_x - quarter_w * 0.5,
                mid_y - quarter_h * 0.5,
                mid_x + quarter_w * 0.5,
                mid_y + quarter_h * 0.5,
            ),
        }));
        // Rect near bottom-right
        actions.push(PdfAction::GoTo(PdfDestination::Page {
            page: target_page_idx,
            kind: fit_r_dest(max_x - quarter_w, max_y - quarter_h, max_x, max_y),
        }));

        let links = actions.clone().create_links();

        tester.on_page(source_page_idx).add_links(&links).unwrap();
        tester.assert_links(&links).context(context_msg).unwrap();
    }
}

#[test]
fn test_rotated_source_page() {
    for source_rotation in [90, 180, 270] {
        let mut tester = PdfTester::new(2);
        tester.set_rotation(source_rotation).unwrap();
        let bounds = tester.bounds().unwrap();

        let inset = 20.0;
        let link_bounds = Rect {
            x0: bounds.x0 + inset,
            y0: bounds.y0 + inset,
            x1: bounds.x0 + inset + 200.0,
            y1: bounds.y0 + inset + 10.0,
        };

        let links = [
            PdfLink {
                bounds: link_bounds,
                action: LinkAction::Action(PdfAction::GoTo(PdfDestination::Page {
                    page: 1,
                    kind: xyz_dest(Some(50.0), Some(400.0), Some(100.0)),
                })),
            },
            PdfLink {
                bounds: Rect {
                    x0: link_bounds.x0,
                    y0: link_bounds.y1 + 5.0,
                    x1: link_bounds.x1,
                    y1: link_bounds.y1 + 15.0,
                },
                action: LinkAction::Action(PdfAction::GoTo(PdfDestination::Page {
                    page: 1,
                    kind: DestinationKind::Fit,
                })),
            },
            PdfLink {
                bounds: Rect {
                    x0: link_bounds.x0,
                    y0: link_bounds.y1 + 20.0,
                    x1: link_bounds.x1,
                    y1: link_bounds.y1 + 30.0,
                },
                action: LinkAction::Action(PdfAction::GoTo(PdfDestination::Page {
                    page: 1,
                    kind: DestinationKind::FitH { top: Some(300.0) },
                })),
            },
            PdfLink {
                bounds: Rect {
                    x0: link_bounds.x0,
                    y0: link_bounds.y1 + 35.0,
                    x1: link_bounds.x1,
                    y1: link_bounds.y1 + 45.0,
                },
                action: LinkAction::Action(PdfAction::Uri("https://example.com".into())),
            },
        ];

        tester.add_links(&links).unwrap();
        tester.assert_links(&links).unwrap();
    }
}

#[test]
fn test_named_goto_and_gotor_links() {
    let page_count = 3;
    let links = NAMED_DESTS
        .iter()
        .cloned()
        .map(PdfAction::GoTo)
        .create_links();

    let resolved =
        repeat_n(LinkAction::Action(NAMED_DEST_RESOLVED), NAMED_DESTS.len()).create_links();

    let names: Vec<&str> = NAMED_DESTS.iter().map(get_named_str).collect();
    let mut tr = PdfTester::with_links(page_count, &links);
    tr.add_named_destinations(&names).unwrap();
    // URI-flattening paths resolve named dests to page numbers. Raw uri and
    // link_annotations preserves Named destinations as written.
    tr.assert_links_split(&resolved, &links, &links).unwrap();

    let links = gotor_actions(
        NAMED_DESTS.iter().cloned(),
        "page_destinations.pdf",
        "https://example.com/document.pdf",
    )
    .create_links();

    let tester = PdfTester::with_links(page_count, &links);
    tester.assert_links(&links).unwrap();
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

    let tester = PdfTester::with_links(1, &links);
    tester.assert_links(&links).unwrap();

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

    let tester = PdfTester::with_links(1, &links);
    tester.assert_links(&links).unwrap();

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
    let clean_links = clean_paths
        .iter()
        .map(|path| PdfAction::Launch(FileSpec::Path(format!("{path}.docx"))))
        .create_links();

    let tr = PdfTester::with_links(1, &links);
    // URI-flattening paths normalize paths via cleanname, link_annotations returns stored paths.
    tr.assert_links_split(&clean_links, &links, &clean_links)
        .unwrap();

    // 3b. GoToR Normalization

    let links = paths
        .iter()
        .map(|path| PdfAction::GoToR {
            file: FileSpec::Path(format!("{path}.pdf")),
            dest: PdfDestination::default(),
        })
        .create_links();
    let clean_links = clean_paths
        .iter()
        .map(|path| PdfAction::GoToR {
            file: FileSpec::Path(format!("{path}.pdf")),
            dest: PdfDestination::default(),
        })
        .create_links();

    let tr = PdfTester::with_links(1, &links);
    // URI-flattening paths normalize paths via cleanname, link_annotations returns stored paths.
    tr.assert_links_split(&clean_links, &links, &clean_links)
        .unwrap();
}

#[test]
fn test_uri_and_lauch() {
    let links = [
        PdfAction::Uri("file:///absolute/path".into()),
        PdfAction::Uri("file://relative/path".into()),
    ]
    .create_links();
    let resolved = [
        PdfAction::Launch(FileSpec::Path("/absolute/path".into())),
        PdfAction::Launch(FileSpec::Path("relative/path".into())),
    ]
    .create_links();

    let tr = PdfTester::with_links(1, &links);
    tr.assert_links_split(&resolved, &links, &links).unwrap()
}

#[test]
fn test_mixed_links() {
    let names = ["Chapter1", "G11.2063217"];
    let named_actions = [
        LinkAction::Action(PdfAction::GoTo(PdfDestination::Named(names[0].to_string()))),
        LinkAction::Dest(PdfDestination::Named(names[1].to_string())),
    ];

    let unnamed_actions = [
        LinkAction::Action(PdfAction::GoTo(PdfDestination::Page {
            page: 2,
            kind: DestinationKind::Fit,
        })),
        LinkAction::Action(PdfAction::GoTo(PdfDestination::Page {
            page: 3,
            kind: xyz_dest(Some(100.0), Some(500.0), Some(150.0)),
        })),
        LinkAction::Action(PdfAction::GoToR {
            file: FileSpec::Path("other.pdf".into()),
            dest: PdfDestination::Page {
                page: 0,
                kind: DestinationKind::Fit,
            },
        }),
        LinkAction::Action(PdfAction::GoToR {
            file: FileSpec::Url("https://example.com/doc.pdf".into()),
            dest: PdfDestination::Page {
                page: 3,
                kind: DestinationKind::FitB,
            },
        }),
        LinkAction::Action(PdfAction::GoToR {
            file: FileSpec::Path("other.pdf".into()),
            dest: PdfDestination::Named("Chapter123".into()),
        }),
        LinkAction::Action(PdfAction::GoToR {
            file: FileSpec::Url("https://example.com/doc.pdf".into()),
            dest: PdfDestination::Named("Chapter123".into()),
        }),
        LinkAction::Action(PdfAction::GoToR {
            file: FileSpec::Url("https://example.com/doc.pdf".into()),
            dest: PdfDestination::Named("G11.2063217mmk".into()),
        }),
        LinkAction::Action(PdfAction::Uri("https://example.com".into())),
        LinkAction::Action(PdfAction::Launch(FileSpec::Path("readme.txt".into()))),
        LinkAction::Dest(PdfDestination::Page {
            page: 1,
            kind: DestinationKind::Fit,
        }),
    ];

    let links = named_actions
        .iter()
        .chain(&unnamed_actions)
        .cloned()
        .create_links();

    let resolved = repeat_n(LinkAction::Action(NAMED_DEST_RESOLVED), named_actions.len())
        .chain(unnamed_actions)
        .map(|action| LinkAction::Action(action.clone().into_pdf_action()))
        .create_links();

    let mut tr = PdfTester::with_links(5, &links);
    tr.add_named_destinations(&names).unwrap();
    // URI-flattening paths resolve named dests to page numbers. Raw uri and
    // link_annotations preserves Named destinations as written.
    // Dest(Named) and GoTo(Named) produce the same URI string (#nameddest=...).
    tr.assert_links_split(&resolved, &links, &links).unwrap();
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
    let links = [PdfAction::GoTo(PdfDestination::Page {
        page: 1,
        kind: xyz_dest(Some(10000.0), Some(10000.0), Some(500.0)),
    })];
    let links = links.create_links();

    let expected_mupdf = [PdfAction::GoTo(PdfDestination::Page {
        page: 1,
        kind: xyz_dest(Some(PAGE_SIZE.width), Some(PAGE_SIZE.height), Some(500.0)),
    })];
    let expected_mupdf = expected_mupdf.create_links();

    let tester = PdfTester::with_links(2, &links);
    let resolved = tester.extract_links().unwrap();
    assert_slice_eq(&resolved, &expected_mupdf, "[resolved_links]", "Link").unwrap();

    let rust_parsed = tester.extract_rust_parsed_links().unwrap();
    assert_slice_eq(&rust_parsed, &links, "[rust_parsed]", "Link").unwrap();

    let annots = tester.extract_links_from_annotations().unwrap();
    assert_slice_eq(&annots, &links, "[link_annotations]", "Link").unwrap();
}

#[test]
fn test_no_links() {
    let tester = PdfTester::with_links(1, &[]);
    tester.assert_links(&[]).unwrap();
}

#[test]
fn test_many_links() {
    let links = (0..100)
        .map(|i| {
            if i % 4 == 0 {
                PdfAction::Uri(format!("https://example.com/page{i}"))
            } else if i % 4 == 1 {
                PdfAction::GoTo(PdfDestination::Page {
                    page: (i % 5) as u32,
                    kind: DestinationKind::Fit,
                })
            } else if i % 4 == 2 {
                PdfAction::Launch(FileSpec::Path(format!("file{i}.txt")))
            } else {
                PdfAction::GoToR {
                    file: FileSpec::Path(format!("doc{i}.pdf")),
                    dest: PdfDestination::default(),
                }
            }
        })
        .create_links();

    let tester = PdfTester::with_links(6, &links);
    tester.assert_links(&links).unwrap();
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

    // MuPDF's URI-flattening path collapses GoToR-with-URL-fragment to Uri,
    // but link_annotations preserves the original GoToR structure.
    let resolved = [
        PdfAction::Uri("http://example.org/doc.pdf#pagemode=bookmarks&page=2&view=Fit".into()),
        PdfAction::Uri("http://example.org/doc.pdf#pagemode=none&nameddest=Chapter1".into()),
    ];
    let resolved = resolved.create_links();

    let tr = PdfTester::with_links(3, &links);
    tr.assert_links_split(&resolved, &links, &resolved).unwrap();
}

#[test]
#[should_panic(
    expected = "PdfPage ownership mismatch: the page is not attached to the provided PdfDocument"
)]
fn test_add_links_with_wrong_document_context() {
    let tester = PdfTester::new(1);

    let mut page = tester.doc.load_pdf_page(0).unwrap();

    let mut doc_alien = PdfDocument::new();

    let links = [PdfAction::Uri("https://fail.com".into())].create_links();
    page.add_links(&mut doc_alien, &links).unwrap();
}

#[test]
fn test_link_with_explicit_dest_action() {
    let page_count = 3;
    let dest_action = page_dests(page_count, EXPLICIT_DESTS)
        .map(LinkAction::Dest)
        .collect::<Vec<_>>();
    let links = dest_action.clone().create_links();

    // URI-flattening paths return Action(GoTo) since they process the stored URI,
    // link_annotations preserves the Dest variant as written.
    let resolved = dest_action
        .into_iter()
        .map(|action| action.into_pdf_action())
        .create_links();

    let tr = PdfTester::with_links(page_count, &links);
    // Dest(Page) and Action(GoTo(Page)) produce the same URI string.
    tr.assert_links_split(&resolved, &links, &resolved).unwrap();
}

#[test]
fn test_link_with_named_dest_action() {
    let links = NAMED_DESTS
        .iter()
        .cloned()
        .map(LinkAction::Dest)
        .create_links();

    let names: Vec<&str> = NAMED_DESTS.iter().map(get_named_str).collect();

    // URI-flattening paths resolve named dests to page destinations,
    // link_annotations preserves the Named variant as written.
    let resolved =
        repeat_n(LinkAction::Action(NAMED_DEST_RESOLVED), NAMED_DESTS.len()).create_links();

    let mut tr = PdfTester::with_links(1, &links);
    tr.add_named_destinations(&names).unwrap();
    // Dest(Named) and GoTo(Named) produce the same URI string (#nameddest=...).
    tr.assert_links_split(&resolved, &links, &links).unwrap();
}

#[test]
fn test_link_action_action_removes_dest() {
    let dest_action = LinkAction::Dest(PdfDestination::Page {
        page: 1,
        kind: DestinationKind::Fit,
    });
    let action_uri = LinkAction::Action(PdfAction::Uri("https://example.com".into()));

    let initial = PdfLink {
        bounds: test_link_rect(0),
        action: dest_action.clone(),
    };
    let action_link = PdfLink {
        bounds: test_link_rect(0),
        action: action_uri.clone(),
    };
    // URI-flattening paths see Dest(Page) as Action(GoTo(Page)) since they process the URI.
    let dest_as_action = PdfLink {
        bounds: test_link_rect(0),
        action: LinkAction::Action(PdfAction::GoTo(PdfDestination::Page {
            page: 1,
            kind: DestinationKind::Fit,
        })),
    };

    let mut tester = PdfTester::with_links(2, slice::from_ref(&initial));

    // Initial state: link_annotations sees Dest, URI-flattening sees Action(GoTo).
    let res = tester.assert_links_split(
        slice::from_ref(&dest_as_action),
        slice::from_ref(&initial),
        slice::from_ref(&dest_as_action),
    );
    res.unwrap();

    // Obtain the link annotation for in-place mutation; drop the page immediately after.
    let mut annot = {
        let page = tester.doc.load_pdf_page(0).unwrap();
        page.link_annotations().unwrap().next().unwrap().unwrap()
    };

    // Switch to Action — should remove /Dest and set /A.
    annot.set_action(&mut tester.doc, &action_uri).unwrap();

    // After switch to Action: all extraction paths see Action(Uri).
    tester.assert_links(slice::from_ref(&action_link)).unwrap();

    // Switch back to Dest — should remove /A and set /Dest.
    annot.set_action(&mut tester.doc, &dest_action).unwrap();

    // After switch back to Dest: link_annotations sees Dest, URI-flattening sees Action(GoTo).
    let res = tester.assert_links_split(
        slice::from_ref(&dest_as_action),
        slice::from_ref(&initial),
        slice::from_ref(&dest_as_action),
    );
    res.unwrap();
}

#[test]
fn test_multiple_add_links_calls() {
    let all_actions = [
        PdfAction::Uri("https://example.com/first".into()),
        PdfAction::GoTo(PdfDestination::Page {
            page: 1,
            kind: DestinationKind::Fit,
        }),
        PdfAction::Launch(FileSpec::Path("readme.txt".into())),
        PdfAction::GoToR {
            file: FileSpec::Path("other.pdf".into()),
            dest: PdfDestination::default(),
        },
    ];

    let all_links = all_actions.create_links();
    let (batch1, batch2) = all_links.split_at(2);

    let mut tester = PdfTester::with_links(3, batch1);
    tester.assert_links(batch1).unwrap();
    tester.add_links(batch2).unwrap();
    tester.assert_links(&all_links).unwrap();

    // Adding an empty batch should not disturb existing links.
    tester.add_links(&[]).unwrap();
    tester.assert_links(&all_links).unwrap();
}

#[test]
fn test_add_links_goto_out_of_range_page() {
    let mut tester = PdfTester::new(3);

    let links = [PdfAction::GoTo(PdfDestination::Page {
        page: 999,
        kind: DestinationKind::Fit,
    })]
    .create_links();

    // add_links calls doc.find_page(999) internally via the resolver.
    // This should fail because page 999 doesn't exist.
    let result = tester.add_links(&links);
    assert!(result.is_err(), "Expected error for out-of-range GoTo page");
}

#[test]
fn test_links_on_non_zero_page() {
    let mut tester = PdfTester::with_sizes(&[PAGE_SIZE, Size::LETTER, PAGE_SIZE]);

    let links = [
        PdfAction::GoTo(PdfDestination::Page {
            page: 0,
            kind: xyz_dest(Some(50.0), Some(400.0), Some(100.0)),
        }),
        PdfAction::GoTo(PdfDestination::Page {
            page: 2,
            kind: DestinationKind::Fit,
        }),
        PdfAction::Uri("https://example.com".into()),
    ]
    .create_links();

    // Add links to page 1 (LETTER size) instead of page 0.
    tester.on_page(1).add_links(&links).unwrap();

    // Extract from page 1.
    let extracted = tester.extract_links().unwrap();
    assert_eq!(extracted, links);

    let from_annots = tester.extract_links_from_annotations().unwrap();
    assert_eq!(from_annots, links);

    let mut tester = PdfTester::new(5);

    let links = [PdfAction::GoTo(PdfDestination::Page {
        page: 0,
        kind: DestinationKind::Fit,
    })]
    .create_links();

    let last_page_idx = tester.page_count() - 1;
    tester.on_page(last_page_idx).add_links(&links).unwrap();

    let extracted = tester.extract_links().unwrap();
    assert_eq!(extracted, links);
}

#[test]
fn test_link_annotations_filters_non_link_annots() {
    let links = [PdfAction::Uri("https://example.com".into())].create_links();
    let tester = PdfTester::with_links(1, &links);

    // Add a non-link annotation (Text) via MuPDF's annotation API.
    {
        let mut page = tester.doc.load_pdf_page(0).unwrap();
        page.create_annotation(crate::pdf::PdfAnnotationType::Text)
            .unwrap();
    }

    let page = tester.doc.load_pdf_page(0).unwrap();

    // link_annotations() should yield only the link, not the text annotation.
    let link_annots: Vec<_> = page
        .link_annotations()
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(link_annots.len(), 1);
    let from_annots = page.links_from_annotations_lossy().unwrap();
    assert_eq!(from_annots, links);
}

#[test]
fn test_negative_coordinates() {
    let page_count = 3;

    let original_kinds = [
        xyz_dest(Some(-50.0), Some(-100.0), Some(100.0)),
        DestinationKind::FitH { top: Some(-200.0) },
        DestinationKind::FitV { left: Some(-150.0) },
        DestinationKind::FitBH { top: Some(-300.0) },
        DestinationKind::FitBV { left: Some(-250.0) },
        fit_r_dest(-10.0, -20.0, -5.0, -15.0),
    ];

    let clamped_kinds = [
        xyz_dest(Some(0.0), Some(0.0), Some(100.0)),
        DestinationKind::FitH { top: Some(0.0) },
        DestinationKind::FitV { left: Some(0.0) },
        DestinationKind::FitBH { top: Some(0.0) },
        DestinationKind::FitBV { left: Some(0.0) },
        fit_r_dest(0.0, 0.0, 5.0, 5.0),
    ];

    let links = page_dests(page_count, original_kinds)
        .map(PdfAction::GoTo)
        .create_links();

    let tester = PdfTester::with_links(page_count, &links);

    let from_annots = tester.extract_links_from_annotations().unwrap();
    assert_eq!(from_annots, links);

    let clamped_links = page_dests(page_count, clamped_kinds)
        .map(PdfAction::GoTo)
        .create_links();

    let mupdf_links = tester.extract_links().unwrap();
    assert_eq!(mupdf_links, clamped_links);
}

#[test]
fn test_empty_named_destination() {
    let links = [PdfAction::GoToR {
        file: FileSpec::Path("other.pdf".into()),
        dest: PdfDestination::Named(String::new()),
    }];
    let links = links.create_links();

    let resolved = [PdfAction::GoToR {
        file: FileSpec::Path("other.pdf".to_owned()),
        dest: PdfDestination::default(),
    }];
    let resolved = resolved.create_links();

    let tr = PdfTester::with_links(1, &links);
    // URI-flattening paths resolve empty named dests to default PdfDestination. Raw uri and
    // link_annotations preserves Named destinations as written.
    tr.assert_links_split(&resolved, &links, &links).unwrap();
}

#[test]
fn test_goto_self_page() {
    let links = [PdfAction::GoTo(PdfDestination::Page {
        page: 0,
        kind: xyz_dest(Some(50.0), Some(100.0), Some(200.0)),
    })]
    .create_links();

    let tester = PdfTester::with_links(1, &links);
    tester.assert_links(&links).unwrap();
}

#[test]
fn test_goto_last_page() {
    let page_count = 10;
    let links = [PdfAction::GoTo(PdfDestination::Page {
        page: (page_count - 1) as u32,
        kind: DestinationKind::Fit,
    })]
    .create_links();

    let tester = PdfTester::with_links(page_count, &links);
    tester.assert_links(&links).unwrap();
}

#[test]
fn test_links_on_different_page_sizes() {
    let sizes = [Size::A4, Size::LETTER, Size::A3, Size::new(200.0, 300.0)];
    let mut tester = PdfTester::with_sizes(&sizes);

    let links = [PdfAction::GoTo(PdfDestination::Page {
        page: 0,
        kind: xyz_dest(Some(10.0), Some(20.0), Some(100.0)),
    })];
    let links = links.create_links();

    for page_idx in 0..sizes.len() {
        tester.on_page(page_idx).add_links(&links).unwrap();
        tester.assert_links(&links).unwrap();
    }
}

#[test]
fn test_named_dest_with_extra_keys() {
    let page_count = 3;
    let name = "Chapter1&foo=bar";

    let links = [PdfAction::GoTo(PdfDestination::Named(name.into()))].create_links();

    let resolved = repeat_n(LinkAction::Action(NAMED_DEST_RESOLVED), 1).create_links();

    let mut tr = PdfTester::with_links(page_count, &links);
    tr.add_named_destinations(&[name]).unwrap();

    // URI-flattening paths resolve named dests to page numbers. Raw URIs and
    // link annotations preserve named destinations as written. Here we test that named
    // destinations containing URI control characters, like "Chapter1&foo=bar", are resolvable.
    // This succeeds (except for the `links_from_annotations_lossy` call) because MuPDF uses
    // `fz_encode_uri_component`. Thus, "Chapter1&foo=bar" becomes "Chapter1%26foo%3Dbar",
    // which prevents the parser from inappropriately splitting on the `&` character.
    tr.assert_links_split(&resolved, &links, &links).unwrap();

    let extracted = tr.extract_raw_uri_links().unwrap();
    assert_eq!(&extracted, &["#nameddest=Chapter1%26foo%3Dbar"]);

    // However, if we provide the unescaped string "#nameddest=Chapter1&foo=bar" directly
    // to the Rust parsing logic, the extra keys are stripped/clipped. This exactly matches
    // the behavior of MuPDF's `parse_uri_named_dest` function.
    let action = parse_external_link("#nameddest=Chapter1&foo=bar");
    let expected = PdfAction::GoTo(PdfDestination::Named("Chapter1".into()));
    assert_eq!(action, Some(expected));
}

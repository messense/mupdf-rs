use std::collections::HashMap;

use super::build::{build_link_annotation, set_link_action_on_annot_dict};
use super::tests_build::{
    test_link_rect, EXPLICIT_DESTS, NAMED_DESTS, PAGE_HEIGHT, PAGE_SIZE, PAGE_WIDTH,
};
use super::{
    extraction::parse_link_action_from_annot_dict, CachedResolver, DestPageResolver, FileSpec,
    LinkAction, PdfAction, PdfDestination, PdfLink, SingleResolver,
};
use crate::pdf::links::tests_build::xyz_dest;
use crate::pdf::{PdfDocument, PdfLinkAnnot, PdfObject};
use crate::{DestinationKind, Error, Matrix, Rect};

/// Helper: creates a doc with `page_count` pages, builds an annot dict via `setup`,
/// injects it on page 0, parses the action, and returns it.
struct LinkAnnotTester {
    doc: PdfDocument,
    page: Option<i32>,
}

impl LinkAnnotTester {
    #[track_caller]
    fn new(page_count: usize) -> Self {
        let mut doc = PdfDocument::new();
        for _ in 0..page_count {
            doc.new_page(PAGE_SIZE).unwrap();
        }
        Self { doc, page: None }
    }

    /// Sets the focus page for subsequent operations
    fn on_page(&mut self, page: i32) -> &mut Self {
        self.page = Some(page);
        self
    }

    #[track_caller]
    fn build_annotation_object(&mut self, setup: AnnotSetup) -> PdfObject {
        let mut page_obj = self.doc.find_page(self.page.unwrap_or_default()).unwrap();
        let mut resolver = noop_resolver();
        let mut annot = build_link_annotation(
            &mut self.doc,
            &page_obj,
            &dummy_link(),
            &None,
            &mut resolver,
        )
        .unwrap();
        let _ = annot.dict_delete("A");
        setup.apply(&mut self.doc, &mut annot).unwrap();
        self.inject_annot(&mut page_obj, &annot);
        annot
    }

    #[track_caller]
    fn parse_annotation_object(&self, annot: &PdfObject) -> Option<LinkAction> {
        parse_link_action_from_annot_dict(annot, &self.doc, self.page).unwrap()
    }

    /// Validates both parsed action and MuPDF URI output.
    #[track_caller]
    fn build_and_assert_annot_objects(&mut self, setup: AnnotSetup, expected: &LinkAction) {
        let annot = self.build_annotation_object(setup);
        let result = self.parse_annotation_object(&annot);
        assert_eq!(result.as_ref(), Some(expected));
        assert_eq!(
            self.read_first_mupdf_link_uri(),
            expected.to_uri(),
            "MuPDF uri vs expected uri"
        );
    }

    fn doc_mut(&mut self) -> &mut PdfDocument {
        &mut self.doc
    }

    /// Encodes a [`LinkAction`] into its raw PDF action/destination object
    /// using production code ([`set_link_action_on_annot_dict`]).
    #[track_caller]
    fn encode_action(&mut self, action: &LinkAction) -> PdfObject {
        let mut dict = self.doc.new_dict_with_capacity(1).unwrap();
        let mut resolver = noop_resolver();
        set_link_action_on_annot_dict(&mut self.doc, &mut dict, action, &mut resolver).unwrap();
        match action {
            LinkAction::Action(_) => dict.get_dict("A").unwrap().unwrap(),
            LinkAction::Dest(_) => dict.get_dict("Dest").unwrap().unwrap(),
        }
    }

    /// Injects an annotation dict into a page's /Annots array.
    #[track_caller]
    fn inject_annot(&mut self, page_obj: &mut PdfObject, annot: &PdfObject) {
        let mut annots = match page_obj.get_dict("Annots").unwrap() {
            Some(a) if a.is_array().unwrap() => a,
            _ => self.doc.new_array_with_capacity(4).unwrap(),
        };

        let indirect = self.doc.add_object(annot).unwrap();
        annots.array_push(indirect).unwrap();
        page_obj.dict_put("Annots", annots).unwrap();
    }

    /// Builds a Named action dict (e.g. FirstPage, LastPage, PrevPage, NextPage).
    #[track_caller]
    fn make_named_action_dict(&mut self, name: &str) -> PdfObject {
        let mut action_dict = self.doc.new_dict_with_capacity(2).unwrap();
        action_dict
            .dict_put("S", PdfObject::new_name("Named").unwrap())
            .unwrap();
        action_dict
            .dict_put("N", PdfObject::new_name(name).unwrap())
            .unwrap();
        action_dict
    }

    /// Sets Root/URI/Base on the document catalog.
    #[track_caller]
    fn set_uri_base(&mut self, base: &str) {
        let mut uri_dict = self.doc.new_dict_with_capacity(1).unwrap();
        uri_dict
            .dict_put("Base", PdfObject::new_string(base).unwrap())
            .unwrap();
        self.doc
            .catalog()
            .unwrap()
            .dict_put("URI", uri_dict)
            .unwrap();
    }

    /// Adds a placeholder link and returns its [`PdfLinkAnnot`].
    #[track_caller]
    fn add_link_annot(&mut self) -> PdfLinkAnnot {
        let page_no = self.page.unwrap_or_default();
        let mut page = self.doc.load_pdf_page(page_no).unwrap();
        page.add_links(&mut self.doc, &[dummy_link()]).unwrap();
        self.link_annot()
    }

    /// Returns the first link annotation from focused page.
    fn link_annot(&self) -> PdfLinkAnnot {
        let page_no = self.page.unwrap_or_default();
        let page = self.doc.load_pdf_page(page_no).unwrap();
        page.link_annotations().unwrap().next().unwrap().unwrap()
    }

    /// Returns the URI string that MuPDF's native link loader produces for the
    /// first link on focused page.
    #[track_caller]
    fn read_first_mupdf_link_uri(&self) -> String {
        let page_no = self.page.unwrap_or_default();
        let mut links = self.doc.load_page(page_no).unwrap().links().unwrap();
        let link = links.next().expect("At least one link was expected");
        assert!(links.next().is_none(), "MuPDF returned multiple elements");
        link.uri
    }

    /// Sets an action via both [`PdfLinkAnnot::set_action`] and
    /// [`PdfLinkAnnot::set_action_with_resolver`], asserting
    /// equality and MuPDF URI match after each write.
    #[track_caller]
    fn set_action_and_assert(
        &mut self,
        annot: &mut PdfLinkAnnot,
        cache: &mut HashMap<u32, (PdfObject, Option<Matrix>)>,
        action: &LinkAction,
        label: &str,
    ) {
        annot.set_action(&mut self.doc, action).unwrap();
        assert_eq!(
            self.link_annot().action(&self.doc, None).unwrap(),
            Some(action.clone()),
            "[{label}], set_action test"
        );
        assert_eq!(
            self.read_first_mupdf_link_uri(),
            action.to_uri(),
            "[{label}], set_action raw uri comparison"
        );

        let mut resolver = CachedResolver::new(cache, resolve_page_inv_ctm);
        annot
            .set_action_with_resolver(&mut self.doc, action, &mut resolver)
            .unwrap();
        assert_eq!(
            self.link_annot().action(&self.doc, None).unwrap().as_ref(),
            Some(action),
            "[{label}], set_action_with_resolver test"
        );
        assert_eq!(
            self.read_first_mupdf_link_uri(),
            action.to_uri(),
            "[{label}], set_action_with_resolver raw uri comparison"
        );
    }
}

#[derive(Default)]
struct AnnotSetup {
    dest: Option<PdfObject>,
    action: Option<PdfObject>,
    aa_d: Option<PdfObject>,
    aa_u: Option<PdfObject>,
}

impl AnnotSetup {
    fn new() -> Self {
        Self::default()
    }

    fn with_dest(mut self, value: PdfObject) -> Self {
        self.dest = Some(value);
        self
    }

    fn with_action(mut self, value: PdfObject) -> Self {
        self.action = Some(value);
        self
    }

    fn with_aa_d(mut self, value: PdfObject) -> Self {
        self.aa_d = Some(value);
        self
    }

    fn with_aa_u(mut self, value: PdfObject) -> Self {
        self.aa_u = Some(value);
        self
    }

    fn apply(self, doc: &mut PdfDocument, annot: &mut PdfObject) -> Result<(), Error> {
        if let Some(dest) = self.dest {
            annot.dict_put("Dest", dest)?;
        }
        if let Some(action) = self.action {
            annot.dict_put("A", action)?;
        }

        if self.aa_d.is_some() || self.aa_u.is_some() {
            let mut aa = doc.new_dict_with_capacity(2)?;
            if let Some(d) = self.aa_d {
                aa.dict_put("D", d)?;
            }
            if let Some(u) = self.aa_u {
                aa.dict_put("U", u)?;
            }
            annot.dict_put("AA", aa)?;
        }

        Ok(())
    }
}

fn dummy_link() -> PdfLink {
    PdfLink {
        bounds: test_link_rect(0),
        action: LinkAction::Action(PdfAction::Uri("https://dummy.test".into())),
    }
}

fn noop_resolver() -> SingleResolver<impl FnMut(&PdfObject) -> Result<Option<Matrix>, Error>> {
    SingleResolver::new(|_: &PdfObject| Ok(None))
}

fn resolve_page_inv_ctm(page_obj: &PdfObject) -> Result<Option<Matrix>, Error> {
    Ok(page_obj.page_ctm()?.invert())
}

#[test]
fn test_set_action_explicit_page_destinations() {
    // Three pages so that target page indices 0–2 are all valid.
    let mut tester = LinkAnnotTester::new(3);
    let mut annot = tester.add_link_annot();
    let mut cache: HashMap<u32, (PdfObject, Option<Matrix>)> = HashMap::new();

    for (index, kind) in EXPLICIT_DESTS.into_iter().enumerate() {
        let page = (index % 3) as u32;

        // 1. LinkAction::Action(GoTo(Page{..}))
        let action = LinkAction::Action(PdfAction::GoTo(PdfDestination::Page { page, kind }));
        let label = format!("GoTo index: {index}");
        tester.set_action_and_assert(&mut annot, &mut cache, &action, &label);
        assert!(cache.contains_key(&page));

        // 2. LinkAction::Action(GoToR { ..., Page{..} })
        let file_spec = if index % 2 == 0 {
            FileSpec::Path("page_destinations.pdf".into())
        } else {
            FileSpec::Url("https://example.com/document.pdf".into())
        };
        let action = LinkAction::Action(PdfAction::GoToR {
            file: file_spec,
            dest: PdfDestination::Page { page, kind },
        });
        let cache_len_before = cache.len();
        let label = format!("GoToR index: {index}");
        tester.set_action_and_assert(&mut annot, &mut cache, &action, &label);
        assert_eq!(cache.len(), cache_len_before);

        // 3. LinkAction::Dest(Page{..})
        let action = LinkAction::Dest(PdfDestination::Page { page, kind });
        let label = format!("Dest index: {index}");
        tester.set_action_and_assert(&mut annot, &mut cache, &action, &label);
        assert!(cache.contains_key(&page));
    }
}

#[test]
fn test_set_action_named_destinations() {
    let mut tester = LinkAnnotTester::new(1);
    let mut annot = tester.add_link_annot();
    let mut cache: HashMap<u32, (PdfObject, Option<Matrix>)> = HashMap::new();

    for (index, dest) in NAMED_DESTS.iter().enumerate() {
        // 1. LinkAction::Action(GoTo(Named))
        let action = LinkAction::Action(PdfAction::GoTo(dest.clone()));
        let label = format!("GoTo index: {index}");
        tester.set_action_and_assert(&mut annot, &mut cache, &action, &label);

        // 2. LinkAction::Action(GoToR { ..., dest: Named })
        let file_spec = if index % 2 == 0 {
            FileSpec::Path("page_destinations.pdf".into())
        } else {
            FileSpec::Url("https://example.com/document.pdf".into())
        };
        let action = LinkAction::Action(PdfAction::GoToR {
            file: file_spec,
            dest: dest.clone(),
        });
        let label = format!("GoToR index: {index}");
        tester.set_action_and_assert(&mut annot, &mut cache, &action, &label);

        // 3. LinkAction::Dest(Named)
        let action = LinkAction::Dest(dest.clone());
        let label = format!("Dest index: {index}");
        tester.set_action_and_assert(&mut annot, &mut cache, &action, &label);
    }
    assert!(cache.is_empty());
}

#[test]
fn test_set_action_misc() {
    let mut tester = LinkAnnotTester::new(1);
    let mut annot = tester.add_link_annot();

    let cases = [
        LinkAction::Action(PdfAction::GoTo(PdfDestination::Named("Chapter1".into()))),
        LinkAction::Action(PdfAction::GoTo(PdfDestination::Named("Section.2.3".into()))),
        LinkAction::Action(PdfAction::GoToR {
            file: FileSpec::Path("other.pdf".into()),
            dest: PdfDestination::Page {
                page: 0,
                kind: DestinationKind::Fit,
            },
        }),
        LinkAction::Action(PdfAction::GoToR {
            file: FileSpec::Path("other.pdf".into()),
            dest: PdfDestination::Named("Appendix".into()),
        }),
        LinkAction::Action(PdfAction::GoToR {
            file: FileSpec::Url("https://example.com/doc.pdf".into()),
            dest: PdfDestination::Page {
                page: 2,
                kind: xyz_dest(Some(100.0), Some(200.0), Some(150.0)),
            },
        }),
        LinkAction::Action(PdfAction::GoToR {
            file: FileSpec::Url("https://example.com/doc.pdf".into()),
            dest: PdfDestination::Named("Table1".into()),
        }),
        LinkAction::Action(PdfAction::Launch(FileSpec::Path("readme.txt".into()))),
        LinkAction::Action(PdfAction::Launch(FileSpec::Url(
            "https://example.com/report.pdf".into(),
        ))),
        LinkAction::Action(PdfAction::Uri("https://example.com".into())),
        LinkAction::Action(PdfAction::Uri("mailto:user@example.com".into())),
    ];

    for (index, action) in cases.into_iter().enumerate() {
        annot.set_action(tester.doc_mut(), &action).unwrap();
        let result = tester.link_annot().action(&tester.doc, None).unwrap();
        // Skip Launch(Url): MuPDF's pdf_parse_link_dest_to_file_with_uri doesn't
        // handle Launch with FileSpec::Url
        if !matches!(
            &action,
            LinkAction::Action(PdfAction::Launch(FileSpec::Url(_)))
        ) {
            assert_eq!(
                tester.read_first_mupdf_link_uri(),
                action.to_uri(),
                "mupdf uri, index: {index}"
            );
        }
        assert_eq!(result, Some(action), "index: {index}");
    }
}

#[test]
fn test_parse_link_action_errors_on_out_of_range_page_index() {
    let mut tester = LinkAnnotTester::new(3);

    let mut dest_array = tester.doc.new_array_with_capacity(2).unwrap();
    dest_array
        .array_push(PdfObject::new_int(9999).unwrap())
        .unwrap();
    dest_array
        .array_push(PdfObject::new_name("Fit").unwrap())
        .unwrap();

    let mut action_dict = tester.doc.new_dict_with_capacity(2).unwrap();
    action_dict
        .dict_put("S", PdfObject::new_name("GoTo").unwrap())
        .unwrap();
    action_dict.dict_put("D", dest_array).unwrap();

    // Rust fails on out-of-range pages
    let annot = tester.build_annotation_object(AnnotSetup::new().with_action(action_dict));

    // MuPDF simply drops the invalid link
    assert!(parse_link_action_from_annot_dict(&annot, &tester.doc, None).is_err());
    let links = tester.doc.load_page(0).unwrap().links().unwrap();
    assert_eq!(links.count(), 0);
}

#[test]
fn test_switches_between_dest_and_action_entries() {
    let mut tester = LinkAnnotTester::new(2);
    let mut annot = tester.add_link_annot();

    let as_dest = LinkAction::Dest(PdfDestination::Page {
        page: 1,
        kind: DestinationKind::Fit,
    });
    let as_action = LinkAction::Action(PdfAction::Uri("https://example.com".into()));

    // Write /Dest — /A must be absent.
    annot.set_action(tester.doc_mut(), &as_dest).unwrap();
    assert!(annot.get_dict("Dest").unwrap().is_some());
    assert!(
        annot.get_dict("A").unwrap().is_none(),
        "/A should be absent after writing /Dest"
    );

    // Switch to /A — /Dest must be cleared.
    annot.set_action(tester.doc_mut(), &as_action).unwrap();
    assert!(
        annot.get_dict("Dest").unwrap().is_none(),
        "/Dest should be absent after writing /A"
    );
    assert!(annot.get_dict("A").unwrap().is_some());

    // Switch back to /Dest — /A must be cleared again.
    annot.set_action(tester.doc_mut(), &as_dest).unwrap();
    assert!(annot.get_dict("Dest").unwrap().is_some());
    assert!(
        annot.get_dict("A").unwrap().is_none(),
        "/A should be absent after writing /Dest"
    );

    let mut tester = LinkAnnotTester::new(2);
    let aa_d = tester.encode_action(&LinkAction::Action(PdfAction::GoTo(
        PdfDestination::default(),
    )));
    let aa_u = tester.encode_action(&LinkAction::Action(PdfAction::Uri(
        "https://other_example.com".into(),
    )));
    let annot_obj =
        tester.build_annotation_object(AnnotSetup::new().with_aa_d(aa_d).with_aa_u(aa_u));
    let mut annot = PdfLinkAnnot::new(annot_obj);

    annot.set_action(tester.doc_mut(), &as_action).unwrap();
    assert!(
        annot.get_dict("AA").unwrap().is_none(),
        "/AA should be absent after replacing with /A"
    );
    assert_eq!(
        annot.action(&tester.doc, None).unwrap(),
        Some(as_action.clone())
    );

    annot.set_action(tester.doc_mut(), &as_dest).unwrap();
    assert!(
        annot.get_dict("AA").unwrap().is_none(),
        "/AA should stay absent"
    );
    assert_eq!(annot.action(&tester.doc, None).unwrap(), Some(as_dest));
}

/// `set_rect` — with `None` CTM writes coordinates as-is.
#[test]
fn test_set_rect_without_ctm_preserves_coordinates() {
    let mut tester = LinkAnnotTester::new(1);
    let mut annot = tester.add_link_annot();

    let cases = [
        Rect::new(10.0, 20.0, 200.0, 80.0),
        Rect::new(0.0, 0.0, PAGE_WIDTH, PAGE_HEIGHT),
        Rect::new(100.0, 300.0, 400.0, 600.0),
        // Coordinates outside the page are stored verbatim.
        Rect::new(-200.0, -50.0, PAGE_WIDTH + 200.0, PAGE_HEIGHT + 50.0),
        // Zero-area rect.
        Rect::new(50.0, 50.0, 50.0, 50.0),
    ];

    for (index, expected) in cases.into_iter().enumerate() {
        annot.set_rect(tester.doc_mut(), expected, None).unwrap();
        let actual = annot.rect(None).unwrap();
        assert_eq!(actual, expected, "index: {index}");
    }
}

/// `set_rect` — with a non-identity CTM transforms coordinates correctly.
#[test]
fn test_set_rect_with_ctm() {
    let mut tester = LinkAnnotTester::new(1);
    let mut annot = tester.add_link_annot();

    let (page_ctm, inv_ctm) = {
        let page = tester.doc_mut().find_page(0).unwrap();
        let ctm = page.page_ctm().unwrap();
        let inv = ctm.invert().expect("page CTM must be invertible");
        (ctm, inv)
    };

    let fitz_rects = [
        Rect::new(50.0, 50.0, 250.0, 100.0),
        // Full page bounds in Fitz space.
        Rect::new(0.0, 0.0, PAGE_WIDTH, PAGE_HEIGHT),
        Rect::new(100.0, 200.0, 400.0, 600.0),
        // Near bottom of Fitz page (large y).
        Rect::new(10.0, 730.0, PAGE_WIDTH - 10.0, PAGE_HEIGHT - 2.0),
    ];

    for (index, expected) in fitz_rects.into_iter().enumerate() {
        annot
            .set_rect(tester.doc_mut(), expected, Some(&inv_ctm))
            .unwrap();
        let actual = tester.link_annot().rect(Some(&page_ctm)).unwrap();
        assert_eq!(actual, expected, "index: {index}");
    }
}

#[test]
fn test_parse_action_aa_fallback_order() {
    // 1. /AA/D only
    let mut tester = LinkAnnotTester::new(2);
    let action = LinkAction::Action(PdfAction::GoTo(PdfDestination::Page {
        page: 1,
        kind: DestinationKind::Fit,
    }));
    let action_dict = tester.encode_action(&action);
    tester.build_and_assert_annot_objects(AnnotSetup::new().with_aa_d(action_dict), &action);

    // 2. /AA/U only (when /AA/D absent)
    let mut tester = LinkAnnotTester::new(1);
    let action = LinkAction::Action(PdfAction::Uri("https://example.com".into()));
    let uri_dict = tester.encode_action(&action);
    tester.build_and_assert_annot_objects(AnnotSetup::new().with_aa_u(uri_dict), &action);

    // 3. /AA/D takes priority over /AA/U
    let mut tester = LinkAnnotTester::new(2);
    let d_action = LinkAction::Action(PdfAction::GoTo(PdfDestination::Page {
        page: 1,
        kind: DestinationKind::Fit,
    }));
    let d_dict = tester.encode_action(&d_action);
    let u_dict = tester.encode_action(&LinkAction::Action(PdfAction::Uri(
        "https://u-action.example.com".into(),
    )));
    tester.build_and_assert_annot_objects(
        AnnotSetup::new().with_aa_d(d_dict).with_aa_u(u_dict),
        &d_action,
    );
}

#[test]
fn test_parse_action_priority_dest_over_a_over_aa() {
    let action = LinkAction::Action(PdfAction::Uri("https://a-entry.example.com".into()));

    // 1. /Dest takes priority over /A
    let mut tester = LinkAnnotTester::new(2);
    let a_obj = tester.encode_action(&action);
    let dest_action = LinkAction::Dest(PdfDestination::Page {
        page: 1,
        kind: DestinationKind::Fit,
    });
    let dest_obj = tester.encode_action(&dest_action);
    tester.build_and_assert_annot_objects(
        AnnotSetup::new().with_dest(dest_obj).with_action(a_obj),
        &dest_action,
    );

    // 2. /A takes priority over /AA
    let mut tester = LinkAnnotTester::new(2);
    let a_obj = tester.encode_action(&action);
    let aa_obj = tester.encode_action(&LinkAction::Action(PdfAction::GoTo(PdfDestination::Page {
        page: 1,
        kind: DestinationKind::Fit,
    })));
    tester.build_and_assert_annot_objects(
        AnnotSetup::new().with_action(a_obj).with_aa_d(aa_obj),
        &action,
    );

    // 3. No /Dest, /A, or /AA returns None
    let mut tester = LinkAnnotTester::new(1);
    let annot = tester.build_annotation_object(AnnotSetup::new());
    let result = tester.parse_annotation_object(&annot);
    assert!(result.is_none(), "No action should return None");
}

#[test]
fn test_parse_named_actions_to_page_destinations() {
    // (name, page_count, page_num, expected_page)
    let cases: &[(&str, usize, Option<i32>, Option<u32>)] = &[
        ("FirstPage", 5, Some(2), Some(0)),
        ("LastPage", 5, Some(0), Some(4)),
        ("PrevPage", 5, Some(3), Some(2)),
        ("PrevPage", 5, Some(0), Some(0)), // clamps to 0
        ("NextPage", 5, Some(2), Some(3)),
        ("NextPage", 5, Some(4), Some(4)),   // clamps to last
        ("PrevPage", 5, None, None),         // None page_num -> None
        ("NextPage", 5, None, None),         // None page_num -> None
        ("UnknownAction", 1, Some(0), None), // unknown -> None
    ];

    for (name, page_count, page_num, expected) in cases {
        let mut tester = LinkAnnotTester::new(*page_count);
        let action_dict = tester.make_named_action_dict(name);
        if let Some(page) = page_num {
            tester.on_page(*page);
        }
        let annot = tester.build_annotation_object(AnnotSetup::new().with_action(action_dict));
        let result = tester.parse_annotation_object(&annot);

        match expected {
            Some(expected_page) => {
                let expected_action = LinkAction::Action(PdfAction::GoTo(PdfDestination::Page {
                    page: *expected_page,
                    kind: DestinationKind::default(),
                }));
                assert_eq!(
                    result,
                    Some(expected_action),
                    "{name} from page {page_num:?} should go to page {expected_page}"
                );
            }
            None => assert!(
                result.is_none(),
                "{name} with page_num={page_num:?} should return None"
            ),
        }
    }
}

#[test]
fn test_parse_uri_action_with_catalog_base() {
    // (base_uri, link_uri, expected_full_uri)
    let cases: &[(Option<&str>, &str, &str)] = &[
        (
            Some("https://example.com/base/"),
            "page.html",
            "https://example.com/base/page.html",
        ),
        (None, "/absolute/path", "file:///absolute/path"),
        (None, "relative/path", "file://relative/path"),
        (
            Some("https://base.example.com/"),
            "https://other.example.com/page",
            "https://other.example.com/page",
        ),
    ];

    for (base, link_uri, expected) in cases {
        let mut tester = LinkAnnotTester::new(1);
        if let Some(base) = base {
            tester.set_uri_base(base);
        }

        let action_dict =
            tester.encode_action(&LinkAction::Action(PdfAction::Uri(link_uri.to_string())));

        let expected_action = LinkAction::Action(PdfAction::Uri(expected.to_string()));
        tester.build_and_assert_annot_objects(
            AnnotSetup::new().with_action(action_dict),
            &expected_action,
        );
    }
}

#[test]
fn test_error_for_missing_or_incomplete_rect() {
    let mut doc = PdfDocument::new();
    doc.new_page(PAGE_SIZE).unwrap();

    // 1. Missing /Rect
    let mut annot_obj = doc.new_dict_with_capacity(2).unwrap();
    annot_obj
        .dict_put("Type", PdfObject::new_name("Annot").unwrap())
        .unwrap();
    annot_obj
        .dict_put("Subtype", PdfObject::new_name("Link").unwrap())
        .unwrap();
    let annot = super::PdfLinkAnnot::new(annot_obj);
    assert!(annot.rect(None).is_err());

    // 2. Incomplete /Rect (only 2 elements)
    let mut annot_obj = doc.new_dict_with_capacity(3).unwrap();
    annot_obj
        .dict_put("Type", PdfObject::new_name("Annot").unwrap())
        .unwrap();
    annot_obj
        .dict_put("Subtype", PdfObject::new_name("Link").unwrap())
        .unwrap();
    let mut rect = doc.new_array_with_capacity(2).unwrap();
    rect.array_push(PdfObject::new_real(10.0).unwrap()).unwrap();
    rect.array_push(PdfObject::new_real(20.0).unwrap()).unwrap();
    annot_obj.dict_put("Rect", rect).unwrap();
    let annot = super::PdfLinkAnnot::new(annot_obj);
    assert!(annot.rect(None).is_err());
}

#[test]
fn test_parse_dest_entry_name_and_string_objects() {
    // /Dest as a PDF name
    let mut tester = LinkAnnotTester::new(1);
    let expected = LinkAction::Dest(PdfDestination::Named("Chapter1".into()));
    let dest = PdfObject::new_name("Chapter1").unwrap();
    tester.build_and_assert_annot_objects(AnnotSetup::new().with_dest(dest), &expected);

    // /Dest as a PDF string
    let mut tester = LinkAnnotTester::new(1);
    let expected = LinkAction::Dest(PdfDestination::Named("Section.2.3".into()));
    let dest = PdfObject::new_string("Section.2.3").unwrap();
    tester.build_and_assert_annot_objects(AnnotSetup::new().with_dest(dest), &expected);
}

#[test]
fn test_parse_gotor_filespec_string() {
    // 1. File spec as a raw string (not dict)
    let mut tester = LinkAnnotTester::new(1);
    let mut action_dict = tester.encode_action(&LinkAction::Action(PdfAction::GoToR {
        file: FileSpec::Path("simple.pdf".into()),
        dest: PdfDestination::default(),
    }));
    let path = PdfObject::new_string("other.pdf").unwrap();
    action_dict.dict_put("F", path).unwrap();
    let expected = LinkAction::Action(PdfAction::GoToR {
        file: FileSpec::Path("other.pdf".into()),
        dest: PdfDestination::default(),
    });
    tester.build_and_assert_annot_objects(AnnotSetup::new().with_action(action_dict), &expected);

    // 2. Single filespec key variants
    let single_key_cases: &[(&str, &str)] = &[
        ("UF", "uf-only.pdf"),
        ("F", "f-only.pdf"),
        ("Unix", "/root/path/doc.pdf"),
        ("DOS", "C:\\docs\\report.pdf"),
        ("Mac", "Macintosh HD:docs:file.pdf"),
    ];
    for (key, path) in single_key_cases {
        let mut tester = LinkAnnotTester::new(1);
        let action_dict = gotor_action_with_filespec_keys(&mut tester, &[(key, path)]);
        let expected = gotor_path(path);
        tester
            .build_and_assert_annot_objects(AnnotSetup::new().with_action(action_dict), &expected);
    }

    // 3. Priority tests: first key in list wins
    let priority_cases: &[(&[(&str, &str)], &str)] = &[
        // UF > F > Unix > DOS > Mac
        (
            &[
                ("UF", "uf-wins.pdf"),
                ("F", "f-loses.pdf"),
                ("Unix", "unix-loses.pdf"),
                ("DOS", "dos-loses.pdf"),
                ("Mac", "mac-loses.pdf"),
            ],
            "uf-wins.pdf",
        ),
        // F > Unix > DOS > Mac (no UF)
        (
            &[
                ("F", "f-wins.pdf"),
                ("Unix", "unix-loses.pdf"),
                ("DOS", "dos-loses.pdf"),
            ],
            "f-wins.pdf",
        ),
        // Unix > DOS > Mac (no UF/F)
        (
            &[
                ("Unix", "unix-wins.pdf"),
                ("DOS", "dos-loses.pdf"),
                ("Mac", "mac-loses.pdf"),
            ],
            "unix-wins.pdf",
        ),
        // DOS > Mac (no UF/F/Unix)
        (
            &[("DOS", "C:\\dos-wins.pdf"), ("Mac", "mac-loses.pdf")],
            "C:\\dos-wins.pdf",
        ),
    ];
    for (keys, expected_path) in priority_cases {
        let mut tester = LinkAnnotTester::new(1);
        let action_dict = gotor_action_with_filespec_keys(&mut tester, keys);
        let expected = gotor_path(expected_path);
        tester
            .build_and_assert_annot_objects(AnnotSetup::new().with_action(action_dict), &expected);
    }

    // 4. FS=URL with /F entry returns FileSpec::Url
    let mut tester = LinkAnnotTester::new(1);
    let action_dict = tester.encode_action(&LinkAction::Action(PdfAction::GoToR {
        file: FileSpec::Url("https://example.com/remote.pdf".into()),
        dest: PdfDestination::Named("chapter1".into()),
    }));
    let expected = LinkAction::Action(PdfAction::GoToR {
        file: FileSpec::Url("https://example.com/remote.pdf".into()),
        dest: PdfDestination::Named("chapter1".into()),
    });
    tester.build_and_assert_annot_objects(AnnotSetup::new().with_action(action_dict), &expected);

    // 4. Empty dict (no recognized keys) returns error
    let mut tester = LinkAnnotTester::new(1);
    let action_dict = gotor_action_with_filespec_keys(&mut tester, &[]);
    let annot = tester.build_annotation_object(AnnotSetup::new().with_action(action_dict));
    let result = parse_link_action_from_annot_dict(&annot, &tester.doc, None);
    assert!(result.is_err(), "Empty filespec should return an error");

    // 5. Launch action with each filespec key
    for (key, path) in [
        ("UF", "launched-uf.txt"),
        ("F", "launched-f.txt"),
        ("Unix", "/usr/local/launched.txt"),
        ("DOS", "C:\\launched.txt"),
        ("Mac", "HD:launched.txt"),
    ] {
        let mut tester = LinkAnnotTester::new(1);
        let action_dict = tester.encode_action(&LinkAction::Action(PdfAction::Launch(
            FileSpec::Path("placeholder.txt".into()),
        )));
        let mut fspec = action_dict.get_dict("F").unwrap().unwrap();
        fspec.dict_delete("F").unwrap();
        fspec.dict_delete("UF").unwrap();
        let path_obj = PdfObject::new_string(path).unwrap();
        fspec.dict_put(key, path_obj).unwrap();

        let expected = LinkAction::Action(PdfAction::Launch(FileSpec::Path(path.into())));
        let annot = tester.build_annotation_object(AnnotSetup::new().with_action(action_dict));
        let parsed = tester.parse_annotation_object(&annot);
        assert_eq!(parsed, Some(expected), "Launch with /{key} key");
    }
}

/// Helper: creates a GoToR action dict with specific filespec keys, removing defaults.
fn gotor_action_with_filespec_keys(
    tester: &mut LinkAnnotTester,
    keys: &[(&str, &str)],
) -> PdfObject {
    let action_dict = tester.encode_action(&LinkAction::Action(PdfAction::GoToR {
        file: FileSpec::Path("placeholder.pdf".into()),
        dest: PdfDestination::default(),
    }));
    let mut fspec = action_dict.get_dict("F").unwrap().unwrap();
    fspec.dict_delete("F").unwrap();
    fspec.dict_delete("UF").unwrap();
    for (key, value) in keys {
        let path = PdfObject::new_string(value).unwrap();
        fspec.dict_put(*key, path).unwrap();
    }
    action_dict
}

fn gotor_path(path: &str) -> LinkAction {
    LinkAction::Action(PdfAction::GoToR {
        file: FileSpec::Path(path.into()),
        dest: PdfDestination::default(),
    })
}

#[test]
fn test_parse_filespec_url_requires_f_entry_regression() {
    let dest = PdfDestination::Page {
        page: 0,
        kind: DestinationKind::default(),
    };

    // FS=URL marks URL-based filespecs, but URL payload must be in /F.
    // When /F is absent, the parser must fall back to standard filename keys
    // and return FileSpec::Path, not FileSpec::Url.
    for key in ["UF", "Unix", "DOS", "Mac"] {
        let mut tester = LinkAnnotTester::new(1);
        let url = "https://example.com/only-fallback.pdf";
        let action_dict = tester.encode_action(&LinkAction::Action(PdfAction::GoToR {
            file: FileSpec::Url(url.into()),
            dest: dest.clone(),
        }));

        let mut fspec = action_dict.get_dict("F").unwrap().unwrap();
        fspec.dict_delete("F").unwrap();
        let path = PdfObject::new_string(url).unwrap();
        fspec.dict_put(key, path).unwrap();

        let annot = tester.build_annotation_object(AnnotSetup::new().with_action(action_dict));
        let parsed = tester.parse_annotation_object(&annot);

        // Without /F, the parser treats the fallback value as a filename path, not a URL.
        assert_eq!(parsed, Some(gotor_path(url)), "FS=URL with /{key} fallback");
    }
}

#[test]
fn test_decode_destination_kind_edge_cases() {
    let doc = PdfDocument::new();

    let make_array = |name, values: &[Option<f32>]| -> Result<PdfObject, Error> {
        let mut array = doc.new_array_with_capacity(6)?;
        // Index 0 is the page reference
        array.array_push(PdfObject::new_int(0)?)?;
        array.array_push(PdfObject::new_name(name)?)?;
        for val in values {
            match *val {
                Some(val) => array.array_push(PdfObject::new_real(val)?)?,
                None => array.array_push(PdfObject::new_null())?,
            }
        }
        Ok(array)
    };

    // Unknown dest type defaults to XYZ (match MuPDF)
    let arr = make_array("UnknownType", &[Some(10.0), Some(20.0), Some(1.5)]).unwrap();
    assert_eq!(
        DestinationKind::decode_from(&arr).unwrap(),
        xyz_dest(Some(10.0), Some(20.0), Some(150.0)) // 1.5 == 150%
    );

    // Zero zoom maps to 100%
    let arr = make_array("XYZ", &[None, None, Some(0.0)]).unwrap();
    assert_eq!(
        DestinationKind::decode_from(&arr).unwrap(),
        xyz_dest(None, None, Some(100.0)) // 0.0 -> 100%
    );

    // Negative zoom maps to 100%
    let arr = make_array("XYZ", &[None, None, Some(-1.0)]).unwrap();
    assert_eq!(
        DestinationKind::decode_from(&arr).unwrap(),
        xyz_dest(None, None, Some(100.0)) // z <= 0.0 -> 100%
    );

    // All-null XYZ coordinates -> default
    let arr = make_array("XYZ", &[None, None, None]).unwrap();
    assert_eq!(
        DestinationKind::decode_from(&arr).unwrap(),
        DestinationKind::default()
    );
}

#[test]
fn test_cached_resolver_reuses_cached_entries() {
    use std::cell::Cell;
    use std::collections::HashMap;

    let mut doc = PdfDocument::new();
    for _ in 0..3 {
        doc.new_page(PAGE_SIZE).unwrap();
    }

    let call_count = Cell::new(0u32);
    let mut cache: HashMap<u32, (PdfObject, Option<Matrix>)> = HashMap::new();

    let mut resolver = CachedResolver::new(&mut cache, |page_obj: &PdfObject| {
        call_count.set(call_count.get() + 1);
        Ok(page_obj.page_ctm()?.invert())
    });

    // First resolve for page 1.
    let _ = resolver.resolve(&doc, 1).unwrap();
    assert_eq!(call_count.get(), 1);

    // Second resolve for page 1 — should use cache.
    let _ = resolver.resolve(&doc, 1).unwrap();
    assert_eq!(call_count.get(), 1, "Should reuse cached entry");

    // Resolve for page 2 — should call fn again.
    let _ = resolver.resolve(&doc, 2).unwrap();
    assert_eq!(call_count.get(), 2);

    assert_eq!(cache.len(), 2,);
}

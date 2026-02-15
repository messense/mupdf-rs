use super::{FileSpec, PdfAction, PdfDestination};
use crate::DestinationKind;

#[test]
fn test_pdf_action_format() {
    let goto = |page, kind| PdfAction::GoTo(PdfDestination::Page { page, kind });
    let goto_named = |name: &str| PdfAction::GoTo(PdfDestination::Named(name.into()));
    let gotor_path = |path: &str, dest| PdfAction::GoToR {
        file: FileSpec::Path(path.into()),
        dest,
    };
    let gotor_url = |url: &str, dest| PdfAction::GoToR {
        file: FileSpec::Url(url.into()),
        dest,
    };

    let cases = vec![
        (goto(0, DestinationKind::default()), "#page=1"),
        (goto(4, DestinationKind::default()), "#page=5"),
        (goto(0, DestinationKind::Fit), "#page=1&view=Fit"),
        (goto(0, DestinationKind::FitB), "#page=1&view=FitB"),
        (
            goto(0, DestinationKind::FitH { top: Some(500.0) }),
            "#page=1&view=FitH,500",
        ),
        (
            goto(0, DestinationKind::FitH { top: None }),
            "#page=1&view=FitH",
        ),
        (
            goto(0, DestinationKind::FitV { left: Some(100.0) }),
            "#page=1&view=FitV,100",
        ),
        (
            goto(0, DestinationKind::FitV { left: None }),
            "#page=1&view=FitV",
        ),
        (
            goto(4, DestinationKind::FitBH { top: Some(200.0) }),
            "#page=5&view=FitBH,200",
        ),
        (
            goto(0, DestinationKind::FitBH { top: None }),
            "#page=1&view=FitBH",
        ),
        (
            goto(0, DestinationKind::FitBV { left: Some(50.0) }),
            "#page=1&view=FitBV,50",
        ),
        (
            goto(0, DestinationKind::FitBV { left: None }),
            "#page=1&view=FitBV",
        ),
        (
            goto(
                0,
                DestinationKind::XYZ {
                    left: Some(100.0),
                    top: Some(600.0),
                    zoom: Some(150.0),
                },
            ),
            "#page=1&zoom=150,100,600",
        ),
        (
            goto(
                0,
                DestinationKind::XYZ {
                    left: None,
                    top: None,
                    zoom: Some(200.0),
                },
            ),
            "#page=1&zoom=200,nan,nan",
        ),
        (
            goto(
                0,
                DestinationKind::XYZ {
                    left: Some(100.0),
                    top: None,
                    zoom: None,
                },
            ),
            "#page=1&zoom=nan,100,nan",
        ),
        (
            goto(
                0,
                DestinationKind::XYZ {
                    left: None,
                    top: Some(250.0),
                    zoom: None,
                },
            ),
            "#page=1&zoom=nan,nan,250",
        ),
        (
            goto(
                2,
                DestinationKind::XYZ {
                    left: Some(10.0),
                    top: Some(20.0),
                    zoom: None,
                },
            ),
            "#page=3&zoom=nan,10,20",
        ),
        (
            goto(
                2,
                DestinationKind::XYZ {
                    left: Some(100.0),
                    top: None,
                    zoom: Some(150.0),
                },
            ),
            "#page=3&zoom=150,100,nan",
        ),
        (
            goto(
                0,
                DestinationKind::XYZ {
                    left: None,
                    top: Some(500.0),
                    zoom: Some(75.0),
                },
            ),
            "#page=1&zoom=75,nan,500",
        ),
        (
            goto(
                0,
                DestinationKind::XYZ {
                    left: None,
                    top: None,
                    zoom: Some(0.0),
                },
            ),
            "#page=1",
        ),
        (
            goto(
                0,
                DestinationKind::XYZ {
                    left: Some(50.0),
                    top: None,
                    zoom: Some(0.0),
                },
            ),
            "#page=1&zoom=nan,50,nan",
        ),
        (
            goto(
                0,
                DestinationKind::XYZ {
                    left: Some(f32::NAN),
                    top: Some(f32::NAN),
                    zoom: Some(f32::NAN),
                },
            ),
            "#page=1",
        ),
        (
            goto(
                0,
                DestinationKind::XYZ {
                    left: Some(0.0),
                    top: Some(0.0),
                    zoom: Some(50.0),
                },
            ),
            "#page=1&zoom=50,0,0",
        ),
        (
            goto(
                1,
                DestinationKind::FitR {
                    left: 50.0,
                    bottom: 100.0,
                    right: 200.0,
                    top: 300.0,
                },
            ),
            "#page=2&viewrect=50,100,150,200",
        ),
        (goto_named("Chapter1"), "#nameddest=Chapter1"),
        (goto_named("section_1.1.b"), "#nameddest=section_1.1.b"),
        (goto_named("章节"), "#nameddest=%E7%AB%A0%E8%8A%82"),
        (
            goto_named("Кириллица"),
            "#nameddest=%D0%9A%D0%B8%D1%80%D0%B8%D0%BB%D0%BB%D0%B8%D1%86%D0%B0",
        ),
        (goto_named("😁"), "#nameddest=%F0%9F%98%81"),
        (
            goto_named("Name With Spaces"),
            "#nameddest=Name%20With%20Spaces",
        ),
        (
            goto_named("Name/With/Slashes"),
            "#nameddest=Name%2FWith%2FSlashes",
        ),
        (goto_named("page=10"), "#nameddest=page%3D10"),
        (
            goto_named("a-b_c.d!e~f*g'h(i)j"),
            "#nameddest=a-b_c.d!e~f*g'h(i)j",
        ),
        (goto_named(""), "#nameddest="),
        (
            PdfAction::Uri("https://example.com".into()),
            "https://example.com",
        ),
        (
            PdfAction::Uri("http://example.com/page".into()),
            "http://example.com/page",
        ),
        (
            PdfAction::Uri("mailto:user@example.com".into()),
            "mailto:user@example.com",
        ),
        (
            PdfAction::Uri("ftp://ftp.example.com/file.txt".into()),
            "ftp://ftp.example.com/file.txt",
        ),
        (
            PdfAction::Uri("custom://resource/path".into()),
            "custom://resource/path",
        ),
        (
            PdfAction::Uri("https://example.com/hello%20world".into()),
            "https://example.com/hello%20world",
        ),
        (PdfAction::Uri(String::new()), ""),
        (
            PdfAction::Launch(FileSpec::Path("docs/readme.txt".into())),
            "file:docs/readme.txt#page=1",
        ),
        (
            PdfAction::Launch(FileSpec::Path("/path/to/file.pdf".into())),
            "file:///path/to/file.pdf#page=1",
        ),
        (
            PdfAction::Launch(FileSpec::Path("/path with spaces.pdf".into())),
            "file:///path%20with%20spaces.pdf#page=1",
        ),
        (
            PdfAction::Launch(FileSpec::Path("文件.docx".into())),
            "file:%E6%96%87%E4%BB%B6.docx#page=1",
        ),
        (
            PdfAction::Launch(FileSpec::Path("../report.pdf".into())),
            "file:../report.pdf#page=1",
        ),
        (
            PdfAction::Launch(FileSpec::Path(String::new())),
            "file:#page=1",
        ),
        (
            PdfAction::Launch(FileSpec::Url("https://example.com/doc.pdf".into())),
            "https://example.com/doc.pdf#page=1",
        ),
        (
            gotor_path("/path with spaces.pdf", PdfDestination::default()),
            "file:///path%20with%20spaces.pdf#page=1",
        ),
        (
            gotor_path(
                "other.pdf",
                PdfDestination::Page {
                    page: 0,
                    kind: DestinationKind::Fit,
                },
            ),
            "file:other.pdf#page=1&view=Fit",
        ),
        (
            gotor_path(
                "doc.pdf",
                PdfDestination::Page {
                    page: 2,
                    kind: DestinationKind::XYZ {
                        left: Some(100.0),
                        top: Some(200.0),
                        zoom: Some(150.0),
                    },
                },
            ),
            "file:doc.pdf#page=3&zoom=150,100,200",
        ),
        (
            gotor_url(
                "https://example.com/doc.pdf",
                PdfDestination::Page {
                    page: 1,
                    kind: DestinationKind::Fit,
                },
            ),
            "https://example.com/doc.pdf#page=2&view=Fit",
        ),
        (
            gotor_url(
                "https://example.com/doc.pdf#frag",
                PdfDestination::Page {
                    page: 1,
                    kind: DestinationKind::Fit,
                },
            ),
            "https://example.com/doc.pdf#frag&page=2&view=Fit",
        ),
        (
            gotor_path("other.pdf", PdfDestination::Named("Chapter1".into())),
            "file:other.pdf#nameddest=Chapter1",
        ),
        (
            gotor_url(
                "https://example.com/doc.pdf",
                PdfDestination::Named("Chapter1".into()),
            ),
            "https://example.com/doc.pdf#nameddest=Chapter1",
        ),
        (
            gotor_url(
                "https://example.com/doc.pdf#frag",
                PdfDestination::Named("Chapter1".into()),
            ),
            "https://example.com/doc.pdf#frag&nameddest=Chapter1",
        ),
        (
            gotor_path("doc.pdf", PdfDestination::Named("章节".into())),
            "file:doc.pdf#nameddest=%E7%AB%A0%E8%8A%82",
        ),
    ];

    for (i, (action, expected)) in cases.iter().enumerate() {
        let actual = action.to_string();
        assert_eq!(
            &actual, expected,
            "Case {i} failed:\n  action:   {action:?}\n  expected: {expected}\n  actual:   {actual}"
        );
        assert_eq!(
            action.to_uri(),
            actual,
            "Case {i}: to_uri() differs from to_string()"
        );
    }
}

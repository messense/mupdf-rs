use super::tests_build::{fit_r_dest, xyz_dest};
use super::{extraction::*, *};
use crate::DestinationKind;

#[test]
fn test_url_unescape() {
    // Simple ASCII percent-encoding
    assert_eq!(decode_uri_component("hello%20world"), "hello world");
    assert_eq!(decode_uri_component("test%2Fpath"), "test/path");
    assert_eq!(decode_uri_component("name%3Dvalue"), "name=value");
    assert_eq!(decode_uri_component("%%25"), "%%");

    // UTF-8 multi-byte sequences (Chinese character "中")
    assert_eq!(decode_uri_component("%E4%B8%AD"), "中");
    // Japanese hiragana "あ"
    assert_eq!(decode_uri_component("%E3%81%82"), "あ");
    // Mixed ASCII and UTF-8
    assert_eq!(decode_uri_component("hello%E4%B8%AD%E6%96%87"), "hello中文");

    // Strings without percent-encoding should pass through unchanged
    assert_eq!(decode_uri_component("hello"), "hello");
    assert_eq!(decode_uri_component("Chapter1"), "Chapter1");
    assert_eq!(decode_uri_component(""), "");

    // Invalid UTF-8 sequence should fall back to original string
    // %FF%FE is not valid UTF-8, so we expect the original back
    assert_eq!(decode_uri_component("%FF%FE"), "%FF%FE");
    assert_eq!(decode_uri_component("100%%"), "100%%");
}

#[test]
fn test_parse_uri_schemes() {
    let cases = [
        "http://example.com/page",
        "https://example.com/secure",
        "mailto:user@example.com",
        "ftp://ftp.example.com/file.txt",
        "tel:+1-555-123-4567",
        "HTTP://EXAMPLE.COM",
    ];

    for uri in cases {
        let expected = Some(PdfAction::Uri(uri.into()));
        assert_eq!(parse_external_link(uri), expected);
    }
}

#[test]
fn test_parse_page_params() {
    let out = parse_external_link("#page=5");
    let expected = PdfAction::GoTo(PdfDestination::Page {
        page: 4,
        kind: xyz_dest(None, None, None),
    });
    assert_eq!(out, Some(expected));

    let out = parse_external_link("#page=0");
    let expected = PdfAction::GoTo(PdfDestination::Page {
        page: 0,
        kind: xyz_dest(None, None, None),
    });
    assert_eq!(out, Some(expected));

    let out = parse_external_link("#page=-3");
    let expected = PdfAction::GoTo(PdfDestination::Page {
        page: 0,
        kind: xyz_dest(None, None, None),
    });
    assert_eq!(out, Some(expected));

    let out = parse_external_link("#page=0&zoom=50");
    let expected = PdfAction::GoTo(PdfDestination::Page {
        page: 0,
        kind: xyz_dest(None, None, Some(50.0)),
    });
    assert_eq!(out, Some(expected));

    let out = parse_external_link("#page=-5&zoom=-20,555");
    let expected = PdfAction::GoTo(PdfDestination::Page {
        page: 0,
        kind: xyz_dest(Some(555.0), None, Some(100.0)),
    });
    assert_eq!(out, Some(expected));

    let out = parse_external_link("#page=3&zoom=0,100,200");
    let expected = PdfAction::GoTo(PdfDestination::Page {
        page: 2,
        kind: xyz_dest(Some(100.0), Some(200.0), Some(100.0)),
    });
    assert_eq!(out, Some(expected));

    let out = parse_external_link("#page=30&zoom=nan,456,nan");
    let expected = PdfAction::GoTo(PdfDestination::Page {
        page: 29,
        kind: xyz_dest(Some(456.0), None, None),
    });
    assert_eq!(out, Some(expected));

    let out = parse_external_link("#page=30&zoom=nan,nan,456");
    let expected = PdfAction::GoTo(PdfDestination::Page {
        page: 29,
        kind: xyz_dest(None, Some(456.0), None),
    });
    assert_eq!(out, Some(expected));

    let out = parse_external_link("#page=30&zoom=nan,nan,nan");
    let expected = PdfAction::GoTo(PdfDestination::Page {
        page: 29,
        kind: xyz_dest(None, None, None),
    });
    assert_eq!(out, Some(expected));

    let out = parse_external_link("#page=1&zoom=100,-50,-100");
    let expected = PdfAction::GoTo(PdfDestination::Page {
        page: 0,
        kind: xyz_dest(Some(-50.0), Some(-100.0), Some(100.0)),
    });
    assert_eq!(out, Some(expected));

    let out = parse_external_link("#page=1&view=FitH,-200");
    let expected = PdfAction::GoTo(PdfDestination::Page {
        page: 0,
        kind: DestinationKind::FitH { top: Some(-200.0) },
    });
    assert_eq!(out, Some(expected));

    let out = parse_external_link("#page=1&view=FitV,-300.5");
    let expected = PdfAction::GoTo(PdfDestination::Page {
        page: 0,
        kind: DestinationKind::FitV { left: Some(-300.5) },
    });
    assert_eq!(out, Some(expected));

    let out = parse_external_link("#page=1&zoom=0");
    let expected = PdfAction::GoTo(PdfDestination::Page {
        page: 0,
        kind: xyz_dest(None, None, Some(100.0)),
    });
    assert_eq!(out, Some(expected));

    let out = parse_external_link("#page=1&zoom=-50");
    let expected = PdfAction::GoTo(PdfDestination::Page {
        page: 0,
        kind: xyz_dest(None, None, Some(100.0)),
    });
    assert_eq!(out, Some(expected));

    let out = parse_external_link("#page=1&zoom=inf");
    let expected = PdfAction::GoTo(PdfDestination::Page {
        page: 0,
        kind: xyz_dest(None, None, Some(100.0)),
    });
    assert_eq!(out, Some(expected));
}

#[test]
fn test_parse_named_dest() {
    let cases = [
        ("#nameddest=Chapter1", "Chapter1"),
        ("#nameddest=%E7%AB%A0%E8%8A%82", "章节"), // UTF-8 encoded
        ("#nameddest=Chapter1&foo=bar", "Chapter1"),
        ("#Introduction", "Introduction"),
        ("#%E7%AB%A0%E8%8A%82", "章节"), // UTF-8 encoded
        ("#page=2&comment=keep-me", "page=2&comment=keep-me"),
    ];

    for (input, expected_name) in cases {
        let expected = Some(PdfAction::GoTo(PdfDestination::Named(expected_name.into())));
        assert_eq!(parse_external_link(input), expected);
    }
}

#[test]
fn test_parse_remote_file_scheme() {
    let out = parse_external_link("file:///path/to/document.pdf");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/path/to/document.pdf".to_string()),
        dest: PdfDestination::default(),
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("/path/to/document.pdf");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/path/to/document.pdf".to_string()),
        dest: PdfDestination::default(),
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("document.pdf");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("document.pdf".to_string()),
        dest: PdfDestination::default(),
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:///path/to/document2.pdf#page=5");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/path/to/document2.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 4,
            kind: xyz_dest(None, None, None),
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:///path/doc.pdf#page=0");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/path/doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: xyz_dest(None, None, None),
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:///path/doc2.pdf#page=-55");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/path/doc2.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: xyz_dest(None, None, None),
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:///path/doc.pdf#nameddest=Chapter3");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/path/doc.pdf".to_string()),
        dest: PdfDestination::Named("Chapter3".to_string()),
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:///path/doc22.pdf#Part555");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/path/doc22.pdf".to_string()),
        dest: PdfDestination::Named("Part555".to_string()),
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:///doc.pdf#page=10&view=Fit");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 9,
            kind: DestinationKind::Fit,
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:///doc.pdf#page=-5&view=FitH,-100");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: DestinationKind::FitH { top: Some(-100.0) },
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:///doc.pdf#page=-5&view=FitH");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: DestinationKind::FitH { top: None },
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:///doc.pdf#page=2&view=FitV,50.5");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 1,
            kind: DestinationKind::FitV { left: Some(50.5) },
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:///doc.pdf#page=2&view=FitV");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 1,
            kind: DestinationKind::FitV { left: None },
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:///doc.pdf#page=1&view=FitB");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: DestinationKind::FitB,
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:///doc.pdf#page=1&view=FitBH,200");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: DestinationKind::FitBH { top: Some(200.0) },
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:///doc.pdf#page=1&view=FitBH");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: DestinationKind::FitBH { top: None },
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:///doc.pdf#page=1&view=FitBV,150");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: DestinationKind::FitBV { left: Some(150.0) },
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:///doc.pdf#page=1&view=FitBV");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: DestinationKind::FitBV { left: None },
        },
    };
    assert_eq!(out, Some(expected));

    // Spec: viewrect=left,top,wd,ht (in PDF space)
    let out = parse_external_link("file:///doc.pdf#page=1&viewrect=15,25,500,600");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: fit_r_dest(15.0, 25.0, 515.0, 625.0),
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:///doc.pdf#page=1&viewrect=10,20,100");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/doc.pdf".to_string()),
        dest: PdfDestination::default(),
    };
    assert_eq!(out, Some(expected));

    // Spec: zoom=scale,left,top.
    let out = parse_external_link("file:///doc.pdf#page=1&zoom=150");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: xyz_dest(None, None, Some(150.0)),
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:///doc.pdf#page=3&zoom=200,10,20");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 2,
            kind: xyz_dest(Some(10.0), Some(20.0), Some(200.0)),
        },
    };
    assert_eq!(out, Some(expected));

    // Spec: "it is possible that later actions will override the effects of previous actions".
    let out = parse_external_link("file:///doc.pdf#page=5&view=FitH,33#page=10");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 9,
            kind: DestinationKind::default(),
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:////server/share/doc.pdf");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/server/share/doc.pdf".to_string()),
        dest: PdfDestination::default(),
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:////server/share/doc.pdf#page=3&view=FitV,120");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/server/share/doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 2,
            kind: DestinationKind::FitV { left: Some(120.0) },
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("//server/share/document.pdf");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/server/share/document.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: DestinationKind::default(),
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:///a/b/../c.pdf");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/a/c.pdf".to_string()),
        dest: PdfDestination::default(),
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:a/b/../../c.pdf");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("c.pdf".into()),
        dest: PdfDestination::default(),
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:///a/b/../../c.pdf");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/c.pdf".into()),
        dest: PdfDestination::default(),
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("my%20document.pdf");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("my document.pdf".into()),
        dest: PdfDestination::default(),
    };
    assert_eq!(out, Some(expected));
}

#[test]
fn test_parse_remote_url_scheme() {
    let out = parse_external_link("http://example.org/doc.pdf#Chapter6");
    let expected = PdfAction::GoToR {
        file: FileSpec::Url("http://example.org/doc.pdf".to_owned()),
        dest: PdfDestination::Named("Chapter6".to_owned()),
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("http://example.org/doc.pdf#page=3");
    let expected = PdfAction::GoToR {
        file: FileSpec::Url("http://example.org/doc.pdf".to_owned()),
        dest: PdfDestination::Page {
            page: 2,
            kind: xyz_dest(None, None, None),
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("http://example.org/doc.pdf#page=3&zoom=200,250,100");
    let expected = PdfAction::GoToR {
        file: FileSpec::Url("http://example.org/doc.pdf".to_owned()),
        dest: PdfDestination::Page {
            page: 2,
            kind: xyz_dest(Some(250.0), Some(100.0), Some(200.0)),
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("http://example.org/doc.pdf#zoom=50");
    let expected = PdfAction::GoToR {
        file: FileSpec::Url("http://example.org/doc.pdf".to_owned()),
        dest: PdfDestination::Page {
            page: 0,
            kind: xyz_dest(None, None, Some(50.0)),
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("http://example.org/doc.pdf#page=72&view=fitH,100");
    let expected = PdfAction::GoToR {
        file: FileSpec::Url("http://example.org/doc.pdf".to_owned()),
        dest: PdfDestination::Page {
            page: 71,
            kind: DestinationKind::FitH { top: Some(100.0) },
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("http://example.org/doc.pdf#pagemode=none");
    let expected = PdfAction::Uri("http://example.org/doc.pdf#pagemode=none".to_owned());
    assert_eq!(out, Some(expected));

    let out = parse_external_link("http://example.org/doc.pdf#pagemode=bookmarks&page=2");
    let expected =
        PdfAction::Uri("http://example.org/doc.pdf#pagemode=bookmarks&page=2".to_owned());
    assert_eq!(out, Some(expected));

    let out = parse_external_link("http://example.org/doc.pdf#page=3&pagemode=thumbs");
    let expected = PdfAction::Uri("http://example.org/doc.pdf#page=3&pagemode=thumbs".to_owned());
    assert_eq!(out, Some(expected));

    let out = parse_external_link(
        "http://example.org/doc.pdf#collab=DAVFDF@http://review_server/Collab/user1",
    );
    let expected = PdfAction::Uri(
        "http://example.org/doc.pdf#collab=DAVFDF@http://review_server/Collab/user1".to_owned(),
    );
    assert_eq!(out, Some(expected));

    let out = parse_external_link(
        "http://example.org/doc.pdf#page=1&comment=452fde0e-fd22-457c-84aa-2cf5bed5a349",
    );
    let expected = PdfAction::Uri(
        "http://example.org/doc.pdf#page=1&comment=452fde0e-fd22-457c-84aa-2cf5bed5a349".to_owned(),
    );
    assert_eq!(out, Some(expected));

    let out = parse_external_link("http://example.org/doc.pdf#fdf=http://example.org/doc.fdf");
    let expected =
        PdfAction::Uri("http://example.org/doc.pdf#fdf=http://example.org/doc.fdf".to_owned());
    assert_eq!(out, Some(expected));
}

#[test]
fn test_parse_relative_pdf_path() {
    let out = parse_external_link("manual.pdf#Chapter6");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("manual.pdf".to_string()),
        dest: PdfDestination::Named("Chapter6".to_string()),
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("../other/document.pdf");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("../other/document.pdf".to_string()),
        dest: PdfDestination::default(),
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("/another/document.pdf");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/another/document.pdf".to_string()),
        dest: PdfDestination::default(),
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("other.pdf#page=10");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("other.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 9,
            kind: DestinationKind::default(),
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("manual.pdf#nameddest=Chapter5");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("manual.pdf".to_string()),
        dest: PdfDestination::Named("Chapter5".to_string()),
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("document.pdf#page=1");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("document.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: xyz_dest(None, None, None),
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("document.pdf#page=0");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("document.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: xyz_dest(None, None, None),
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("document.pdf#page=-5");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("document.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: xyz_dest(None, None, None),
        },
    };
    assert_eq!(out, Some(expected));
}

#[test]
fn test_parse_non_pdf() {
    let out = parse_external_link("readme.txt");
    let expected = PdfAction::Launch(FileSpec::Path("readme.txt".to_string()));
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:///path/to/document.doc");
    let expected = PdfAction::Launch(FileSpec::Path("/path/to/document.doc".to_string()));
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:///path/to/document.doc#page=0");
    let expected = PdfAction::Launch(FileSpec::Path("/path/to/document.doc".to_string()));
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:other/path/document.txt");
    let expected = PdfAction::Launch(FileSpec::Path("other/path/document.txt".to_string()));
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:another/path/document.docx#page=10");
    let expected = PdfAction::Launch(FileSpec::Path("another/path/document.docx".to_string()));
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:////server/share/doc.doc#page=1&view=Fit");
    let expected = PdfAction::Launch(FileSpec::Path("/server/share/doc.doc".to_string()));
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:.docx");
    let expected = PdfAction::Launch(FileSpec::Path(".docx".to_string()));
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:..docx");
    let expected = PdfAction::Launch(FileSpec::Path("..docx".to_string()));
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:../report.docx");
    let expected = PdfAction::Launch(FileSpec::Path("../report.docx".to_string()));
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:a/...docx");
    let expected = PdfAction::Launch(FileSpec::Path("a/...docx".to_string()));
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:a/../b.docx");
    let expected = PdfAction::Launch(FileSpec::Path("b.docx".to_string()));
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:a/../../b.docx");
    let expected = PdfAction::Launch(FileSpec::Path("../b.docx".to_string()));
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:/a//b/./c.docx");
    let expected = PdfAction::Launch(FileSpec::Path("/a/b/c.docx".to_string()));
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:/../a.docx");
    let expected = PdfAction::Launch(FileSpec::Path("/a.docx".to_string()));
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:/a/../../b.docx");
    let expected = PdfAction::Launch(FileSpec::Path("/b.docx".to_string()));
    assert_eq!(out, Some(expected));

    let out = parse_external_link("ab:some/path");
    let expected = PdfAction::Launch(FileSpec::Path("ab:some/path".to_string()));
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:a/./b/../c.txt");
    let expected = PdfAction::Launch(FileSpec::Path("a/c.txt".into()));
    assert_eq!(out, Some(expected));

    let out = parse_external_link("my%20file.txt");
    let expected = PdfAction::Launch(FileSpec::Path("my file.txt".into()));
    assert_eq!(out, Some(expected));
}

#[test]
fn test_parse_edge_cases() {
    let expected = FileSpec::Path("path with spaces.pdf".into());
    assert!(matches!(
        parse_external_link("path%20with%20spaces.pdf"),
        Some(PdfAction::GoToR { file, .. }) if file == expected
    ));

    let uri_cases = [
        "custom://resource/path",
        "http://example.com/a%2Fb",
        "cmd://goto-page/12",
        "abc:some/path",
    ];

    for uri in uri_cases {
        let expected = Some(PdfAction::Uri(uri.into()));
        assert_eq!(parse_external_link(uri), expected, "Failed on URI: {}", uri);
    }
}

// ==========================================================================
// is_external_link tests
// ==========================================================================

#[test]
fn test_is_external_link() {
    assert!(!is_external_link("/path/to/file.pdf"));
    assert!(!is_external_link("/usr/local/bin"));

    assert!(!is_external_link("\\\\server\\share\\file.pdf"));
    assert!(!is_external_link("//server/share/file.pdf"));

    // Windows drive letters should be detected
    assert!(!is_external_link("C:/docs/a.pdf"));
    assert!(!is_external_link("C:\\docs\\a.pdf"));
    assert!(!is_external_link("D:/file.txt"));
    assert!(!is_external_link("C:/path/to/file"));
    assert!(!is_external_link("D:\\path\\to\\file"));
    assert!(!is_external_link("x:something"));

    assert!(!is_external_link("./file.pdf"));
    assert!(!is_external_link("../other/file.pdf"));
    assert!(!is_external_link(".\\file.pdf"));
    assert!(!is_external_link("..\\other\\file.pdf"));
    assert!(!is_external_link("ab:some/path"));
    // Scheme must start with alpha.
    assert!(!is_external_link("123:path"));
    assert!(!is_external_link("-ab:path"));
    // No colon at all -> not external.
    assert!(!is_external_link("just-a-path"));
    assert!(!is_external_link(""));

    // These should NOT be detected as local paths (they're URIs)
    assert!(is_external_link("http://example.com"));
    assert!(is_external_link("mailto:user@example.com"));
    assert!(is_external_link("file:///path/to/file"));
    assert!(is_external_link("file:path/to/file"));
    assert!(is_external_link("abc:some/path"));
    // Scheme can contain digits, +, -, .
    assert!(is_external_link("svn+ssh://server/path"));
    assert!(is_external_link("coap+tcp://device/path"));
    assert!(is_external_link("a.b.c:path"));
}

// ==========================================================================
// Windows path handling tests
// ==========================================================================

#[test]
fn test_parse_windows_path() {
    let paths = [
        "C:/docs/document.pdf",
        "/C:/docs/document.pdf",
        "C:\\docs\\document.pdf",
        "/C:\\docs\\document.pdf",
    ];

    for (idx, path) in paths.into_iter().enumerate() {
        let expected = PdfAction::GoToR {
            file: FileSpec::Path(path.into()),
            dest: PdfDestination::default(),
        };
        assert_eq!(parse_external_link(path), Some(expected), "index: {idx}");
    }
}

// ==========================================================================
// is_valid_pdf_path tests
// ==========================================================================

#[test]
fn test_is_pdf_path() {
    let valid = ["file.pdf", "file.PDF", "file.Pdf", "F.PDF", "f.pdf", ".pdf"];
    let invalid = ["file.txt", "pdf", ".pd"];

    for path in valid {
        assert!(is_pdf_path(path), "Should be valid: {path}");
    }
    for path in invalid {
        assert!(!is_pdf_path(path), "Should be invalid: {path}");
    }
}

// ==========================================================================
// Command case-insensitivity
// ==========================================================================
#[test]
fn test_case_insensitive() {
    // Change 'nameddest' -> 'NaMedDesT'
    let out = parse_external_link("https://path/doc.pdf#NaMedDesT=Chapter3");
    let expected = PdfAction::GoToR {
        file: FileSpec::Url("https://path/doc.pdf".to_string()),
        dest: PdfDestination::Named("Chapter3".to_string()),
    };
    assert_eq!(out, Some(expected));

    // Named destination without key
    let out = parse_external_link("https://path/doc22.PdF#Part555");
    let expected = PdfAction::GoToR {
        file: FileSpec::Url("https://path/doc22.PdF".to_string()),
        dest: PdfDestination::Named("Part555".to_string()),
    };
    assert_eq!(out, Some(expected));

    // 'page' -> 'PaGe', 'view' -> 'ViEw', 'Fit' -> 'fIt'
    let out = parse_external_link("https://doc.pdf#PaGe=10&ViEw=fIt");
    let expected = PdfAction::GoToR {
        file: FileSpec::Url("https://doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 9,
            kind: DestinationKind::Fit,
        },
    };
    assert_eq!(out, Some(expected));

    // 'page' -> 'pAgE', 'view' -> 'vIeW', 'FitH' -> 'FiTh'
    let out = parse_external_link("https://doc.pdf#pAgE=-5&vIeW=FiTh,-100");
    let expected = PdfAction::GoToR {
        file: FileSpec::Url("https://doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: DestinationKind::FitH { top: Some(-100.0) },
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("https://doc.pdf#PAGE=-5&VIEW=fITh");
    let expected = PdfAction::GoToR {
        file: FileSpec::Url("https://doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: DestinationKind::FitH { top: None },
        },
    };
    assert_eq!(out, Some(expected));

    // 'page' -> 'PaGe', 'view' -> 'ViEw', 'FitV' -> 'FiTv'
    let out = parse_external_link("https://doc.pdf#PaGe=2&ViEw=FiTv,50.5");
    let expected = PdfAction::GoToR {
        file: FileSpec::Url("https://doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 1,
            kind: DestinationKind::FitV { left: Some(50.5) },
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("https://doc.pdf#pAge=2&viEW=FITv");
    let expected = PdfAction::GoToR {
        file: FileSpec::Url("https://doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 1,
            kind: DestinationKind::FitV { left: None },
        },
    };
    assert_eq!(out, Some(expected));

    // 'FitB' -> 'fItB'
    let out = parse_external_link("https://doc.pdf#PAGE=1&VIEW=fItB");
    let expected = PdfAction::GoToR {
        file: FileSpec::Url("https://doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: DestinationKind::FitB,
        },
    };
    assert_eq!(out, Some(expected));

    // 'FitBH' -> 'fiTbh'
    let out = parse_external_link("https://doc.pdf#PaGe=1&ViEw=fiTbh,200");
    let expected = PdfAction::GoToR {
        file: FileSpec::Url("https://doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: DestinationKind::FitBH { top: Some(200.0) },
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("https://doc.pdf#page=1&view=FITBH");
    let expected = PdfAction::GoToR {
        file: FileSpec::Url("https://doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: DestinationKind::FitBH { top: None },
        },
    };
    assert_eq!(out, Some(expected));

    // 'FitBV' -> 'FiTbV'
    let out = parse_external_link("https://doc.pdf#PaGe=1&ViEw=FiTbV,150");
    let expected = PdfAction::GoToR {
        file: FileSpec::Url("https://doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: DestinationKind::FitBV { left: Some(150.0) },
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("https://doc.pdf#page=1&view=FITBV");
    let expected = PdfAction::GoToR {
        file: FileSpec::Url("https://doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: DestinationKind::FitBV { left: None },
        },
    };
    assert_eq!(out, Some(expected));

    // 'viewrect' -> 'VIEWRECT' (in PDF space)
    let out = parse_external_link("https://doc.pdf#PaGe=1&ViEwReCt=10,20,100,200");
    let expected = PdfAction::GoToR {
        file: FileSpec::Url("https://doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: fit_r_dest(10.0, 20.0, 110.0, 220.0),
        },
    };
    assert_eq!(out, Some(expected));

    // 'VIEWRECT' (Caps)
    let out = parse_external_link("https://doc.pdf#pAGe=1&VIEWRECT=10,20,100");
    let expected = PdfAction::GoToR {
        file: FileSpec::Url("https://doc.pdf".to_string()),
        dest: PdfDestination::default(),
    };
    assert_eq!(out, Some(expected));

    // Spec: zoom=scale,left,top. -> 'ZooM'
    let out = parse_external_link("https://doc.pdf#page=1&ZooM=150");
    let expected = PdfAction::GoToR {
        file: FileSpec::Url("https://doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: xyz_dest(None, None, Some(150.0)),
        },
    };
    assert_eq!(out, Some(expected));

    // 'zOoM'
    let out = parse_external_link("https://doc.pdf#Page=3&zOoM=200,10,20");
    let expected = PdfAction::GoToR {
        file: FileSpec::Url("https://doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 2,
            kind: xyz_dest(Some(10.0), Some(20.0), Some(200.0)),
        },
    };
    assert_eq!(out, Some(expected));
}

#[test]
fn test_order_zoom_before_page_page_overrides_zoom() {
    // Per spec note: later actions may override earlier actions; recommended order is page then zoom.
    // Here we encode the expected "page resets view" behavior:
    let out = parse_external_link("file:///doc.pdf#zoom=200&page=3");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 2,
            kind: DestinationKind::default(), // zoom overridden by later page action
        },
    };
    assert_eq!(out, Some(expected));
}

#[test]
fn test_other_specification_params2() {
    let cases = [
        "file:///doc.pdf#page=1&comment=452fde0e-fd22-2cf5bed5a349&view=FitH,255",
        "file:///doc.pdf#pagemode=bookmarks#search=\"word1 word2\"#page=35",
        "file:///doc.pdf#pagemode=thumbs#toolbar=1#statusbar=1#messages=1#page=895",
        "file:///doc.pdf#pagemode=none#navpanes=1#highlight=10,55,33,45#page=48",
        "file:///doc.pdf#collab=DAVFDF@http://review/Collab/user1##fdf=http://example.org/doc.fdf#page=2&view=FitV,56",
    ];

    for (idx, input) in cases.into_iter().enumerate() {
        let expected = PdfAction::Uri(input.into());
        assert_eq!(parse_external_link(input), Some(expected), "index: {idx}");
    }
}

#[test]
fn test_parse_invalid_uri() {
    assert_eq!(parse_external_link(""), None);
    assert_eq!(parse_external_link("   "), None);
    assert_eq!(parse_external_link("\t"), None);
    assert_eq!(parse_external_link(" \n "), None);
    assert_eq!(parse_external_link("#"), None);
    assert_eq!(parse_external_link("#nameddest="), None);
    assert_eq!(parse_external_link("file:"), None);
}

#[test]
fn test_parse_very_large_page_number() {
    // Very large page number overflows i32::parse() -> None -> falls back to Named(params)
    // Handling this as a special case is redundant because a non-existent Named destination
    // would simply be ignored anyway.
    let out = parse_external_link("#page=999999999999");
    let expected = PdfAction::GoTo(PdfDestination::Named("page=999999999999".to_owned()));
    assert_eq!(out, Some(expected));
}

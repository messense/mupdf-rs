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
    match parse_external_link("http://example.com/page") {
        Some(PdfAction::Uri(uri)) => assert_eq!(uri, "http://example.com/page"),
        other => panic!("Expected Uri, got {:?}", other),
    }

    match parse_external_link("https://example.com/secure") {
        Some(PdfAction::Uri(uri)) => assert_eq!(uri, "https://example.com/secure"),
        other => panic!("Expected Uri, got {:?}", other),
    }

    match parse_external_link("mailto:user@example.com") {
        Some(PdfAction::Uri(uri)) => assert_eq!(uri, "mailto:user@example.com"),
        other => panic!("Expected Uri, got {:?}", other),
    }

    match parse_external_link("ftp://ftp.example.com/file.txt") {
        Some(PdfAction::Uri(uri)) => assert_eq!(uri, "ftp://ftp.example.com/file.txt"),
        other => panic!("Expected Uri, got {:?}", other),
    }

    match parse_external_link("tel:+1-555-123-4567") {
        Some(PdfAction::Uri(uri)) => assert_eq!(uri, "tel:+1-555-123-4567"),
        other => panic!("Expected Uri, got {:?}", other),
    }

    // RFC 3986: schemes are case-insensitive
    match parse_external_link("HTTP://EXAMPLE.COM") {
        Some(PdfAction::Uri(uri)) => assert_eq!(uri, "HTTP://EXAMPLE.COM"),
        other => panic!("Expected Uri, got {:?}", other),
    }
}

#[test]
fn test_parse_page_params() {
    assert_eq!(
        parse_external_link("#page=5"),
        Some(PdfAction::GoTo(PdfDestination::Page {
            page: 4,
            kind: DestinationKind::XYZ {
                left: None,
                top: None,
                zoom: None,
            }
        }))
    );

    assert_eq!(
        parse_external_link("#page=0"),
        Some(PdfAction::GoTo(PdfDestination::Page {
            page: 0,
            kind: DestinationKind::XYZ {
                left: None,
                top: None,
                zoom: None,
            }
        }))
    );

    assert_eq!(
        parse_external_link("#page=-3"),
        Some(PdfAction::GoTo(PdfDestination::Page {
            page: 0,
            kind: DestinationKind::XYZ {
                left: None,
                top: None,
                zoom: None,
            }
        }))
    );

    assert_eq!(
        parse_external_link("#page=0&zoom=50"),
        Some(PdfAction::GoTo(PdfDestination::Page {
            page: 0,
            kind: DestinationKind::XYZ {
                left: None,
                top: None,
                zoom: Some(50.0),
            }
        }))
    );

    assert_eq!(
        parse_external_link("#page=-5&zoom=-20,555"),
        Some(PdfAction::GoTo(PdfDestination::Page {
            page: 0,
            kind: DestinationKind::XYZ {
                left: Some(555.0),
                top: None,
                zoom: Some(100.0),
            }
        }))
    );

    assert_eq!(
        parse_external_link("#page=3&zoom=0,100,200"),
        Some(PdfAction::GoTo(PdfDestination::Page {
            page: 2,
            kind: DestinationKind::XYZ {
                left: Some(100.0),
                top: Some(200.0),
                zoom: Some(100.0),
            }
        }))
    );

    assert_eq!(
        parse_external_link("#page=30&zoom=nan,456,nan"),
        Some(PdfAction::GoTo(PdfDestination::Page {
            page: 29,
            kind: DestinationKind::XYZ {
                left: Some(456.0),
                top: None,
                zoom: None,
            }
        }))
    );

    assert_eq!(
        parse_external_link("#page=30&zoom=nan,nan,456"),
        Some(PdfAction::GoTo(PdfDestination::Page {
            page: 29,
            kind: DestinationKind::XYZ {
                left: None,
                top: Some(456.0),
                zoom: None,
            }
        }))
    );

    assert_eq!(
        parse_external_link("#page=30&zoom=nan,nan,nan"),
        Some(PdfAction::GoTo(PdfDestination::Page {
            page: 29,
            kind: DestinationKind::XYZ {
                left: None,
                top: None,
                zoom: None,
            }
        }))
    );
}

#[test]
fn test_parse_named_dest() {
    match parse_external_link("#nameddest=Chapter1") {
        Some(PdfAction::GoTo(PdfDestination::Named(name))) => assert_eq!(name, "Chapter1"),
        other => panic!("Expected GoTo(Named), got {:?}", other),
    }

    // UTF-8 encoded named destination
    match parse_external_link("#nameddest=%E7%AB%A0%E8%8A%82") {
        Some(PdfAction::GoTo(PdfDestination::Named(name))) => assert_eq!(name, "章节"),
        other => panic!("Expected GoTo(Named), got {:?}", other),
    }

    match parse_external_link("#Introduction") {
        Some(PdfAction::GoTo(PdfDestination::Named(name))) => assert_eq!(name, "Introduction"),
        other => panic!("Expected GoTo(Named), got {:?}", other),
    }

    // UTF-8 encoded named destination
    match parse_external_link("#%E7%AB%A0%E8%8A%82") {
        Some(PdfAction::GoTo(PdfDestination::Named(name))) => assert_eq!(name, "章节"),
        other => panic!("Expected GoTo(Named), got {:?}", other),
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
            kind: DestinationKind::XYZ {
                left: None,
                top: None,
                zoom: None,
            },
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:///path/doc.pdf#page=0");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/path/doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: DestinationKind::XYZ {
                left: None,
                top: None,
                zoom: None,
            },
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:///path/doc2.pdf#page=-55");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/path/doc2.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: DestinationKind::XYZ {
                left: None,
                top: None,
                zoom: None,
            },
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
            kind: DestinationKind::FitR {
                left: 15.0,
                bottom: 25.0,
                right: 515.0,
                top: 625.0,
            },
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
            kind: DestinationKind::XYZ {
                left: None,
                top: None,
                zoom: Some(150.0),
            },
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("file:///doc.pdf#page=3&zoom=200,10,20");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("/doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 2,
            kind: DestinationKind::XYZ {
                left: Some(10.0),
                top: Some(20.0),
                zoom: Some(200.0),
            },
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
            kind: DestinationKind::XYZ {
                left: None,
                top: None,
                zoom: None,
            },
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("http://example.org/doc.pdf#page=3&zoom=200,250,100");
    let expected = PdfAction::GoToR {
        file: FileSpec::Url("http://example.org/doc.pdf".to_owned()),
        dest: PdfDestination::Page {
            page: 2,
            kind: DestinationKind::XYZ {
                left: Some(250.0),
                top: Some(100.0),
                zoom: Some(200.0),
            },
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("http://example.org/doc.pdf#zoom=50");
    let expected = PdfAction::GoToR {
        file: FileSpec::Url("http://example.org/doc.pdf".to_owned()),
        dest: PdfDestination::Page {
            page: 0,
            kind: DestinationKind::XYZ {
                left: None,
                top: None,
                zoom: Some(50.0),
            },
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
            kind: DestinationKind::XYZ {
                left: None,
                top: None,
                zoom: None,
            },
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("document.pdf#page=0");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("document.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: DestinationKind::XYZ {
                left: None,
                top: None,
                zoom: None,
            },
        },
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("document.pdf#page=-5");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("document.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 0,
            kind: DestinationKind::XYZ {
                left: None,
                top: None,
                zoom: None,
            },
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
}

#[test]
fn test_parse_edge_cases() {
    match parse_external_link("path%20with%20spaces.pdf") {
        Some(PdfAction::GoToR { file, .. }) => {
            assert_eq!(file, FileSpec::Path("path with spaces.pdf".to_string()));
        }
        other => panic!("Expected GoToR, got {:?}", other),
    }

    // Unknown schemes should still be treated as URIs
    match parse_external_link("custom://resource/path") {
        Some(PdfAction::Uri(uri)) => assert_eq!(uri, "custom://resource/path"),
        other => panic!("Expected Uri, got {:?}", other),
    }

    // External URIs should preserve percent-encoding
    match parse_external_link("http://example.com/a%2Fb") {
        Some(PdfAction::Uri(uri)) => assert_eq!(uri, "http://example.com/a%2Fb"),
        other => panic!("Expected Uri, got {:?}", other),
    }

    match parse_external_link("cmd://goto-page/12") {
        Some(PdfAction::Uri(u)) => assert_eq!(u, "cmd://goto-page/12"),
        other => panic!("Expected Uri for cmd:// scheme, got {:?}", other),
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

    assert!(!is_external_link("./file.pdf"));
    assert!(!is_external_link("../other/file.pdf"));
    assert!(!is_external_link(".\\file.pdf"));
    assert!(!is_external_link("..\\other\\file.pdf"));

    // These should NOT be detected as local paths (they're URIs)
    assert!(is_external_link("http://example.com"));
    assert!(is_external_link("mailto:user@example.com"));
    assert!(is_external_link("file:///path/to/file"));
    assert!(is_external_link("file:path/to/file"));
}

// ==========================================================================
// Windows path handling tests
// ==========================================================================

#[test]
fn test_parse_windows_path() {
    let out = parse_external_link("C:/docs/document.pdf");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("C:/docs/document.pdf".to_string()),
        dest: PdfDestination::default(),
    };
    assert_eq!(out, Some(expected));

    let out = parse_external_link("C:\\docs\\document.pdf");
    let expected = PdfAction::GoToR {
        file: FileSpec::Path("C:\\docs\\document.pdf".to_string()),
        dest: PdfDestination::default(),
    };
    assert_eq!(out, Some(expected));
}

// ==========================================================================
// is_valid_pdf_path tests
// ==========================================================================

#[test]
fn test_is_valid_pdf_path() {
    assert!(is_pdf_path("file.pdf"));
    assert!(is_pdf_path("file.PDF"));
    assert!(is_pdf_path("file.Pdf"));
    assert!(is_pdf_path("FILE.PDF"));
    assert!(is_pdf_path("f.pdf"));
    assert!(is_pdf_path(".pdf"));

    assert!(!is_pdf_path("file.txt"));
    assert!(!is_pdf_path("pdf")); // Too short
    assert!(!is_pdf_path(".pd")); // Too short
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
            kind: DestinationKind::FitR {
                left: 10.0,
                bottom: 20.0,
                right: 110.0,
                top: 220.0,
            },
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
            kind: DestinationKind::XYZ {
                left: None,
                top: None,
                zoom: Some(150.0),
            },
        },
    };
    assert_eq!(out, Some(expected));

    // 'zOoM'
    let out = parse_external_link("https://doc.pdf#Page=3&zOoM=200,10,20");
    let expected = PdfAction::GoToR {
        file: FileSpec::Url("https://doc.pdf".to_string()),
        dest: PdfDestination::Page {
            page: 2,
            kind: DestinationKind::XYZ {
                left: Some(10.0),
                top: Some(20.0),
                zoom: Some(200.0),
            },
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
fn test_other_specification_params() {
    let out = parse_external_link(
        "file:///doc.pdf#page=1&comment=452fde0e-fd22-2cf5bed5a349&view=FitH,255",
    );
    let expected = PdfAction::Uri(
        "file:///doc.pdf#page=1&comment=452fde0e-fd22-2cf5bed5a349&view=FitH,255".to_owned(),
    );
    assert_eq!(out, Some(expected));

    let out =
        parse_external_link("file:///doc.pdf#pagemode=bookmarks#search=\"word1 word2\"#page=35");
    let expected = PdfAction::Uri(
        "file:///doc.pdf#pagemode=bookmarks#search=\"word1 word2\"#page=35".to_owned(),
    );
    assert_eq!(out, Some(expected));

    let out = parse_external_link(
        "file:///doc.pdf#pagemode=thumbs#toolbar=1#statusbar=1#messages=1#page=895",
    );
    let expected = PdfAction::Uri(
        "file:///doc.pdf#pagemode=thumbs#toolbar=1#statusbar=1#messages=1#page=895".to_owned(),
    );
    assert_eq!(out, Some(expected));

    let out = parse_external_link(
        "file:///doc.pdf#pagemode=none#navpanes=1#highlight=10,55,33,45#page=48",
    );
    let expected = PdfAction::Uri(
        "file:///doc.pdf#pagemode=none#navpanes=1#highlight=10,55,33,45#page=48".to_owned(),
    );
    assert_eq!(out, Some(expected));

    let out = parse_external_link(
        "file:///doc.pdf#collab=DAVFDF@http://review/Collab/user1##fdf=http://example.org/doc.fdf#page=2&view=FitV,56",
    );
    let expected = PdfAction::Uri(
        "file:///doc.pdf#collab=DAVFDF@http://review/Collab/user1##fdf=http://example.org/doc.fdf#page=2&view=FitV,56".to_owned(),
    );
    assert_eq!(out, Some(expected));
}

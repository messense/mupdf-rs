use percent_encoding::percent_decode_str;
use std::borrow::Cow;

use super::{FileSpec, LinkAction, PdfAction, PdfDestination};
use crate::destination::not_nan;
use crate::pdf::{PdfDocument, PdfObject};
use crate::{DestinationKind, Error, Rect};

/// Parses a [MuPDF-compatible] link URI string into a structured [`PdfAction`] based on the Adobe
/// specification ["Parameters for Opening PDF Files"] from the Adobe Acrobat SDK, version 8.1.
///
/// This is the inverse of [`PdfAction`]'s [`fmt::Display`](std::fmt::Display) implementation.
/// It takes the URI string that MuPDF produces when reading link annotations (via
/// [`pdf_parse_link_action`]) and reconstructs the corresponding [`PdfAction`] variant.
///
/// # Input -> Output mapping
///
/// | Input pattern                                    | Output action                    |
/// |--------------------------------------------------|----------------------------------|
/// | `#page=<N><dest_params>`                         | `GoTo(Page { page: N-1, kind })` |
/// | `#nameddest=<encoded>`                           | `GoTo(Named(decoded_name))`      |
/// | `#<raw_name (encoded)>`                          | `GoTo(Named(decoded_name))`      |
/// | `file://<path>.pdf#<params>`                     | `GoToR { file: Path(..), dest }` |
/// | `file:<path>.pdf#<params>`                       | `GoToR { file: Path(..), dest }` |
/// | `<scheme>://<host>/<..>.pdf#<params>` (external) | `GoToR { file: Url(..),  dest }` |
/// | `<path>.pdf#<params>` (no scheme)                | `GoToR { file: Path(..), dest }` |
/// | `file:<path>` (non-PDF)                          | `Launch(Path(..))`               |
/// | `<local_path>` (non-external, non-PDF)           | `Launch(Path(..))`               |
/// | `<scheme>://<..>` (non-PDF, external)            | `Uri(uri)`                       |
///
/// where:
///
/// - `<N>` is a 1-based page number (converted to 0-based)
/// - `<dest_params>` are `&view=` / `&zoom=` / `&viewrect=` parameters, parsed into a [`DestinationKind`]
///   (see [`pdf_new_explicit_dest_from_uri`] and ["Parameters for Opening PDF Files"])
/// - `<params>` in the `GoToR` rows are parsed as:
///   - explicit destination parameters
///   - a named destination
///   - or [`PdfDestination::default`]
///
/// File paths are percent-decoded and normalized (`.`/`..` resolved), matching MuPDF's
/// [`parse_file_uri_path`]. Named destinations are percent-decoded matching MuPDF's
/// [`fz_decode_uri_component`].
///
/// # Disambiguation
///
/// MuPDF flattens all PDF action types into a single URI string (via [`pdf_parse_link_action`]),
/// so this function uses heuristics to reconstruct the original action type:
///
/// - Fragment-only (`#...`) -> `GoTo`
/// - `.pdf` suffix in path -> `GoToR`
/// - `file:` scheme or non-external path -> `Launch`
/// - External URI (scheme length > 2, matching MuPDF's [`fz_is_external_link`]) -> `Uri`
///
/// A `Launch` with URL-based `FileSpec` (`FS`=`URL`) is indistinguishable from a `URI` action
/// in the flattened string, so all external non-PDF URIs map to [`PdfAction::Uri`].
///
/// # MuPDF source mapping
///
/// | Step                     | MuPDF function(s)                                                         |
/// |--------------------------|---------------------------------------------------------------------------|
/// | Overall reconstruct      | [`pdf_new_action_from_link`]                                              |
/// | `file:` detection        | [`is_file_uri`]                                                           |
/// | External link detection  | [`fz_is_external_link`]                                                   |
/// | Explicit dest parsing    | [`pdf_new_explicit_dest_from_uri`]                                        |
/// | Named dest parsing       | [`parse_uri_named_dest`]                                                  |
/// | File path decoding       | [`parse_file_uri_path`] -> [`fz_decode_uri_component`] + [`fz_cleanname`] |
/// | Named dest decoding      | [`fz_decode_uri_component`]                                               |
///
/// ["Parameters for Opening PDF Files"]: https://web.archive.org/web/20170921000830/http://www.adobe.com/content/dam/Adobe/en/devnet/acrobat/pdfs/pdf_open_parameters.pdf
/// [MuPDF-compatible]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/include/mupdf/pdf/annot.h#L317
/// [`pdf_parse_link_action`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L519
/// [`pdf_new_action_from_link`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1177
/// [`is_file_uri`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L861
/// [`fz_is_external_link`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/fitz/link.c#L68
/// [`pdf_new_explicit_dest_from_uri`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L941
/// [`parse_uri_named_dest`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L908
/// [`parse_file_uri_path`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L886
/// [`fz_decode_uri_component`]: https://github.com/ArtifexSoftware/mupdf/blob/b462c9bd31a7b023e4239b75c38f2e6098805c3e/source/fitz/string.c#L326
/// [`fz_cleanname`]: https://github.com/ArtifexSoftware/mupdf/blob/b462c9bd31a7b023e4239b75c38f2e6098805c3e/source/fitz/string.c#L469
pub(crate) fn parse_external_link(uri: &str) -> Option<PdfAction> {
    let uri = uri.trim();
    if uri.is_empty() {
        return None;
    }
    let (head, params) = uri
        .split_once('#')
        .map(|(head, params)| (head.trim(), params.trim()))
        .unwrap_or((uri, ""));

    // MuPDF: fragment-only URIs map to GoTo (`pdf_new_action_from_link` function)
    // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1189
    if head.is_empty() {
        let dest = match parse_params(params) {
            ParsedFragment::Explicit(page, kind) => PdfDestination::Page { page, kind },
            ParsedFragment::Named(name) => PdfDestination::Named(name),
            ParsedFragment::ContainsUnknownKeys => {
                // Matches MuPDF's `parse_uri_named_dest` behavior
                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L908
                let name = strip_prefix_icase(params, "nameddest=")
                    .map(|raw| raw.split_once(['&', '#']).map_or(raw, |(head, _)| head))
                    .unwrap_or(params);
                PdfDestination::Named(decode_uri_component(name).into_owned())
            }
            ParsedFragment::Empty => return None,
        };

        return Some(PdfAction::GoTo(dest));
    }

    // MuPDF: `is_file_uri` checks for the "file:" scheme
    // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L861
    // If the path starts with "/", MuPDF adds the "file://" prefix; otherwise, it adds the "file:" prefix
    // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1066
    // also MuPDF adds "file://" to unresolved Uri the
    // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L539
    let (link, is_explicit_file) = strip_prefix_icase(head, "file://")
        .or_else(|| strip_prefix_icase(head, "file:"))
        .map(|path| (path, true))
        .unwrap_or((head, false));

    if is_pdf_path(link) {
        let dest = match parse_params(params) {
            ParsedFragment::Empty => PdfDestination::default(),
            ParsedFragment::Explicit(page, kind) => PdfDestination::Page { page, kind },
            ParsedFragment::Named(name) => PdfDestination::Named(name),
            ParsedFragment::ContainsUnknownKeys => return Some(PdfAction::Uri(uri.to_owned())),
        };

        let is_url = !is_explicit_file && is_external_link(link);

        let file = if is_url {
            FileSpec::Url(link.to_string())
        } else {
            // MuPDF: `parse_file_uri_path` does decode + cleanname
            // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L896
            FileSpec::Path(decode_and_clean_path(link))
        };

        return Some(PdfAction::GoToR { file, dest });
    }

    // MuPDF flattens `Launch` and `URI` actions into a single URI string, normalizing local files
    // to `file:` scheme or relative paths. We use heuristics to reconstruct the action type:
    //
    // 1. `file:` scheme or URIs that do not look like external links (local paths) -> `Launch`.
    // 2. URIs that look like external links (http, mailto, etc.) -> `Uri`.
    //
    // Note: The PDF specification allows a `Launch` with URL `FileSpec` (FS entry = URL).
    // However, it is impossible to distinguish a "Launch with URL FileSpec" from a standard
    // "URI Action" based solely on the flattened string provided by MuPDF. Therefore,
    // all detected files are mapped to `FileSpec::Path`, and all web links to `PdfAction::Uri`.
    //
    // Look at MuPDF's `pdf_parse_link_action`/`convert_file_spec_to_URI` functions:
    //
    // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L519
    // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L288

    if link.is_empty() && is_explicit_file {
        return None;
    }

    let action = if is_explicit_file {
        PdfAction::Launch(FileSpec::Path(decode_and_clean_path(link)))
    } else if !is_external_link(uri) {
        PdfAction::Launch(FileSpec::Path(decode_and_clean_path(uri)))
    } else {
        // URI entries are ASCII strings, so we return uri as it is
        // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L545
        PdfAction::Uri(uri.to_string())
    };
    Some(action)
}

/// Rust port of [`fz_is_external_link`] function.
///
/// Checks if a string represents an external web URI (e.g., `http`, `mailto`).
///
/// # Heuristic
///
/// Checks for a colon (`:`) that appears at an index greater than 2 (i.e., the scheme length must be
/// at least 3 characters). This constraint is used to distinguish external links from
/// DOS/Windows drive letters (e.g., `C:` or `D:`).
///
/// [`fz_is_external_link`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/fitz/link.c#L68
pub(super) fn is_external_link(uri: &str) -> bool {
    let Some((scheme, _)) = uri.split_once(':') else {
        return false;
    };
    if scheme.len() < 3 || !scheme.as_bytes()[0].is_ascii_alphabetic() {
        return false;
    }
    scheme[1..]
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
}

/// Heuristic for determining whether a URI path targets a PDF file.
/// Returns `true` if `file_name` ends with `.pdf` (case-insensitive).
pub(super) fn is_pdf_path(file_name: &str) -> bool {
    file_name
        .get(file_name.len().saturating_sub(4)..)
        .is_some_and(|extension| extension.eq_ignore_ascii_case(".pdf"))
}

#[derive(Debug, PartialEq)]
enum ParsedFragment {
    Empty,
    Explicit(u32, DestinationKind),
    Named(String),
    ContainsUnknownKeys,
}

/// Parses destination parameters from a URI fragment string.
///
/// # Supported Parameters
///
/// - `page`: The target page number (1-based in string, converted to 0-based).
/// - `nameddest`: A named destination (percent-decoded).
/// - `view`: The zoom/view mode (`Fit`, `FitH`, `FitV`, etc.).
/// - `zoom`: XYZ zoom parameters.
/// - `viewrect`: Rectangle parameters for `FitR`.
///
/// # Behavior
///
/// - Iterates left-to-right over key-value pairs.
///
/// - `page` and `nameddest` are mutually exclusive, with the last one seen taking precedence
///   and resetting the other (matching the Adobe specification)
///
/// - Returns [`ParsedFragment::ContainsUnknownKeys`] if an unrecognized parameter key is encountered.
///   This signals that the fragment contains additional options (as per the Adobe specification)
///   and should likely be treated as a standard URI rather than a PDF explicit destination
///   to avoid losing information.
///
/// - Returns [`ParsedFragment::Empty`] if the fragment is empty or contains no parameters at all.
///
/// - Returns [`ParsedFragment::Named`] if:
///   - an explicit `nameddest=` key was the last destination identifier, or
///   - no key-value pairs were recognized (the entire fragment is treated as a raw
///     named destination, matching MuPDF's [`parse_uri_named_dest`] fallback).
///
/// - Returns [`ParsedFragment::Explicit`] if at least one `page` or view parameter was parsed
///   and no `nameddest` followed.
///
/// Named destinations are percent-decoded via [`decode_uri_component`].
///
/// This is the Rust analogue of MuPDF's [`pdf_new_explicit_dest_from_uri`] combined
/// with [`parse_uri_named_dest`].
///
/// [`parse_uri_named_dest`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L908
/// [`pdf_new_explicit_dest_from_uri`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L941
fn parse_params(params: &str) -> ParsedFragment {
    if params.is_empty() {
        return ParsedFragment::Empty;
    }
    let mut page = None;
    let mut kind = None;
    let mut named_dest = None;

    for (k, v) in fragment_kv_pairs(params) {
        if k.eq_ignore_ascii_case("page") {
            if let Some(new_page) = parse_page_1based_to_0based(v) {
                page = Some(new_page);
                // reset view state on page change
                kind = Some(DestinationKind::default());
                named_dest = None; // explicit page overrides named dest
            }
            continue;
        }

        if k.eq_ignore_ascii_case("nameddest") {
            named_dest = Some(v);
            page = None; // named dest overrides explicit page
            kind = None;
            continue;
        }

        let new_kind = if k.eq_ignore_ascii_case("viewrect") {
            parse_viewrect(v)
        } else if k.eq_ignore_ascii_case("zoom") {
            Some(parse_zoom(v))
        } else if k.eq_ignore_ascii_case("view") {
            parse_view(v)
        } else {
            return ParsedFragment::ContainsUnknownKeys;
        };

        if let Some(k) = new_kind {
            kind = Some(k);
        }
    }

    if let Some(name) = named_dest {
        return if !name.is_empty() {
            ParsedFragment::Named(decode_uri_component(name).into())
        } else {
            ParsedFragment::Empty
        };
    }

    match (page, kind) {
        (None, None) => ParsedFragment::Named(decode_uri_component(params).into()),
        (p, k) => ParsedFragment::Explicit(p.unwrap_or_default(), k.unwrap_or_default()),
    }
}

/// Iterates over `key=value` pairs in a URI fragment string.
///
/// - Splits on `&` or `#`.
/// - Skips segments that contain no `=` sign.
fn fragment_kv_pairs(fragment: &str) -> impl Iterator<Item = (&str, &str)> {
    fragment
        .split(['&', '#'])
        .filter_map(|part| part.split_once('='))
}

/// Parses a 1-based page number string to a 0-based integer. Returns 0 if the result
/// would be negative. Mirrors MuPDF's logic in [`pdf_new_explicit_dest_from_uri:957`].
///
/// [`pdf_new_explicit_dest_from_uri:957`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L957
fn parse_page_1based_to_0based(s: &str) -> Option<u32> {
    let n: i32 = s.parse().ok()?;
    if n < 2 {
        Some(0)
    } else {
        Some(n as u32 - 1)
    }
}

/// Parses a `viewrect=` fragment parameter into a [`DestinationKind::FitR`].
/// Requires 4 comma-separated floats (left, bottom, width, height).
///
/// Mirrors MuPDF's parsing in [`pdf_new_explicit_dest_from_uri:963`].
///
/// [`pdf_new_explicit_dest_from_uri:963`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L963
fn parse_viewrect(s: &str) -> Option<DestinationKind> {
    let mut floats = FloatParser::new(s);
    let x = floats.next_finite()?;
    let y = floats.next_finite()?;
    let w = floats.next_finite()?;
    let h = floats.next_finite()?;

    Some(DestinationKind::FitR {
        left: x,
        bottom: y,
        right: x + w,
        top: y + h,
    })
}

/// Parses a `zoom=` fragment parameter into a [`DestinationKind::XYZ`].
///
/// Format: `zoom=[scale,left,top]`. All fields except the first are optional.
/// The `scale` (zoom) is in percent (e.g., `150` for 150%). A value `≤ 0` or `inf`
/// is normalized to `100%`.
///
/// Mirrors MuPDF's parsing in [`pdf_new_explicit_dest_from_uri:972`].
///
/// [`pdf_new_explicit_dest_from_uri:972`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L972
fn parse_zoom(s: &str) -> DestinationKind {
    let mut floats = FloatParser::new(s);
    let zoom = floats.next_not_nan().map(|n| {
        if n <= 0.0 || n.is_infinite() {
            100.0
        } else {
            n
        }
    });

    DestinationKind::XYZ {
        left: floats.next_finite(),
        top: floats.next_finite(),
        zoom,
    }
}

/// Parses a `view=` fragment parameter into a [`DestinationKind`] variant
/// (Fit, FitH, FitV, etc.).
///
/// Mirrors MuPDF's parsing in [`pdf_new_explicit_dest_from_uri:983`].
///
/// [`pdf_new_explicit_dest_from_uri:983`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L983
fn parse_view(s: &str) -> Option<DestinationKind> {
    if s.is_empty() {
        return None;
    }

    let mut iter = s.split(',').map(str::trim);
    if let Some(key) = iter.next() {
        if key.eq_ignore_ascii_case("Fit") {
            return Some(DestinationKind::Fit);
        } else if key.eq_ignore_ascii_case("FitB") {
            return Some(DestinationKind::FitB);
        }

        let val = iter
            .next()
            .and_then(|s| s.parse::<f32>().ok())
            .filter(|num| num.is_finite());

        if key.eq_ignore_ascii_case("FitH") {
            return Some(DestinationKind::FitH { top: val });
        } else if key.eq_ignore_ascii_case("FitBH") {
            return Some(DestinationKind::FitBH { top: val });
        } else if key.eq_ignore_ascii_case("FitV") {
            return Some(DestinationKind::FitV { left: val });
        } else if key.eq_ignore_ascii_case("FitBV") {
            return Some(DestinationKind::FitBV { left: val });
        }
    }

    None
}

/// Helper for parsing comma-separated `f32` values.
struct FloatParser<'a>(std::str::Split<'a, char>);

impl<'a> FloatParser<'a> {
    fn new(s: &'a str) -> Self {
        Self(s.split(','))
    }

    fn next(&mut self) -> Option<f32> {
        self.0.next().and_then(|p| p.trim().parse::<f32>().ok())
    }

    fn next_finite(&mut self) -> Option<f32> {
        self.next().filter(|num| num.is_finite())
    }

    fn next_not_nan(&mut self) -> Option<f32> {
        self.next().filter(|num| !num.is_nan())
    }
}

/// Strips `pat` from the start of string using ASCII case-insensitive matching.
/// Returns Some(remainder) if the prefix matches, otherwise None.
fn strip_prefix_icase<'a>(s: &'a str, pat: &str) -> Option<&'a str> {
    let len = pat.len();
    s.get(..len)
        .filter(|head| head.eq_ignore_ascii_case(pat))
        .and_then(|_| s.get(len..))
}

/// Decodes URL-encoded sequences and normalizes the path.
/// See [`decode_uri_component`] and [`cleanname`] documentation.
fn decode_and_clean_path(path: &str) -> String {
    let decoded = decode_uri_component(path);
    cleanname(&decoded)
}

/// Normalizes a path, removing . and .. segments and collapsing duplicate slashes.
/// Mirrors MuPDF's [`fz_cleanname`] behavior
///
/// [`fz_cleanname`]: https://github.com/ArtifexSoftware/mupdf/blob/b462c9bd31a7b023e4239b75c38f2e6098805c3e/source/fitz/string.c#L469
fn cleanname(name: &str) -> String {
    let rooted = name.starts_with('/');
    let mut parts = Vec::with_capacity(8);
    let mut dotdot_depth: usize = 0;

    for component in name.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                if !rooted && parts.len() <= dotdot_depth {
                    // relative path and can't backtrack — keep the ".."
                    parts.push("..");
                    dotdot_depth += 1;
                } else if parts.len() > dotdot_depth {
                    parts.pop();
                }
                // rooted path: ".." at root is just ignored
            }
            part => parts.push(part),
        }
    }

    if parts.is_empty() {
        return if rooted { "/".into() } else { ".".into() };
    }

    let cap = rooted as usize + parts.iter().map(|p| p.len()).sum::<usize>() + parts.len() - 1;

    let mut out = String::with_capacity(cap);
    if rooted {
        out.push('/');
    }
    out.push_str(parts[0]);
    for part in &parts[1..] {
        out.push('/');
        out.push_str(part);
    }
    out
}

/// Unescapes URL-encoded sequences (%XX) in a string.
///
/// Uses RFC 3986 compliant decoding via the percent-encoding crate.
/// Falls back to the original string if the decoded bytes are not valid UTF-8.
///
/// The same as MuPDF's [`fz_decode_uri_component`] function
///
/// [`fz_decode_uri_component`]: https://github.com/ArtifexSoftware/mupdf/blob/b462c9bd31a7b023e4239b75c38f2e6098805c3e/source/fitz/string.c#L326
pub(super) fn decode_uri_component(s: &str) -> Cow<'_, str> {
    percent_decode_str(s)
        .decode_utf8()
        .unwrap_or(Cow::Borrowed(s))
}

/// Parses a [`LinkAction`] from a link annotation's PDF dictionary.
///
/// Reads entries in this priority (matching [`pdf_load_link`] in MuPDF):
/// 1. `/Dest` entry reads as [`LinkAction::Dest`] (see [`parse_dest_value`]).
/// 2. `/A` (Action) dictionary reads as [`LinkAction::Action`] (see [`parse_action_dict`]).
/// 3. `/AA` (Additional Actions) reads as [`LinkAction::Action`]: tries `/D` (mouse-down)
///    then `/U` (mouse-up).
///
/// `page_no` is the 0-based page number where this annotation resides, used to resolve
/// relative `Named` actions (`PrevPage`, `NextPage`). Pass `None` if the page number is
/// unknown. Absolute named actions (`FirstPage`, `LastPage`) are always resolved.
///
/// For `Page { .. }` destinations, the page index is resolved from an indirect page
/// object reference via [`PdfDocument::lookup_page_number`], and coordinates are
/// transformed to Fitz coordinate space (see [`parse_dest_array`]).
///
/// [`pdf_load_link`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L652
pub(crate) fn parse_link_action_from_annot_dict(
    obj: &PdfObject,
    doc: &PdfDocument,
    page_no: Option<i32>,
) -> Result<Option<LinkAction>, Error> {
    // 1. Check /Dest entry first (per PDF spec priority)
    if let Some(dest_obj) = obj.get_dict("Dest")? {
        return parse_dest_value(&dest_obj, doc).map(|dest| dest.map(LinkAction::Dest));
    }

    // 2. Check /A (Action) entry
    if let Some(action_obj) = obj.get_dict("A")? {
        return parse_action_dict(&action_obj, doc, page_no).map(|opt| opt.map(LinkAction::Action));
    }
    // 3. Check /AA (Additional Actions) - D (mouse-down) then U (mouse-up)
    if let Some(add_action_obj) = obj.get_dict("AA")? {
        // ISO 32000-2:2020 (PDF 2.0) - abbreviated names take precedence
        // https://github.com/ArtifexSoftware/mupdf/blob/53d140db017352190eae56b2c5a71d93494dd226/source/pdf/pdf-object.c#L2466
        if let Some(d) = add_action_obj.get_dict("D")? {
            return parse_action_dict(&d, doc, page_no).map(|opt| opt.map(LinkAction::Action));
        }
        if let Some(u) = add_action_obj.get_dict("U")? {
            return parse_action_dict(&u, doc, page_no).map(|opt| opt.map(LinkAction::Action));
        }
    }

    Ok(None)
}

/// Parses a raw destination PDF object defined in [PDF 32000-1:2008, 12.3.2]
/// into a [`PdfDestination`]. This is the Rust analogue of MuPDF's [`pdf_parse_link_dest`].
///
/// [PDF 32000-1:2008, 12.3.2]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.2063217
/// [`pdf_parse_link_dest`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1126
fn parse_dest_value(dest: &PdfObject, doc: &PdfDocument) -> Result<Option<PdfDestination>, Error> {
    if dest.is_array()? && dest.len()? > 0 {
        parse_dest_array(dest, doc).map(Some)
    } else if dest.is_name()? {
        let name = std::str::from_utf8(dest.as_name()?).map_err(|_| Error::InvalidUtf8)?;
        Ok(Some(PdfDestination::Named(name.to_owned())))
    } else if dest.is_string()? {
        Ok(Some(PdfDestination::Named(dest.as_string()?.to_owned())))
    } else {
        Ok(None)
    }
}

/// Parses an Explicit Destination array ([PDF 32000-1:2008, 12.3.2.2]) into a
/// [`PdfDestination::Page`].
///
/// Expected format is `[page_ref, /View, ...params]`, where:
/// - `page_ref` is an indirect reference to a page object (for local destinations)
///   or an integer page number (for remote `GoToR` destinations).
/// - `/View` is one of the supported fit types: `/XYZ`, `/Fit`, `/FitH`, `/FitV`,
///   `/FitR`, `/FitB`, `/FitBH`, or `/FitBV`.
///
/// # Behavior
///
/// - Local destinations coordinates are transformed to Fitz space (Y-axis down) using the
///   target page's CTM.
/// - The resolved page index is clamped to `[0..page_count - 1]`.
///
/// Mirrors MuPDF's [`populate_destination`].
///
/// [PDF 32000-1:2008, 12.3.2.2]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.1696125
/// [`populate_destination`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L66
fn parse_dest_array(array: &PdfObject, doc: &PdfDocument) -> Result<PdfDestination, Error> {
    let page_obj = array
        .get_array(0)?
        .ok_or_else(|| Error::InvalidDestination("missing page reference in dest array".into()))?;

    let (page_idx, page_obj) = if page_obj.is_int()? {
        let idx = page_obj.as_int()?;
        (idx, doc.find_page(idx)?)
    } else {
        (doc.lookup_page_number(&page_obj)?, page_obj)
    };

    let page_count = doc.page_count()?;
    let page = if page_count > 0 {
        page_idx.clamp(0, page_count - 1)
    } else {
        0
    };

    let mut kind = DestinationKind::decode_from(array)?;

    // TODO: Find a way to use DestinationKind::transform here.
    if page_obj.is_dict()? {
        let ctm = page_obj.page_ctm()?;
        kind = match kind {
            DestinationKind::FitH { top } => DestinationKind::FitH {
                top: top.map(|t| ctm.transform_xy(0.0, t).1),
            },
            DestinationKind::FitBH { top } => DestinationKind::FitBH {
                top: top.map(|t| ctm.transform_xy(0.0, t).1),
            },
            DestinationKind::FitV { left } => DestinationKind::FitV {
                left: left.map(|l| ctm.transform_xy(l, 0.0).0),
            },
            DestinationKind::FitBV { left } => DestinationKind::FitBV {
                left: left.map(|l| ctm.transform_xy(l, 0.0).0),
            },
            DestinationKind::XYZ { left, top, zoom } => {
                // MuPDF uses 0.0 for missing values before transform, then
                // checks original presence afterward (pdf-link.c:137-141).
                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L137
                let (tx, ty) = ctm.transform_xy(left.unwrap_or(0.0), top.unwrap_or(0.0));
                DestinationKind::XYZ {
                    left: left.and(not_nan(tx)),
                    top: top.and(not_nan(ty)),
                    zoom,
                }
            }
            DestinationKind::FitR {
                left,
                bottom,
                right,
                top,
            } => {
                // Explicit normalization matching pdf-link.c:149-152
                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L149
                let tr = Rect::new(left, bottom, right, top).transform(&ctm);
                DestinationKind::FitR {
                    left: tr.x0.min(tr.x1),
                    bottom: tr.y0.min(tr.y1),
                    right: tr.x0.max(tr.x1),
                    top: tr.y0.max(tr.y1),
                }
            }
            kind => kind,
        }
    }

    Ok(PdfDestination::Page {
        page: page as u32,
        kind,
    })
}

/// Dispatches on the `/S` (action sub-type) entry of a PDF action dictionary and
/// constructs the corresponding [`PdfAction`] ([PDF 32000-1:2008, 12.6.4]).
///
/// `page_no` is the 0-based page number where this annotation resides, used to resolve
/// relative `Named` actions (`PrevPage`, `NextPage`). Pass `None` if the page number is
/// unknown. Absolute named actions (`FirstPage`, `LastPage`) are always resolved.
///
/// This is the Rust analogue of MuPDF's [`pdf_parse_link_action`].
///
/// # Supported action types
///
/// | `/S` value | Key(s) read          | Result                                              |
/// |------------|----------------------|-----------------------------------------------------|
/// | `GoTo`     | `/D` dest            | `PdfAction::GoTo(_)` via [`parse_dest_value`]       |
/// | `URI`      | `/URI` string        | `PdfAction::Uri(_)`, base URI prepended if relative |
/// | `GoToR`    | `/F` file, `/D` dest | `PdfAction::GoToR { .. }` via [`parse_filespec`]    |
/// | `Launch`   | `/F` file            | `PdfAction::Launch(_)` via [`parse_filespec`]       |
/// | `Named`    | `/N` name            | `PdfAction::GoTo(Page { .. })` for known names      |
///
/// [PDF 32000-1:2008, 12.6.4]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.1697199
/// [`pdf_parse_link_action`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L519
fn parse_action_dict(
    action: &PdfObject,
    doc: &PdfDocument,
    page_no: Option<i32>,
) -> Result<Option<PdfAction>, Error> {
    let Some(type_obj) = action.get_dict("S")? else {
        return Ok(None);
    };

    match type_obj.as_name()? {
        b"GoTo" => {
            let Some(dest_obj) = action.get_dict("D")? else {
                return Ok(None);
            };
            let dest = parse_dest_value(&dest_obj, doc)?;
            Ok(dest.map(PdfAction::GoTo))
        }
        b"URI" => {
            let Some(uri_obj) = action.get_dict("URI")? else {
                return Ok(None);
            };
            let uri = uri_obj.as_string()?.to_owned();
            // MuPDF prepends the document URI base for non-external URIs.
            // pdf-link.c:536-543
            if is_external_link(&uri) {
                Ok(Some(PdfAction::Uri(uri)))
            } else {
                let base = doc
                    .trailer()?
                    .get_dict("Root")?
                    .and_then(|root| root.get_dict("URI").ok().flatten())
                    .and_then(|uri_dict| uri_dict.get_dict("Base").ok().flatten())
                    .and_then(|base_obj| base_obj.as_string().ok().map(|s| s.to_owned()));
                let mut full_uri = base.unwrap_or_else(|| "file://".to_owned());
                full_uri.reserve(uri.len());
                full_uri.push_str(&uri);
                Ok(Some(PdfAction::Uri(full_uri)))
            }
        }
        b"GoToR" => {
            let file = match action.get_dict("F")? {
                Some(f) => parse_filespec(&f)?,
                None => return Ok(None),
            };
            let dest = match action.get_dict("D")? {
                Some(dest) => {
                    if dest.is_array()? && dest.len()? > 0 {
                        // Remote dest arrays use integer page numbers
                        let page_obj = dest.get_array(0)?.ok_or_else(|| {
                            Error::InvalidDestination("missing page in GoToR dest".into())
                        })?;
                        let page = page_obj.as_int()?.max(0) as u32;
                        let kind = DestinationKind::decode_from(&dest)?;
                        PdfDestination::Page { page, kind }
                    } else if dest.is_name()? {
                        let name =
                            std::str::from_utf8(dest.as_name()?).map_err(|_| Error::InvalidUtf8)?;
                        PdfDestination::Named(name.to_owned())
                    } else if dest.is_string()? {
                        PdfDestination::Named(dest.as_string()?.to_owned())
                    } else {
                        PdfDestination::default()
                    }
                }
                None => PdfDestination::default(),
            };
            Ok(Some(PdfAction::GoToR { file, dest }))
        }
        b"Launch" => match action.get_dict("F")? {
            Some(f) => Ok(Some(PdfAction::Launch(parse_filespec(&f)?))),
            None => Ok(None),
        },
        // Named action (S=Named): navigates to a well-known page using the /N entry.
        // MuPDF: pdf_parse_link_action, lines 558-580
        // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L558
        b"Named" => {
            let Some(dest_obj) = action.get_dict("N")? else {
                return Ok(None);
            };
            let total = doc.page_count()?;
            let target = match dest_obj.as_name()? {
                b"FirstPage" => Some(0),
                b"LastPage" => Some((total - 1).max(0)),
                b"PrevPage" => page_no.map(|p| (p - 1).max(0)),
                b"NextPage" => page_no.map(|p| (p + 1).min(total - 1)),
                _ => return Ok(None),
            };
            let Some(page) = target else {
                return Ok(None);
            };
            Ok(Some(PdfAction::GoTo(PdfDestination::Page {
                page: page as u32,
                kind: DestinationKind::default(),
            })))
        }
        _ => Ok(None),
    }
}

/// Parses a PDF file specification object ([PDF 32000-1:2008, 7.11.3]) into a [`FileSpec`].
///
/// | Object type                                 | Result                                                   |
/// |---------------------------------------------|----------------------------------------------------------|
/// | String                                      | `FileSpec::Path(string)`                                 |
/// | Dict with `FS = /URL` and `F` entry         | `FileSpec::Url(F value)`                                 |
/// | Dict with `FS = /URL` but missing `F`       | falls back to name keys and returns `FileSpec::Path(..)` |
/// | Dict without `FS = /URL`, with a name entry | `FileSpec::Path(name value)`                             |
///
/// # `FS = /URL` and key precedence
///
/// Per [PDF 32000-1:2008, 7.11.5], URL file specifications store the URL in the `F`
/// entry (7-bit ASCII URL string). This parser therefore only returns
/// [`FileSpec::Url`] when `FS = /URL` and `F` is a string.
///
/// If `FS = /URL` is present but `F` is absent or invalid, we do **not** reinterpret
/// `UF`, `Unix`, `DOS`, or `Mac` as URL text, as MuPDF does, since `F` should be
/// only a 7-bit ASCII URL string, whereas `UF`, for example, is Unicode text.
/// Those keys are treated via the normal filename fallback and parsed as
/// [`FileSpec::Path`]. MuPDF most likely does this unintentionally: it first [`resolves`]
/// a MuPDF file string (from `UF`, `F`, `Unix`, `DOS`, or `Mac`, see [`get_file_stream_and_name`])
/// and only [`afterward`] checks whether the resulting string is a URL.
///
/// This is the Rust analogue of MuPDF's [`convert_file_spec_to_URI`].
///
/// [PDF 32000-1:2008, 7.11.3]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G6.1640878
/// [PDF 32000-1:2008, 7.11.5]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G6.1640997
/// [`convert_file_spec_to_URI`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L287
/// [`resolves`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L296
/// [`get_file_stream_and_name`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L226
/// [`afterward`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L304
fn parse_filespec(obj: &PdfObject) -> Result<FileSpec, Error> {
    if obj.is_string()? {
        return Ok(FileSpec::Path(obj.as_string()?.to_owned()));
    }

    if obj.is_dict()? {
        // Check if it's a URL-based file spec
        if let Some(fs) = obj.get_dict("FS")? {
            if fs.is_name()? && fs.as_name()? == b"URL" {
                // If `FS = /URL` is present but `F` is absent or invalid, we do not reinterpret
                // `UF`, `Unix`, `DOS`, or `Mac` as URL text, as MuPDF does, since `F` should be
                // only a 7-bit ASCII URL string, whereas `UF`, for example, is Unicode text.
                if let Some(f) = obj.get_dict("F")? {
                    return Ok(FileSpec::Url(f.as_string()?.to_owned()));
                }
            }
        }

        if let Some(name) = get_file_name(obj)? {
            if name.is_string()? {
                return Ok(FileSpec::Path(name.as_string()?.to_owned()));
            }
        }
    }

    Err(Error::InvalidDestination(
        "invalid file specification object".into(),
    ))
}

/// Returns the filename object from a PDF file specification dictionary.
///
/// Tries the following keys in priority order, returning the value of the first
/// one present: `UF`, `F`, `Unix`, `DOS`, `Mac`.
///
/// This mirrors the name-lookup portion of MuPDF's [`get_file_stream_and_name`].
///
/// [`get_file_stream_and_name`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L225
fn get_file_name(fs: &PdfObject) -> Result<Option<PdfObject>, Error> {
    for key in ["UF", "F", "Unix", "DOS", "Mac"] {
        if let Some(v) = fs.get_dict(key)? {
            return Ok(Some(v));
        }
    }
    Ok(None)
}

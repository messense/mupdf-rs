//! Link annotation extraction and creation helpers for PDFs.
//!
//! This module provides PDF-specific link handling built on MuPDF's low-level APIs.

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt;

use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};

use crate::pdf::{PdfDocument, PdfObject};
use crate::{DestinationKind, Error, Matrix, Rect};

/// Percent-encoding set matching MuPDF's [`URIUNESCAPED`] (RFC 2396 unreserved characters).
/// Encodes everything except: alphanumeric, `-`, `_`, `.`, `!`, `~`, `*`, `'`, `(`, `)`.
///
/// [`URIUNESCAPED`]: https://github.com/ArtifexSoftware/mupdf/blob/b462c9bd31a7b023e4239b75c38f2e6098805c3e/source/fitz/string.c#L298
const URI_COMPONENT_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'!')
    .remove(b'~')
    .remove(b'*')
    .remove(b'\'')
    .remove(b'(')
    .remove(b')');

/// Same as [`URI_COMPONENT_SET`] but also preserves `/` for path encoding.
/// Matches MuPDF's [`fz_encode_uri_pathname`].
///
/// [`fz_encode_uri_pathname`]: https://github.com/ArtifexSoftware/mupdf/blob/b462c9bd31a7b023e4239b75c38f2e6098805c3e/source/fitz/string.c#L408
const URI_PATH_SET: &AsciiSet = &URI_COMPONENT_SET.remove(b'/');

mod build;
pub(crate) use build::build_link_annotation;
pub(crate) use build::set_link_action_on_annot_dict;

mod extraction;
pub(crate) use extraction::parse_external_link;
pub(crate) use extraction::parse_link_action_from_annot_dict;

mod link_annot;
pub use link_annot::PdfLinkAnnot;

#[cfg(test)]
mod tests_build;
#[cfg(test)]
mod tests_extraction;
#[cfg(test)]
mod tests_format;
#[cfg(test)]
mod tests_link_annot;

/// Extracted link data from a source page.
/// Contains all information needed to reconstruct the link in the destination document.
#[derive(Debug, Clone, PartialEq)]
pub struct PdfLink {
    /// Link rectangle in Fitz coordinate space.
    pub bounds: Rect,
    /// Link action or destination (see [PDF 32000-1:2008, 12.5.6.5], Table 173).
    ///
    /// [PDF 32000-1:2008, 12.5.6.5]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.1951136
    pub action: LinkAction,
}

/// A link annotation's action or destination entry (see [PDF 32000-1:2008, 12.5.6.5], Table 173).
///
/// [PDF 32000-1:2008, 12.5.6.5]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.1951136
#[derive(Debug, Clone, PartialEq)]
pub enum LinkAction {
    /// Action dictionary (`A` entry in the annotation dictionary) (see [PDF 32000-1:2008, 12.6.4]).
    ///
    /// [PDF 32000-1:2008, 12.6.4]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.1697199
    Action(PdfAction),
    /// Direct destination (`Dest` entry in the annotation dictionary) (see [PDF 32000-1:2008, 12.3.2]).
    ///
    /// [PDF 32000-1:2008, 12.3.2]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.2063217
    Dest(PdfDestination),
}

impl LinkAction {
    /// Converts this `LinkAction` into a [`PdfAction`].
    ///
    /// `Dest(d)` becomes `PdfAction::GoTo(d)`, `Action(a)` passes through.
    pub fn into_pdf_action(self) -> PdfAction {
        match self {
            LinkAction::Action(a) => a,
            LinkAction::Dest(d) => PdfAction::GoTo(d),
        }
    }

    /// Returns the destination, if any.
    ///
    /// Returns `Some` for `Dest(d)` or `Action(GoTo(d))`.
    pub fn destination(&self) -> Option<&PdfDestination> {
        match self {
            LinkAction::Action(PdfAction::GoTo(d)) => Some(d),
            LinkAction::Dest(d) => Some(d),
            _ => None,
        }
    }

    /// Convenience method that returns the [`Display`](fmt::Display) output as an owned `String`.
    ///
    /// See [`fmt::Display`] impl for output format details and MuPDF source references.
    pub fn to_uri(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for LinkAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LinkAction::Action(action) => write!(f, "{action}"),
            LinkAction::Dest(dest) => write!(f, "#{dest}"),
        }
    }
}

impl From<PdfAction> for LinkAction {
    fn from(action: PdfAction) -> Self {
        LinkAction::Action(action)
    }
}

impl From<PdfDestination> for LinkAction {
    fn from(dest: PdfDestination) -> Self {
        LinkAction::Dest(dest)
    }
}

/// PDF link destination representing an action associated with a link annotation
/// (see [PDF 32000-1:2008, 12.6.4]).
///
/// Maps the standard PDF action types — GoTo, GoToR, Launch, and URI to Rust variants.
/// Each variant corresponds to a specific action dictionary `S` (type) value defined in Table 198.
///
/// [PDF 32000-1:2008, 12.6.4]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.1697199
#[derive(Debug, Clone, PartialEq)]
pub enum PdfAction {
    /// Go-to action (`S`=`GoTo`): changes the view to a destination in the current document
    /// (see PDF 32000-1:2008, [12.6.4.2], Table 199).
    ///
    /// The `D` entry in the action dictionary specifies the destination to jump to,
    /// represented here as a [`PdfDestination`].
    ///
    /// [12.6.4.2]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.1963731
    GoTo(PdfDestination),
    /// Remote go-to action (`S`=`GoToR`): jumps to a destination in another PDF file
    /// (see PDF 32000-1:2008, [12.6.4.3], Table 200).
    ///
    /// `file` is the remote file specification (`F` entry) and `dest` is the
    /// destination within that file (`D` entry).
    ///
    /// [12.6.4.3]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.1951685
    GoToR {
        file: FileSpec,
        dest: PdfDestination,
    },
    /// Launch action (`S`=`Launch`): launches an application or opens/prints a document
    /// (see PDF 32000-1:2008, [12.6.4.5], Table 203).
    ///
    /// The `F` entry in the action dictionary specifies the file to be launched,
    /// represented here as a [`FileSpec`].
    ///
    /// [12.6.4.5]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.1952224
    Launch(FileSpec),
    /// URI action (`S`=`URI`): resolves a uniform resource identifier
    /// (see PDF 32000-1:2008, [12.6.4.7], Table 206).
    ///
    /// **Constraint:** The value should be a [7-bit ASCII] string (e.g., `"https://example.com"`).
    ///
    /// [12.6.4.7]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.1939903
    Uri(String),
}

impl PdfAction {
    /// Convenience method that returns the [`Display`](fmt::Display) output as an owned `String`.
    ///
    /// See [`fmt::Display`] impl for output format details and MuPDF source references.
    pub fn to_uri(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for PdfAction {
    /// Formats this action as a [MuPDF-compatible] link URI string based on the Adobe specification
    /// ["Parameters for Opening PDF Files"] from the Adobe Acrobat SDK, version 8.1.
    ///
    /// This is the Rust analogue of MuPDF's [`pdf_parse_link_action`] output - the URI string
    /// that MuPDF produces when reading link annotations from a PDF document. MuPDF uses this
    /// URI format both internally and in its public API as the canonical representation of link
    /// destinations.
    ///
    /// # Output shapes
    ///
    /// - `GoTo` with explicit dest -> `#page=<N><dest_suffix>`
    /// - `GoTo` with named dest -> `#nameddest=<percent-encoded name>`
    /// - `Uri` -> the URI string as-is
    /// - `Launch` -> `<file_spec_uri>#page=1`
    /// - `GoToR` with explicit dest -> `<file_spec_uri>#page=<N><dest_suffix>`
    /// - `GoToR` with named dest -> `<file_spec_uri>#nameddest=<percent-encoded name>`
    ///
    /// where:
    ///
    /// - `<dest_suffix>` is the [`DestinationKind`] fragment (e.g. `&view=Fit`, `&zoom=100,10,20`)
    /// - `<file_spec_uri>` is the [`FileSpec`] formatted as a URI prefix:
    ///   - `file://<path>` for absolute [`FileSpec::Path`]
    ///   - `file:<path>` for relative [`FileSpec::Path`]
    ///   - the URL as-is for [`FileSpec::Url`] (see [`convert_file_spec_to_URI`])
    /// - `<N>` is the 1-based page number
    ///
    /// For `GoToR`, the fragment separator is `&` instead of `#` when the file spec URI
    /// already contains a `#` (possible only for [`FileSpec::Url`]).
    ///
    /// # MuPDF source mapping
    ///
    /// | Variant                   | MuPDF function(s)                                                                |
    /// |---------------------------|----------------------------------------------------------------------------------|
    /// | `GoTo(Page { .. })`       | [`pdf_new_uri_from_explicit_dest`] -> [`format_explicit_dest_link_uri`]          |
    /// | `GoTo(Named(..))`         | [`pdf_format_remote_link_uri_from_name`] -> [`format_named_dest_link_uri`]       |
    /// | `Uri(..)`                 | [`pdf_parse_link_action`] (returns URI as-is)                                    |
    /// | `Launch(..)`              | [`pdf_parse_link_action`] -> [`convert_file_spec_to_URI`]                        |
    /// | `GoToR { .. }` (explicit) | [`pdf_new_uri_from_path_and_explicit_dest`] -> [`format_explicit_dest_link_uri`] |
    /// | `GoToR { .. }` (named)    | [`pdf_new_uri_from_path_and_named_dest`] -> [`format_named_dest_link_uri`]       |
    ///
    /// File spec conversion (`FileSpec` -> URI prefix) follows [`convert_file_spec_to_URI`],
    /// and named destination percent-encoding follows [`pdf_append_named_dest_to_uri`].
    ///
    /// ["Parameters for Opening PDF Files"]: https://web.archive.org/web/20170921000830/http://www.adobe.com/content/dam/Adobe/en/devnet/acrobat/pdfs/pdf_open_parameters.pdf
    /// [MuPDF-compatible]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/include/mupdf/pdf/annot.h#L317
    /// [`pdf_parse_link_action`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L519
    /// [`pdf_new_uri_from_explicit_dest`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1120
    /// [`format_explicit_dest_link_uri`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L771
    /// [`pdf_format_remote_link_uri_from_name`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L845
    /// [`format_named_dest_link_uri`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L835
    /// [`convert_file_spec_to_URI`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L288
    /// [`pdf_new_uri_from_path_and_explicit_dest`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1089
    /// [`pdf_new_uri_from_path_and_named_dest`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1052
    /// [`pdf_append_named_dest_to_uri`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1023
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PdfAction::GoTo(dest) => write!(f, "#{dest}"),
            PdfAction::Uri(uri) => {
                // MuPDF: pdf_parse_link_action returns URI as-is
                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L545
                f.write_str(uri)
            }
            PdfAction::Launch(file) => {
                // MuPDF: convert_file_spec_to_URI
                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L288
                let sep = match file {
                    FileSpec::Url(url) if url.contains('#') => '&',
                    _ => '#',
                };
                write!(f, "{file}{sep}page=1")
            }
            PdfAction::GoToR { file, dest } => {
                // MuPDF: convert_file_spec_to_URI
                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L288
                // `FileSpec::Path` never contains '#' (it gets percent-encoded),
                // so only `FileSpec::Url` can already have a fragment.
                let sep = match file {
                    FileSpec::Url(url) if url.contains('#') => '&',
                    _ => '#',
                };
                write!(f, "{file}{sep}{dest}")
            }
        }
    }
}

/// PDF file specification (see [PDF 32000-1:2008, 7.11]).
///
/// Represents a file reference that can be either a local filesystem path
/// (absolute or relative, per [7.11.2]) or a URL-based reference (when `FS` is `URL`,
/// per [7.11.5]).
///
/// [PDF 32000-1:2008, 7.11]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G6.1640832
/// [7.11.2]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G6.1914353
/// [7.11.5]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G6.1640997
#[derive(Debug, Clone, PartialEq)]
pub enum FileSpec {
    /// Local filesystem path (e.g., `/Docs/path/file.pdf`, `path/file.pdf`, or `../file.pdf`).
    ///
    /// This variant accepts a UTF-8 string, covering the full Unicode range.
    ///
    /// When serialized to PDF, this is stored in the `UF` (Unicode File) entry of the
    /// file specification dictionary (see [7.11.2.2]) encoded as UTF-16BE (per PDF 32000-1:2008,
    /// [7.9.2.2]), ensuring cross-platform compatibility for non-ASCII filenames.
    ///
    /// [7.9.2.2]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G6.1957385
    /// [7.11.2.2]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G6.1958046
    Path(String),

    /// URL-based file specification (e.g., `http://example.com/file.pdf`).
    ///
    /// **Constraint:** Must be a 7-bit ASCII string (per PDF 32000-1:2008, [7.11.5]).
    ///
    /// Any characters that are not representable in 7-bit U.S. ASCII or are considered
    /// unsafe according to RFC 1738 must be percent-encoded (escaped).
    ///
    /// Note that for relative URL-based specifications, RFC 1808 rules apply, and
    /// sections such as scheme, query, or fragment are not allowed (per PDF 32000-1:2008, [7.11.2.2]).
    ///
    /// [7.11.2.2]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G6.1958046
    /// [7.11.5]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G6.1640997
    Url(String),
}

impl FileSpec {
    /// Encodes this file specification as a PDF file specification dictionary
    /// (see [PDF 32000-1:2008, 7.11]) and adds it as an indirect object.
    ///
    /// - `Path`: creates a dict with `/Type /Filespec`, `/F` (ASCII-safe), `/UF` (Unicode).
    ///   Rust analogue of MuPDF's [`pdf_add_filespec`].
    /// - `Url`: creates a dict with `/Type /Filespec`, `/FS /URL`, `/F`.
    ///   Rust analogue of MuPDF's [`pdf_add_url_filespec`].
    ///
    /// [PDF 32000-1:2008, 7.11]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G6.1640832
    /// [`pdf_add_filespec`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1223
    /// [`pdf_add_url_filespec`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1268
    pub(crate) fn encode_into(&self, doc: &mut PdfDocument) -> Result<PdfObject, Error> {
        match self {
            FileSpec::Path(path) => {
                let mut spec = doc.new_dict_with_capacity(3)?;
                spec.dict_put("Type", PdfObject::new_name("Filespec")?)?;
                let asciiname: String = path
                    .chars()
                    .map(|c| if matches!(c, ' '..='~') { c } else { '_' })
                    .collect();
                spec.dict_put("F", PdfObject::new_string(&asciiname)?)?;
                spec.dict_put("UF", PdfObject::new_string(path)?)?;
                // MuPDF uses pdf_add_new_dict which creates an indirect object (pdf-link.c:1249)
                doc.add_object(&spec)
            }
            FileSpec::Url(url) => {
                let mut spec = doc.new_dict_with_capacity(3)?;
                spec.dict_put("Type", PdfObject::new_name("Filespec")?)?;
                spec.dict_put("FS", PdfObject::new_name("URL")?)?;
                spec.dict_put("F", PdfObject::new_string(url)?)?;
                // MuPDF uses pdf_add_new_dict which creates an indirect object (pdf-link.c:1268)
                doc.add_object(&spec)
            }
        }
    }
}

impl fmt::Display for FileSpec {
    /// Formats this file specification as a [MuPDF-compatible] URI string.
    ///
    /// Follows MuPDF's [`convert_file_spec_to_URI`] logic:
    /// - `FileSpec::Path` -> `file://<percent-encoded path>` (absolute) or `file:<percent-encoded path>` (relative)
    /// - `FileSpec::Url` -> the URL string as-is
    ///
    /// [`convert_file_spec_to_URI`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L288
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileSpec::Path(path) => {
                let prefix = if path.starts_with('/') {
                    "file://"
                } else {
                    "file:"
                };
                write!(f, "{prefix}{}", utf8_percent_encode(path, URI_PATH_SET))
            }
            FileSpec::Url(url) => f.write_str(url),
        }
    }
}

/// Destination within a PDF document (see [PDF 32000-1:2008, 12.3.2]).
///
/// Represents the `D` entry in both `GoTo` and `GoToR` [`PdfAction`] or
/// `Dest` entry in [`LinkAction`].
///
/// [PDF 32000-1:2008, 12.3.2]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.2063217
#[derive(Debug, Clone, PartialEq)]
pub enum PdfDestination {
    /// Explicit destination: zero-based page number with view settings (e.g., page 0, Fit).
    Page { page: u32, kind: DestinationKind },
    /// Named destination string resolved in the remote document's name tree (e.g., `"Chapter1"`).
    Named(String),
}

impl PdfDestination {
    /// Convenience method that returns the [`Display`](fmt::Display) output as an owned `String`.
    ///
    /// See [`fmt::Display`] impl for output format details and MuPDF source references.
    pub fn to_uri(&self) -> String {
        self.to_string()
    }

    /// Encode as a local destination (for `/GoTo` actions and direct `/Dest` entries).
    ///
    /// - `Page { page, kind }`: resolves page to indirect ref via `resolver`,
    ///   transforms coordinates from Fitz space to PDF user space, and builds
    ///   the destination array `[page_ref, /Kind, params...]`.
    /// - `Named(name)`: returns a PDF string object.
    pub(crate) fn encode_local(
        &self,
        doc: &mut PdfDocument,
        resolver: &mut impl DestPageResolver,
    ) -> Result<PdfObject, Error> {
        match self {
            PdfDestination::Page { page, kind } => {
                // MuPDF: GoTo action + explicit destination array
                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1315
                let mut dest = doc.new_array_with_capacity(6)?;
                let (dest_page_obj, dest_inv_ctm) = resolver.resolve(doc, *page)?;
                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1325
                dest.array_push_ref(dest_page_obj)?;

                // MuPDF uses inv_ctm to transform coordinates
                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1328
                let dest_kind = dest_inv_ctm
                    .as_ref()
                    .map(|inv_ctm| kind.transform(inv_ctm))
                    .unwrap_or(*kind);
                dest_kind.encode_into(&mut dest)?;
                Ok(dest)
            }
            // MuPDF stores the named destination as-is
            // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1297
            // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1192
            PdfDestination::Named(name) => PdfObject::new_string(name),
        }
    }

    /// Encode as a remote destination (for `/GoToR` actions).
    ///
    /// - `Page { page, kind }`: pushes page as integer, encodes kind as-is
    ///   (coordinates are already in PDF default user space).
    /// - `Named(name)`: returns a PDF string object.
    pub(crate) fn encode_remote(&self, doc: &mut PdfDocument) -> Result<PdfObject, Error> {
        match self {
            PdfDestination::Page { page, kind } => {
                let mut dest = doc.new_array_with_capacity(6)?;
                // Push the page as-is.
                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1319
                dest.array_push(PdfObject::new_int(*page as i32)?)?;
                // MuPDF uses an identity matrix to transform coordinates, but we could just not do that
                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1320
                kind.encode_into(&mut dest)?;
                Ok(dest)
            }
            // same as PdfDestination::Named(_) in encode_local
            PdfDestination::Named(name) => PdfObject::new_string(name),
        }
    }
}

impl Default for PdfDestination {
    fn default() -> Self {
        Self::Page {
            page: 0,
            kind: DestinationKind::default(),
        }
    }
}

impl fmt::Display for PdfDestination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PdfDestination::Page { page, kind } => {
                write!(f, "page={}{kind}", page.saturating_add(1))
            }
            PdfDestination::Named(name) => {
                // MuPDF: pdf_append_named_dest_to_uri
                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1023
                write!(
                    f,
                    "nameddest={}",
                    utf8_percent_encode(name, URI_COMPONENT_SET)
                )
            }
        }
    }
}

/// Resolves destination page objects and their inverse CTMs for link building.
///
/// This trait abstracts the retrieval of page objects and their inverse CTMs,
/// allowing callers to control caching strategy. For example, a [`HashMap`]-backed
/// implementation avoids redundant lookups when many links target the same page,
/// while a single-slot implementation is lighter for one-off operations.
pub trait DestPageResolver {
    fn resolve(
        &mut self,
        doc: &PdfDocument,
        page_num: u32,
    ) -> Result<(&PdfObject, Option<&Matrix>), Error>;
}

/// Resolver for bulk operations. Caches pages in a HashMap.
pub struct CachedResolver<'a, F> {
    cache: &'a mut HashMap<u32, (PdfObject, Option<Matrix>)>,
    fn_dest_inv_ctm: F,
}

impl<'a, F> CachedResolver<'a, F> {
    pub fn new(
        cache: &'a mut HashMap<u32, (PdfObject, Option<Matrix>)>,
        fn_dest_inv_ctm: F,
    ) -> Self {
        Self {
            cache,
            fn_dest_inv_ctm,
        }
    }
}

impl<'a, F> DestPageResolver for CachedResolver<'a, F>
where
    F: FnMut(&PdfObject) -> Result<Option<Matrix>, Error>,
{
    fn resolve(
        &mut self,
        doc: &PdfDocument,
        page_num: u32,
    ) -> Result<(&PdfObject, Option<&Matrix>), Error> {
        match self.cache.entry(page_num) {
            Entry::Occupied(entry) => {
                let (obj, mat) = entry.into_mut();
                Ok((obj, mat.as_ref()))
            }
            Entry::Vacant(entry) => {
                let page_obj = doc.find_page(page_num as i32)?;
                let inv_ctm = (self.fn_dest_inv_ctm)(&page_obj)?;
                let (obj, mat) = entry.insert((page_obj, inv_ctm));
                Ok((obj, mat.as_ref()))
            }
        }
    }
}

/// Resolver for single operations. Uses a single `Option` slot to own the data.
pub struct SingleResolver<F> {
    slot: Option<(PdfObject, Option<Matrix>)>,
    fn_dest_inv_ctm: F,
}

impl<F> SingleResolver<F> {
    pub fn new(fn_dest_inv_ctm: F) -> Self {
        Self {
            slot: None,
            fn_dest_inv_ctm,
        }
    }
}

impl<F> DestPageResolver for SingleResolver<F>
where
    F: FnMut(&PdfObject) -> Result<Option<Matrix>, Error>,
{
    fn resolve(
        &mut self,
        doc: &PdfDocument,
        page_num: u32,
    ) -> Result<(&PdfObject, Option<&Matrix>), Error> {
        let page_obj = doc.find_page(page_num as i32)?;
        let inv_ctm = (self.fn_dest_inv_ctm)(&page_obj)?;
        let (obj, mat) = self.slot.insert((page_obj, inv_ctm));
        Ok((obj, mat.as_ref()))
    }
}

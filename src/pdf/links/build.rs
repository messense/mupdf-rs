use std::collections::hash_map::Entry;
use std::collections::HashMap;

use super::{FileSpec, PdfAction, PdfDestination, PdfLink};
use crate::pdf::{PdfDocument, PdfObject};
use crate::{Error, Matrix};

/// Builds a PDF link annotation dictionary from a [`PdfLink`] (see [PDF 32000-1:2008, 12.5.6.5]
/// for link annotations, [12.6.4] for action types).
///
/// This is the Rust analogue of MuPDF's [`pdf_set_link_uri`] -> [`pdf_new_action_from_link`]
/// flow. While MuPDF serializes actions to a URI string and then reconstructs the PDF
/// dictionary from that string, this function builds the action dictionary directly from the
/// structured [`PdfAction`] type.
///
/// # Action dictionary shapes
///
/// | Variant                   | `S` (type) | `D` entry (`URI` for `Uri`) | `F` entry     |
/// |---------------------------|------------|-----------------------------|---------------|
/// | `GoTo(Page { .. })`       | `GoTo`     | `[page_ref, /Kind, ...]`    | -             |
/// | `GoTo(Named(..))`         | `GoTo`     | `(name)`                    | -             |
/// | `Uri(..)`                 | `URI`      | `(uri)`                     | -             |
/// | `GoToR { .. }` (explicit) | `GoToR`    | `[page_int, /Kind, ...]`    | filespec dict |
/// | `GoToR { .. }` (named)    | `GoToR`    | `(name)`                    | filespec dict |
/// | `Launch(..)`              | `Launch`   | -                           | filespec dict |
///
/// where:
///
/// - `page_ref` is an indirect reference to the destination page object (local document)
/// - `page_int` is the zero-based page number as an integer (remote document)
/// - `/Kind` is the PDF destination type name (e.g. `/Fit`, `/XYZ`) followed by its parameters
///   (see [`crate::DestinationKind::encode_into`])
/// - `filespec dict` is a file specification dictionary built by:
///   - [`build_filespec`] for [`FileSpec::Path`]
///   - [`build_url_filespec`] for [`FileSpec::Url`]
///
/// For `GoTo(Page { .. })`, destination coordinates are transformed from MuPDF page space back
/// to PDF default user space using `fn_dest_inv_ctm` (see [`crate::DestinationKind::transform`]).
/// For `GoToR`, coordinates are used as-is (already in PDF default user space).
///
/// # MuPDF source mapping
///
/// | Variant                   | MuPDF function(s)                                                          |
/// |---------------------------|----------------------------------------------------------------------------|
/// | `GoTo(Page { .. })`       | [`pdf_new_action_from_link`] (`#` branch) + [`pdf_new_dest_from_link`]     |
/// | `GoTo(Named(..))`         | [`pdf_new_action_from_link`] (`#` branch) + [`pdf_new_dest_from_link`]     |
/// | `Uri(..)`                 | [`pdf_new_action_from_link`] ([`fz_is_external_link`] branch)              |
/// | `GoToR { .. }`            | [`pdf_new_action_from_link`] (`file:` branch) + [`pdf_new_dest_from_link`] |
/// | `Launch(..)`              | (no direct MuPDF equivalent, see [PDF 32000-1:2008, 12.6.4.5])             |
/// | File spec (path)          | [`pdf_add_filespec`]                                                       |
/// | File spec (URL)           | [`pdf_add_url_filespec`]                                                   |
///
/// [PDF 32000-1:2008, 12.5.6.5]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.1951136
/// [12.6.4]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.1697199
/// [PDF 32000-1:2008, 12.6.4.5]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.1952224
/// [`pdf_set_link_uri`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L614
/// [`pdf_new_action_from_link`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1177
/// [`pdf_new_dest_from_link`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1286
/// [`fz_is_external_link`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/fitz/link.c#L68
/// [`pdf_add_filespec`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1223
/// [`pdf_add_url_filespec`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1268
pub(crate) fn build_link_annotation<F>(
    doc: &mut PdfDocument,
    page_obj: &PdfObject,
    link: &PdfLink,
    annot_inv_ctm: &Option<Matrix>,
    fn_dest_inv_ctm: &mut F,
    page_cache: &mut HashMap<u32, (PdfObject, Option<Matrix>)>,
) -> Result<PdfObject, Error>
where
    F: FnMut(&PdfObject) -> Result<Option<Matrix>, Error>,
{
    let rect = annot_inv_ctm
        .as_ref()
        .map(|inv_ctm| link.bounds.transform(inv_ctm))
        .unwrap_or(link.bounds);

    let mut annot = doc.new_dict_with_capacity(5)?;
    annot.dict_put("Subtype", PdfObject::new_name("Link")?)?;

    let mut rect_array = doc.new_array_with_capacity(4)?;
    rect.encode_into(&mut rect_array)?;
    annot.dict_put("Rect", rect_array)?;

    let mut border_style = doc.new_dict_with_capacity(1)?;
    border_style.dict_put("W", PdfObject::new_int(0)?)?;
    annot.dict_put("BS", border_style)?;

    match &link.action {
        PdfAction::GoTo(dest) => match dest {
            PdfDestination::Page { page, kind } => {
                // MuPDF: GoTo action + explicit destination array
                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1315
                let mut dest = doc.new_array_with_capacity(6)?;

                let (dest_page_obj, dest_inv_ctm) = match page_cache.entry(*page) {
                    Entry::Occupied(entry) => entry.into_mut(),
                    Entry::Vacant(entry) => {
                        let page_obj = doc.find_page(*page as i32)?;
                        let inv_ctm = fn_dest_inv_ctm(&page_obj)?;
                        entry.insert((page_obj, inv_ctm))
                    }
                };
                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1325
                dest.array_push_ref(dest_page_obj)?;

                // MuPDF uses inv_ctm to transform coodinates
                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1328
                let dest_kind = dest_inv_ctm
                    .as_ref()
                    .map(|inv_ctm| kind.transform(inv_ctm))
                    .unwrap_or(*kind);
                dest_kind.encode_into(&mut dest)?;

                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1191
                let mut action = doc.new_dict_with_capacity(2)?;
                action.dict_put("S", PdfObject::new_name("GoTo")?)?;
                action.dict_put("D", dest)?;
                annot.dict_put("A", action)?;
            }
            PdfDestination::Named(name) => {
                let mut action = doc.new_dict_with_capacity(2)?;
                action.dict_put("S", PdfObject::new_name("GoTo")?)?;
                // MuPDF stores the named destination as-is
                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1297
                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1192
                action.dict_put("D", PdfObject::new_string(name)?)?;
                annot.dict_put("A", action)?;
            }
        },
        PdfAction::Uri(uri) => {
            // MuPDF reads a URI action and stores the URI string as-is, since URI entries are ASCII strings
            // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L545
            // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1205
            let mut action = doc.new_dict_with_capacity(2)?;
            action.dict_put("S", PdfObject::new_name("URI")?)?;
            action.dict_put("URI", PdfObject::new_string(uri)?)?;
            annot.dict_put("A", action)?;
        }
        PdfAction::GoToR { file, dest } => {
            // MuPDF: GoToR action uses destination + filespec
            // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1197
            let mut action = doc.new_dict_with_capacity(3)?;
            action.dict_put("S", PdfObject::new_name("GoToR")?)?;

            match dest {
                PdfDestination::Page { page, kind } => {
                    let mut dest = doc.new_array_with_capacity(6)?;
                    // Push the page as-is.
                    // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1319
                    dest.array_push(PdfObject::new_int(*page as i32)?)?;
                    // MuPDF uses an identity matrix to transform coordinates, but we could just not do that
                    // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1320
                    kind.encode_into(&mut dest)?;
                    action.dict_put("D", dest)?;
                }
                PdfDestination::Named(name) => {
                    // same as PdfDestination::Named(_)
                    action.dict_put("D", PdfObject::new_string(name)?)?;
                }
            }

            // Same as MuPDF `pdf_add_filespec_from_link` function
            // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1152
            let file_spec = match file {
                FileSpec::Url(url) => build_url_filespec(doc, url)?,
                FileSpec::Path(path) => build_filespec(doc, path)?,
            };

            action.dict_put("F", file_spec)?;
            annot.dict_put("A", action)?;
        }
        PdfAction::Launch(file) => {
            // Same as MuPDF `pdf_add_filespec_from_link` function
            // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1152
            let file_spec = match file {
                FileSpec::Url(url) => build_url_filespec(doc, url)?,
                FileSpec::Path(path) => build_filespec(doc, path)?,
            };
            // No direct MuPDF code, see PDF 32000-1:2008, section 12.6.4.5, Table 203.
            // https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.1952224
            let mut action = doc.new_dict_with_capacity(2)?;
            action.dict_put("S", PdfObject::new_name("Launch")?)?;
            action.dict_put("F", file_spec)?;
            annot.dict_put("A", action)?;
        }
    }

    annot.dict_put_ref("P", page_obj)?;

    Ok(annot)
}

/// This is the Rust analogue of MuPDF's logic found in `pdf_add_filespec` function
/// (<https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1223>).
fn build_filespec(doc: &PdfDocument, file: &str) -> Result<PdfObject, Error> {
    let mut spec = doc.new_dict_with_capacity(3)?;
    spec.dict_put("Type", PdfObject::new_name("Filespec")?)?;
    let asciiname: String = file
        .chars()
        .map(|c| if matches!(c, ' '..='~') { c } else { '_' })
        .collect();
    spec.dict_put("F", PdfObject::new_string(&asciiname)?)?;
    spec.dict_put("UF", PdfObject::new_string(file)?)?;
    Ok(spec)
}

/// This is the Rust analogue of MuPDF's logic found in `pdf_add_url_filespec` function
/// (<https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1268>).
fn build_url_filespec(doc: &PdfDocument, file: &str) -> Result<PdfObject, Error> {
    let mut spec = doc.new_dict_with_capacity(3)?;
    spec.dict_put("Type", PdfObject::new_name("Filespec")?)?;
    spec.dict_put("FS", PdfObject::new_name("URL")?)?;
    spec.dict_put("F", PdfObject::new_string(file)?)?;
    Ok(spec)
}

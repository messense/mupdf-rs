use super::{DestPageResolver, LinkAction, PdfAction, PdfLink};
use crate::pdf::{PdfDocument, PdfObject};
use crate::{Error, Matrix};

/// Builds a PDF link annotation dictionary from a [`PdfLink`]
/// (see [PDF 32000-1:2008, 12.5.6.5] for link annotations, [12.6.4] for action types).
///
/// This is the Rust analogue of MuPDF's [`pdf_create_link`] (annotation-dict building portion).
/// Like [`pdf_create_link`], it transforms the bounding rectangle from Fitz space to PDF user
/// space and assembles the annotation dictionary. It differs from [`pdf_create_link`] in that it:
///
/// - supports writing either `/A` (action) or `/Dest` (direct destination) via
///   [`set_link_action_on_annot_dict`], while MuPDF always writes `/A` via
///   [`pdf_new_action_from_link`],
/// - adds a `/P` back-reference to the parent page object (see below),
/// - returns the dict without inserting it into `/Annots` or creating an indirect object
///   (the caller handles those steps).
///
/// # Dictionary structure
///
/// | Entry      | Value                                                                |
/// |------------|----------------------------------------------------------------------|
/// | `Type`     | `/Annot`                                                             |
/// | `Subtype`  | `/Link`                                                              |
/// | `Rect`     | Link bounding box in PDF user space (transformed from Fitz)          |
/// | `BS`       | Border style dict: `{S: /S, Type: /Border, W: 0}` — invisible border |
/// | `A`/`Dest` | Action or destination (see [`set_link_action_on_annot_dict`])        |
/// | `P`        | Indirect reference to the parent page object                         |
///
/// where:
/// - `BS` is the Border Style dictionary ([PDF 32000-1:2008, 12.5.4], Table 166).
/// - `S = /S` selects the solid border style, `Type = /Border` types the sub-dictionary,
///   and `W = 0` sets zero width — producing an invisible border while the link region
///   remains active.
/// - `P` is the parent page back-reference ([PDF 32000-1:2008, 12.5.2], Table 164).
///
/// For action and destination entry shapes and per-variant MuPDF function mapping, see
/// [`set_link_action_on_annot_dict`].
///
/// [PDF 32000-1:2008, 12.5.6.5]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.1951136
/// [12.6.4]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.1697199
/// [`pdf_create_link`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-annot.c#L717
/// [`pdf_new_action_from_link`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1177
/// [PDF 32000-1:2008, 12.5.4]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.1696585
/// [PDF 32000-1:2008, 12.5.2]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.2292182
pub(crate) fn build_link_annotation(
    doc: &mut PdfDocument,
    page_obj: &PdfObject,
    link: &PdfLink,
    annot_inv_ctm: &Option<Matrix>,
    resolver: &mut impl DestPageResolver,
) -> Result<PdfObject, Error> {
    let rect = annot_inv_ctm
        .as_ref()
        .map(|inv_ctm| link.bounds.transform(inv_ctm))
        .unwrap_or(link.bounds);

    let mut annot = doc.new_dict_with_capacity(6)?;
    annot.dict_put("Type", PdfObject::new_name("Annot")?)?;
    annot.dict_put("Subtype", PdfObject::new_name("Link")?)?;

    let mut rect_array = doc.new_array_with_capacity(4)?;
    rect.encode_into(&mut rect_array)?;
    annot.dict_put("Rect", rect_array)?;

    let mut border_style = doc.new_dict_with_capacity(3)?;
    border_style.dict_put("S", PdfObject::new_name("S")?)?;
    border_style.dict_put("Type", PdfObject::new_name("Border")?)?;
    border_style.dict_put("W", PdfObject::new_int(0)?)?;
    annot.dict_put("BS", border_style)?;

    set_link_action_on_annot_dict(doc, &mut annot, &link.action, resolver)?;

    annot.dict_put_ref("P", page_obj)?;

    Ok(annot)
}

/// Writes the link target entry onto an annotation dictionary from a [`LinkAction`]
/// (see [PDF 32000-1:2008, 12.5.6.5], Table 173).
///
/// - [`LinkAction::Action`] -> delegates to [`set_action_on_annot_dict`] (writes `/A`).
/// - [`LinkAction::Dest`] -> encodes the destination and writes it as `/Dest` directly.
///
/// Unlike MuPDF's [`pdf_create_link`] and [`pdf_set_link_uri`], which always write `/A`
/// via [`pdf_new_action_from_link`], this function additionally supports writing `/Dest`
/// directly, matching PDF link annotations that carry a direct destination instead of an
/// action dictionary (see [PDF 32000-1:2008, 12.3.2]).
///
/// **Note:** Callers are responsible for removing the conflicting `/Dest` or `/A` entry before
/// calling this function when updating existing annotations.
///
/// [PDF 32000-1:2008, 12.5.6.5]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.1951136
/// [PDF 32000-1:2008, 12.3.2]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.2063217
/// [`pdf_create_link`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-annot.c#L717
/// [`pdf_set_link_uri`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L614
/// [`pdf_new_action_from_link`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1177
pub(crate) fn set_link_action_on_annot_dict(
    doc: &mut PdfDocument,
    annot: &mut PdfObject,
    action: &LinkAction,
    resolver: &mut impl DestPageResolver,
) -> Result<(), Error> {
    match action {
        LinkAction::Action(pdf_action) => {
            set_action_on_annot_dict(doc, annot, pdf_action, resolver)
        }
        LinkAction::Dest(dest) => annot.dict_put("Dest", dest.encode_local(doc, resolver)?),
    }
}

/// Builds and puts the `/A` action dictionary onto an annotation dictionary from a
/// [`PdfAction`] (see [PDF 32000-1:2008, 12.6.4]).
///
/// This is the Rust analogue of MuPDF's [`pdf_new_action_from_link`], which dispatches on
/// the URI scheme to build different action dictionary shapes. This function performs the
/// same dispatch directly on structured [`PdfAction`] values. Unlike MuPDF's [`pdf_new_action_from_link`]
/// this function upports [`PdfAction::Launch`].
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
/// For `GoTo(Page { .. })`, destination coordinates are transformed from MuPDF page space to
/// PDF default user space using the inverse CTM from the `resolver`. For `GoToR` coordinates
/// are passed as-is (already in PDF default user space).
///
/// **Note:** Callers are responsible for removing any conflicting `/Dest` entry before calling
/// this function when updating existing annotations.
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
///
/// [PDF 32000-1:2008, 12.6.4]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.1697199
/// [PDF 32000-1:2008, 12.6.4.5]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.1952224
/// [`pdf_new_action_from_link`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1177
/// [`pdf_new_dest_from_link`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1286
/// [`fz_is_external_link`]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/fitz/link.c#L68
fn set_action_on_annot_dict(
    doc: &mut PdfDocument,
    annot: &mut PdfObject,
    action: &PdfAction,
    resolver: &mut impl DestPageResolver,
) -> Result<(), Error> {
    match action {
        PdfAction::GoTo(dest) => {
            let dest_obj = dest.encode_local(doc, resolver)?;
            // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1191
            let mut action = doc.new_dict_with_capacity(2)?;
            action.dict_put("S", PdfObject::new_name("GoTo")?)?;
            action.dict_put("D", dest_obj)?;
            annot.dict_put("A", action)
        }
        PdfAction::Uri(uri) => {
            // MuPDF reads a URI action and stores the URI string as-is, since URI entries are ASCII strings
            // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L545
            // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1205
            let mut action = doc.new_dict_with_capacity(2)?;
            action.dict_put("S", PdfObject::new_name("URI")?)?;
            action.dict_put("URI", PdfObject::new_string(uri)?)?;
            annot.dict_put("A", action)
        }
        PdfAction::GoToR { file, dest } => {
            // MuPDF: GoToR action uses destination + filespec
            // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1197
            let mut action = doc.new_dict_with_capacity(3)?;
            action.dict_put("S", PdfObject::new_name("GoToR")?)?;
            let dest_obj = dest.encode_remote(doc)?;
            action.dict_put("D", dest_obj)?;

            // Same as MuPDF `pdf_add_filespec_from_link` function
            // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1152
            let file_spec = file.encode_into(doc)?;

            action.dict_put("F", file_spec)?;
            annot.dict_put("A", action)
        }
        PdfAction::Launch(file) => {
            // Same as MuPDF `pdf_add_filespec_from_link` function
            // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1152
            let file_spec = file.encode_into(doc)?;
            // No direct MuPDF code, see PDF 32000-1:2008, section 12.6.4.5, Table 203.
            // https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.1952224
            let mut action = doc.new_dict_with_capacity(2)?;
            action.dict_put("S", PdfObject::new_name("Launch")?)?;
            action.dict_put("F", file_spec)?;
            annot.dict_put("A", action)
        }
    }
}

use std::fmt;

use crate::pdf::PdfObject;
use crate::{Error, Matrix, Rect};

use mupdf_sys::*;

#[derive(Debug, Clone)]
pub struct Destination {
    /// Indirect reference to page object.
    page: PdfObject,
    kind: DestinationKind,
}

impl Destination {
    pub(crate) fn new(page: PdfObject, kind: DestinationKind) -> Self {
        Self { page, kind }
    }

    /// Encode this destination into a PDF "Dest" array.
    /// See [DestinationKind::encode_into] documentation for more information
    pub(crate) fn encode_into(self, array: &mut PdfObject) -> Result<(), Error> {
        debug_assert_eq!(array.len()?, 0);
        array.array_push(self.page)?;
        self.kind.encode_into(array)
    }
}

/// A MuPDF link destination view kind (backed by [`fz_link_dest`]).
///
/// This enum represents destinations produced by MuPDF across multiple document
/// handlers that support link destinations (PDF and some non-PDF formats).
///
/// # Format Dependence
///
/// Not all variants are produced for all document formats. For example, PDF documents
/// utilize the full range of variants (`Fit`, `FitH`, `XYZ`, etc.), while formats like
/// EPUB, HTML, or XPS typically emit only `XYZ`.
///
/// # Missing Values (`None` vs `NaN`)
///
/// MuPDF internally uses `NaN` to represent missing, unspecified, or "current" values.
/// This Rust API maps internal `NaN`s to `None`. The semantic meaning of `None` depends
/// on the format:
///
/// * **PDF:** `None` usually has a functional meaning (e.g., "preserve the current
///   zoom level" or "keep the current scroll position").
///
/// * **Non-PDF:** `None` means MuPDF did not specify the value (NaN).
///
/// # Manual Construction
///
/// If you construct or modify `DestinationKind` variants manually (e.g., for writing PDFs),
/// ensure that `Some(value)` does not contain `f32::NAN`. Use `None` to represent a missing value.
/// For example, in PDF context, `Some(f32::NAN)` is invalid and may result in malformed output,
/// as PDF does not support `NaN` as a real number.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DestinationKind {
    /// Display the page at a scale which just fits the whole page
    /// in the window both horizontally and vertically.
    Fit,

    /// Display the page with the vertical coordinate `top` at the top edge of the window,
    /// and the magnification set to fit the document horizontally.
    ///
    /// For PDF, `None` represents a missing (`null`) parameter (vertical position unchanged).
    FitH { top: Option<f32> },

    /// Display the page with the horizontal coordinate `left` at the left edge of the window,
    /// and the magnification set to fit the document vertically.
    ///
    /// For PDF, `None` represents a missing (`null`) parameter (horizontal position unchanged).
    FitV { left: Option<f32> },

    /// Display the page with (`left`, `top`) at the upper-left corner
    /// of the window and the page magnified by factor `zoom`.
    ///
    /// For PDF format:
    ///
    /// - `left`/`top`/`zoom` being `None` represents missing (`null`) parameters (unchanged).
    ///
    /// - `zoom` is specified as a percentage. For example, pass `Some(100.0)` for 100% zoom
    ///   (actual size), or `Some(50.0)` for 50%. Note that in [PDF 32000-1:2008, 12.3.2.2]
    ///   the `/XYZ` zoom value is a **scale factor**, not a percentage. If you convert this
    ///   value for PDF serialization, divide it by 100.0 (`zoom / 100.0`).
    ///
    /// [PDF 32000-1:2008, 12.3.2.2]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf
    XYZ {
        left: Option<f32>,
        top: Option<f32>,
        zoom: Option<f32>,
    },

    /// Display the page zoomed to show the rectangle specified by `left`, `bottom`, `right`, and `top`.
    ///
    /// For PDF, all four coordinates are required (`/FitR`).
    FitR {
        left: f32,
        bottom: f32,
        right: f32,
        top: f32,
    },

    /// Display the page like `/Fit`, but use the bounding box of the page’s contents,
    /// rather than the crop box.
    FitB,

    /// Display the page like `/FitH`, but use the bounding box of the page’s contents,
    /// rather than the crop box.
    ///
    /// For PDF, `None` represents a missing (`null`) parameter (vertical position unchanged).
    FitBH { top: Option<f32> },

    /// Display the page like `/FitV`, but use the bounding box of the page’s contents,
    /// rather than the crop box.
    ///
    /// For PDF, `None` represents a missing (`null`) parameter (horizontal position unchanged).
    FitBV { left: Option<f32> },
}

impl DestinationKind {
    /// Transforms destination coordinates using a matrix.
    ///
    /// This is primarily used to convert MuPDF-resolved destination coordinates
    /// (MuPDF user space) into PDF destination coordinates before serializing to
    /// a PDF Dest array.
    ///
    /// # Note on Non-PDF formats
    ///
    /// For non-PDF formats (HTML, XPS, etc.), coordinates are typically already in
    /// their target space. Applying a transform unless specifically intended may yield
    /// incorrect results.
    ///
    /// # PDF coordinate space
    ///
    /// In PDF, explicit destinations (`/XYZ`, `/FitH`, `/FitV`, `/FitR`, etc.) store all
    /// coordinates in the **default user space** of the destination page
    /// ([PDF 32000-1:2008, 12.3.2.2](https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf)).
    ///
    /// # MuPDF coordinate space
    ///
    /// For PDF documents, MuPDF exposes destination coordinates in MuPDF (fitz) page
    /// space, which may differ from PDF default user space.
    ///
    /// Therefore, `matrix` must convert from **current destination coords** into **PDF default user space**.
    ///
    /// # Choosing the matrix
    ///
    /// ## Local destinations (GoTo)
    ///
    /// For local destinations, MuPDF uses a page transformation matrix (called `ctm` in the
    /// MuPDF source) to convert PDF default user space into MuPDF page space
    /// (see [source](https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L96)).
    ///
    /// To convert destination coordinates back into **PDF default user space**, pass the
    /// inverse matrix (MuPDF [`invctm`](https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1328)):
    ///
    /// ```text
    /// let mu_to_pdf = page.page_ctm()?.invert();
    /// let pdf_dest = dest_kind.transform(&mu_to_pdf);
    /// ```
    ///
    /// ## Remote destinations (GoToR)
    ///
    /// For remote destinations, MuPDF uses coordinates already in **PDF default user space**
    /// (see [source](https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L77)),
    /// so no additional conversion is needed. You can pass `Matrix::IDENTITY`, matching MuPDF
    /// [behaviour](https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1320):
    ///
    /// ```text
    /// let pdf_dest = dest_kind.transform(&Matrix::IDENTITY);
    /// ```
    ///
    /// # Implementation Note
    ///
    /// Ported from [`pdf_new_dest_from_link`] in MuPDF (`pdf/pdf-link.c`).
    pub fn transform(self, matrix: &Matrix) -> Self {
        match self {
            Self::Fit => Self::Fit,
            Self::FitB => Self::FitB,

            // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1337
            // MuPDF: p = fz_transform_point_xy(0, val.y, invctm);
            // write NULL if isnan(p.y). Here we only do the transform, `null`
            // emission should be done in encode step.
            Self::FitH { top } => {
                let top = top.map(|t| matrix.transform_xy(0.0, t).1);
                Self::FitH { top }
            }

            // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1345
            Self::FitBH { top } => {
                let top = top.map(|t| matrix.transform_xy(0.0, t).1);
                Self::FitBH { top }
            }

            // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1353
            Self::FitV { left } => {
                let left = left.map(|l| matrix.transform_xy(l, 0.0).0);
                Self::FitV { left }
            }

            // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1361
            Self::FitBV { left } => {
                let left = left.map(|l| matrix.transform_xy(l, 0.0).0);
                Self::FitBV { left }
            }

            // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1369
            // MuPDF uses NaN to represent missing val.x/val.y.
            // For 90/270 degrees, missing X becomes missing Y after rotation and vice versa
            Self::XYZ { left, top, zoom } => {
                let (left, top) = if matrix.a == 0.0 && matrix.d == 0.0 {
                    // Rotating by 90 or 270 degrees
                    let (tx, ty) = matrix.transform_xy(left.unwrap_or(0.0), top.unwrap_or(0.0));

                    // MuPDF: if isnan(val.x) p.y = val.x / if isnan(val.y) p.x = val.y;
                    (top.and(Some(tx)), left.and(Some(ty)))
                } else if matrix.b == 0.0 && matrix.c == 0.0 {
                    // No rotation, or 180 degrees
                    let (tx, ty) = matrix.transform_xy(left.unwrap_or(0.0), top.unwrap_or(0.0));

                    // MuPDF: if isnan(val.x) p.x = val.x / if isnan(val.y) p.y = val.y;
                    (left.and(Some(tx)), top.and(Some(ty)))
                } else {
                    let (tx, ty) =
                        matrix.transform_xy(left.unwrap_or(f32::NAN), top.unwrap_or(f32::NAN));
                    (not_nan(tx), not_nan(ty))
                };

                Self::XYZ { left, top, zoom }
            }
            // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1404
            Self::FitR {
                left,
                bottom,
                right,
                top,
            } => {
                let r = Rect {
                    x0: left,
                    y0: bottom,
                    x1: right,
                    y1: top,
                };
                let tr = r.transform(matrix);
                Self::FitR {
                    left: tr.x0,
                    bottom: tr.y0,
                    right: tr.x1,
                    top: tr.y1,
                }
            }
        }
    }

    /// Encode this destination into a PDF "Dest" array.
    ///
    /// # MuPDF parity / Source
    ///
    /// Ported from MuPDF [`pdf_new_dest_from_link`] (pdf/pdf-link.c).
    ///
    /// In MuPDF logic:
    ///
    /// - optional parameters are represented internally by `NaN` (missing).
    /// - when serializing into a PDF destination array, MuPDF writes `null`
    ///   for `NaN` (missing) values.
    ///
    /// In this Rust crate:
    ///
    /// - **`Option::None`** represents a missing parameter.
    /// - **`Option::None`** is serialized as the PDF `null` object.
    ///
    /// Additionally for `/XYZ`:
    ///
    /// - MuPDF stores zoom internally as a percentage (100 == 100% zoom).
    /// - PDF `/XYZ` expects a scale factor (1.0 == 100%), so we write `zoom/100`.
    ///
    /// # Coordinate space
    ///
    /// This method does **not** apply any coordinate transforms.
    /// It expects `self` coordinates to already be in PDF user space for the *target page*.
    pub fn encode_into(self, array: &mut PdfObject) -> Result<(), Error> {
        #[cold]
        fn push_null(array: &mut PdfObject) -> Result<(), Error> {
            array.array_push(PdfObject::new_null())
        }

        #[inline]
        fn push_real_or_null(array: &mut PdfObject, v: Option<f32>) -> Result<(), Error> {
            match v {
                Some(v) => {
                    if !v.is_nan() {
                        array.array_push(PdfObject::new_real(v)?)
                    } else {
                        push_null(array) // move out from hot path
                    }
                }
                None => array.array_push(PdfObject::new_null()),
            }
        }

        match self {
            DestinationKind::Fit => array.array_push(PdfObject::new_name("Fit")?),

            DestinationKind::FitH { top } => {
                array.array_push(PdfObject::new_name("FitH")?)?;
                // MuPDF: if isnan(p.y) push NULL else real
                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1340
                push_real_or_null(array, top)
            }

            DestinationKind::FitV { left } => {
                array.array_push(PdfObject::new_name("FitV")?)?;
                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1356
                push_real_or_null(array, left)
            }

            DestinationKind::XYZ { left, top, zoom } => {
                array.array_push(PdfObject::new_name("XYZ")?)?;
                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1391
                push_real_or_null(array, left)?;
                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1395
                push_real_or_null(array, top)?;

                // MuPDF: stores zoom as percent (100 == 100%), but PDF wants scale (1.0 == 100%)
                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1399
                push_real_or_null(array, zoom.map(|z| z / 100.0))
            }

            DestinationKind::FitR {
                left,
                bottom,
                right,
                top,
            } => {
                array.array_push(PdfObject::new_name("FitR")?)?;
                // In PDF all 4 coordinates are required -> always real.
                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1411
                array.array_push(PdfObject::new_real(left)?)?;
                array.array_push(PdfObject::new_real(bottom)?)?;
                array.array_push(PdfObject::new_real(right)?)?;
                array.array_push(PdfObject::new_real(top)?)
            }

            DestinationKind::FitB => array.array_push(PdfObject::new_name("FitB")?),

            DestinationKind::FitBH { top } => {
                array.array_push(PdfObject::new_name("FitBH")?)?;
                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1348
                push_real_or_null(array, top)
            }

            DestinationKind::FitBV { left } => {
                array.array_push(PdfObject::new_name("FitBV")?)?;
                // https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/pdf/pdf-link.c#L1364
                push_real_or_null(array, left)
            }
        }
    }

    /// Decodes a PDF destination array into a `DestinationKind`.
    ///
    /// The `array` should be the full destination array `[page, /Name, params...]`.
    /// This reads from index 1 onward (skipping the page reference at index 0).
    /// Coordinates are returned as-is in PDF user space (no CTM applied).
    ///
    /// This is the inverse of [`encode_into`](Self::encode_into).
    pub fn decode_from(array: &PdfObject) -> Result<DestinationKind, Error> {
        let kind_obj = array
            .get_array(1)?
            .ok_or_else(|| Error::InvalidDestination("missing destination type name".into()))?;
        let kind_name = kind_obj.as_name()?;

        /// Read a float from an array index, returning None if the element is null/missing.
        #[inline(always)]
        fn read_optional_float(array: &PdfObject, idx: i32) -> Result<Option<f32>, Error> {
            match array.get_array(idx)? {
                Some(obj) => {
                    if obj.is_null()? {
                        Ok(None)
                    } else {
                        Ok(Some(obj.as_float()?))
                    }
                }
                None => Ok(None),
            }
        }

        #[inline(always)]
        fn read_float(array: &PdfObject, idx: i32) -> Result<f32, Error> {
            array
                .get_array(idx)?
                .ok_or_else(|| Error::InvalidDestination(format!("missing float at index {idx}")))?
                .as_float()
        }

        let destination_kind = match kind_name {
            b"Fit" => DestinationKind::Fit,
            b"FitB" => DestinationKind::FitB,
            b"FitH" => DestinationKind::FitH {
                top: read_optional_float(array, 2)?,
            },
            b"FitBH" => DestinationKind::FitBH {
                top: read_optional_float(array, 2)?,
            },
            b"FitV" => DestinationKind::FitV {
                left: read_optional_float(array, 2)?,
            },
            b"FitBV" => DestinationKind::FitBV {
                left: read_optional_float(array, 2)?,
            },
            b"FitR" => DestinationKind::FitR {
                left: read_float(array, 2)?,
                bottom: read_float(array, 3)?,
                right: read_float(array, 4)?,
                top: read_float(array, 5)?,
            },
            _ => DestinationKind::XYZ {
                left: read_optional_float(array, 2)?,
                top: read_optional_float(array, 3)?,
                // PDF /XYZ stores zoom as a scale factor (1.0 = 100%).
                // DestinationKind stores it as a percentage (100.0 = 100%).
                zoom: read_optional_float(array, 4)?
                    .map(|z| if z > 0.0 { z * 100.0 } else { 100.0 }),
            },
        };
        Ok(destination_kind)
    }
}

impl Default for DestinationKind {
    fn default() -> Self {
        // This analogue of MuPDF's `fz_make_link_dest_none` function
        // (https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/source/fitz/link.c#L96)
        DestinationKind::XYZ {
            left: None,
            top: None,
            zoom: None,
        }
    }
}

impl fmt::Display for DestinationKind {
    /// Formats this destination as a [MuPDF-compatible] URI fragment suffix based on the Adobe specification
    /// ["Parameters for Opening PDF Files"] from the Adobe Acrobat SDK, version 8.1.
    ///
    /// This is only the parameter tail appended after `#page=...` in a link URI, so any emitted suffix starts
    /// with `&`. The output may also be empty (e.g. for the default `XYZ` with all fields missing).
    ///
    /// Output shapes:
    ///
    /// - `Fit` / `FitB` -> `&view=Fit` / `&view=FitB`
    /// - `FitH` / `FitV` / `FitBH` / `FitBV` -> `&view=Name` or `&view=Name,<coord>`
    /// - `XYZ` -> `&zoom=<zoom>,<left>,<top>`
    /// - `FitR` -> `&viewrect=<left>,<bottom>,<width>,<height>`
    ///
    /// Missing values:
    ///
    /// - `None` means “unspecified” (`null` as per [PDF 32000-1:2008, 12.3.2.2]). `Some(NaN)` is treated as `None`.
    /// - For `Fit*`, a missing coordinate is omitted.
    /// - For `XYZ`, the fragment is omitted when all fields are missing, otherwise missing fields are written as `nan`.
    /// - For `XYZ`, `zoom == 0` is treated as missing.
    ///
    /// Values are written as-is (no transforms applied).
    ///
    /// [MuPDF-compatible]: https://github.com/ArtifexSoftware/mupdf/blob/60bf95d09f496ab67a5e4ea872bdd37a74b745fe/include/mupdf/pdf/annot.h#L317
    /// [PDF 32000-1:2008, 12.3.2.2]: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf#G11.1696125
    /// ["Parameters for Opening PDF Files"]: https://web.archive.org/web/20170921000830/http://www.adobe.com/content/dam/Adobe/en/devnet/acrobat/pdfs/pdf_open_parameters.pdf
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cold]
        #[inline(never)]
        fn cold_nan_handler() {}

        #[inline(always)]
        fn filter_nan(v: Option<f32>) -> Option<f32> {
            match v {
                Some(val) => {
                    if !val.is_nan() {
                        Some(val)
                    } else {
                        cold_nan_handler(); // move out from hot path
                        None
                    }
                }
                None => None,
            }
        }

        match *self {
            Self::Fit => f.write_str("&view=Fit"),
            Self::FitB => f.write_str("&view=FitB"),
            Self::FitH { top } => write_fit_view(f, "FitH", filter_nan(top)),
            Self::FitBH { top } => write_fit_view(f, "FitBH", filter_nan(top)),
            Self::FitV { left } => write_fit_view(f, "FitV", filter_nan(left)),
            Self::FitBV { left } => write_fit_view(f, "FitBV", filter_nan(left)),
            Self::XYZ { left, top, zoom } => {
                let x = filter_nan(left);
                let y = filter_nan(top);
                let z = filter_nan(zoom).filter(|&v| v != 0.0);

                if x.is_some() || y.is_some() || z.is_some() {
                    f.write_str("&zoom=")?;
                    write_f32_or_nan(f, z)?;
                    f.write_str(",")?;
                    write_f32_or_nan(f, x)?;
                    f.write_str(",")?;
                    write_f32_or_nan(f, y)?;
                }
                Ok(())
            }
            Self::FitR {
                left,
                bottom,
                right,
                top,
            } => {
                let w = right - left;
                let h = top - bottom;
                write!(f, "&viewrect={left},{bottom},{w},{h}")
            }
        }
    }
}

/// Writes `&view=Name,val` or `&view=Name` depending on whether `val` is present.
fn write_fit_view(f: &mut fmt::Formatter<'_>, name: &str, val: Option<f32>) -> fmt::Result {
    match val {
        Some(v) => write!(f, "&view={name},{v}"),
        None => write!(f, "&view={name}"),
    }
}

fn write_f32_or_nan(f: &mut fmt::Formatter<'_>, v: Option<f32>) -> fmt::Result {
    match v {
        Some(v) => write!(f, "{v}"),
        None => f.write_str("nan"),
    }
}

impl From<fz_link_dest> for DestinationKind {
    #[allow(non_upper_case_globals)]
    fn from(value: fz_link_dest) -> Self {
        match value.type_ {
            FZ_LINK_DEST_FIT => Self::Fit,
            FZ_LINK_DEST_FIT_B => Self::FitB,
            FZ_LINK_DEST_FIT_H => Self::FitH {
                top: not_nan(value.y),
            },
            FZ_LINK_DEST_FIT_BH => Self::FitBH {
                top: not_nan(value.y),
            },
            FZ_LINK_DEST_FIT_V => Self::FitV {
                left: not_nan(value.x),
            },
            FZ_LINK_DEST_FIT_BV => Self::FitBV {
                left: not_nan(value.x),
            },
            FZ_LINK_DEST_XYZ => Self::XYZ {
                left: not_nan(value.x),
                top: not_nan(value.y),
                zoom: not_nan(value.zoom),
            },
            FZ_LINK_DEST_FIT_R => Self::FitR {
                left: value.x,
                bottom: value.y,
                right: value.x + value.w,
                top: value.y + value.h,
            },
            _ => unreachable!(),
        }
    }
}

#[inline]
pub(crate) fn not_nan(val: f32) -> Option<f32> {
    if val.is_nan() {
        None
    } else {
        Some(val)
    }
}

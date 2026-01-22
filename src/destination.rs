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
    ///
    /// # MuPDF parity / Source
    /// Ported from MuPDF `pdf_new_dest_from_link(...)` (pdf/pdf-link.c).
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
    /// It expects `self.kind` coordinates to already be in PDF user space for the *target page*.
    pub fn encode_into(self, array: &mut PdfObject) -> Result<(), Error> {
        debug_assert_eq!(array.len()?, 0);

        #[inline]
        fn push_real_or_null(array: &mut PdfObject, v: Option<f32>) -> Result<(), Error> {
            match v {
                Some(v) => array.array_push(PdfObject::new_real(v)?),
                None => array.array_push(PdfObject::new_null()),
            }
        }

        // 1) Page reference (local destination)
        array.array_push(self.page)?;

        // 2) Kind
        match self.kind {
            DestinationKind::Fit => array.array_push(PdfObject::new_name("Fit")?),

            DestinationKind::FitB => array.array_push(PdfObject::new_name("FitB")?),

            DestinationKind::FitH { top } => {
                array.array_push(PdfObject::new_name("FitH")?)?;
                // MuPDF: if isnan(p.y) push NULL else real (pdf/pdf-link.c: 1340)
                push_real_or_null(array, top)
            }

            DestinationKind::FitBH { top } => {
                array.array_push(PdfObject::new_name("FitBH")?)?;
                // MuPDF: (pdf/pdf-link.c: 1348)
                push_real_or_null(array, top)
            }

            DestinationKind::FitV { left } => {
                array.array_push(PdfObject::new_name("FitV")?)?;
                // MuPDF: (pdf/pdf-link.c: 1356)
                push_real_or_null(array, left)
            }

            DestinationKind::FitBV { left } => {
                array.array_push(PdfObject::new_name("FitBV")?)?;
                // MuPDF: (pdf/pdf-link.c: 1364)
                push_real_or_null(array, left)
            }

            DestinationKind::XYZ { left, top, zoom } => {
                array.array_push(PdfObject::new_name("XYZ")?)?;
                // MuPDF: (pdf/pdf-link.c: 1391)
                push_real_or_null(array, left)?;
                // MuPDF: (pdf/pdf-link.c: 1395)
                push_real_or_null(array, top)?;

                // MuPDF: (pdf/pdf-link.c: 1399) stores zoom as percent (100 == 100%),
                // but PDF wants scale (1.0 == 100%)
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
                // (pdf/pdf-link.c: 1411..1414)
                array.array_push(PdfObject::new_real(left)?)?;
                array.array_push(PdfObject::new_real(bottom)?)?;
                array.array_push(PdfObject::new_real(right)?)?;
                array.array_push(PdfObject::new_real(top)?)
            }
        }
    }
}

/// A MuPDF link destination view kind (backed by `fz_link_dest`).
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
    /// - `left`/`top`/`zoom` being `None` represents missing (`null`) parameters (unchanged).
    /// - MuPDF stores `zoom` as percent (100.0 == 100%), while PDF expects scale (1.0 == 100%).
    ///   Serialization should handle this conversion.
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
    /// (MuPDF page/user space) into PDF destination coordinates (page space)
    /// before serializing to a PDF Dest array.
    ///
    /// # Note on Non-PDF formats
    ///
    /// For non-PDF formats (HTML, XPS, etc.), coordinates are typically already in
    /// their target space. Applying a transform unless specifically intended may yield
    /// incorrect results.
    ///
    /// # Matrix Parameter
    ///
    /// The `matrix` parameter should typically be the **inverse** of the page CTM.
    ///
    /// ```text
    /// let inv_ctm = page.page_ctm()?.invert();
    /// let transformed = dest_kind.transform(&inv_ctm);
    /// ```
    /// # Implementation Note
    ///
    /// Ported from `pdf_new_dest_from_link` in MuPDF (`pdf/pdf-link.c`).
    pub fn transform(self, matrix: &Matrix) -> Self {
        // Helper function, similar to `fz_transform_point_xy` (fitz/geometry.c: 344)
        // from C. Performs bare math without checking for NaN.
        #[inline(always)]
        fn transform_xy(m: &Matrix, x: f32, y: f32) -> (f32, f32) {
            (x * m.a + y * m.c + m.e, x * m.b + y * m.d + m.f)
        }

        match self {
            Self::Fit => Self::Fit,
            Self::FitB => Self::FitB,

            // MuPDF: (pdf/pdf-link.c: 1337)
            //     p = fz_transform_point_xy(0, val.y, invctm);
            // write NULL if isnan(p.y). Here we only do the transform, `null`
            // emission should be done in encode step.
            Self::FitH { top } => {
                let top = top.map(|t| transform_xy(matrix, 0.0, t).1);
                Self::FitH { top }
            }

            // MuPDF: (pdf/pdf-link.c: 1345)
            Self::FitBH { top } => {
                let top = top.map(|t| transform_xy(matrix, 0.0, t).1);
                Self::FitBH { top }
            }

            // MuPDF: (pdf/pdf-link.c: 1353)
            Self::FitV { left } => {
                let left = left.map(|l| transform_xy(matrix, l, 0.0).0);
                Self::FitV { left }
            }

            // MuPDF: (pdf/pdf-link.c: 1361)
            Self::FitBV { left } => {
                let left = left.map(|l| transform_xy(matrix, l, 0.0).0);
                Self::FitBV { left }
            }

            // MuPDF: (pdf/pdf-link.c: 1369)
            // MuPDF uses NaN to represent missing val.x/val.y.
            // For 90/270 degrees, missing X becomes missing Y after rotation and vice versa
            Self::XYZ { left, top, zoom } => {
                let (left, top) = if matrix.a == 0.0 && matrix.d == 0.0 {
                    // Rotating by 90 or 270 degrees
                    let (tx, ty) = transform_xy(matrix, left.unwrap_or(0.0), top.unwrap_or(0.0));

                    // MuPDF: if isnan(val.x) p.y = val.x / if isnan(val.y) p.x = val.y;
                    (top.and(Some(tx)), left.and(Some(ty)))
                } else if matrix.b == 0.0 && matrix.c == 0.0 {
                    // No rotation, or 180 degrees
                    let (tx, ty) = transform_xy(matrix, left.unwrap_or(0.0), top.unwrap_or(0.0));

                    // MuPDF: if isnan(val.x) p.x = val.x / if isnan(val.y) p.y = val.y;
                    (left.and(Some(tx)), top.and(Some(ty)))
                } else {
                    let (tx, ty) =
                        transform_xy(matrix, left.unwrap_or(f32::NAN), top.unwrap_or(f32::NAN));
                    (not_nan(tx), not_nan(ty))
                };

                Self::XYZ { left, top, zoom }
            }
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
pub fn not_nan(val: f32) -> Option<f32> {
    if val.is_nan() {
        None
    } else {
        Some(val)
    }
}

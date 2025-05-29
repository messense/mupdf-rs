use crate::pdf::PdfObject;
use crate::{Error, Matrix, Point, Rect};

use mupdf_sys::*;

#[derive(Debug, Clone)]
pub struct Destination {
    /// Indirect reference to page object.
    page: PdfObject,
    kind: DestinationKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DestinationKind {
    /// Display the page at a scale which just fits the whole page
    /// in the window both horizontally and vertically.
    Fit,
    /// Display the page with the vertical coordinate `top` at the top edge of the window,
    /// and the magnification set to fit the document horizontally.
    FitH { top: f32 },
    /// Display the page with the horizontal coordinate `left` at the left edge of the window,
    /// and the magnification set to fit the document vertically.
    FitV { left: f32 },
    /// Display the page with (`left`, `top`) at the upper-left corner
    /// of the window and the page magnified by factor `zoom`.
    XYZ {
        left: Option<f32>,
        top: Option<f32>,
        zoom: Option<f32>,
    },
    /// Display the page zoomed to show the rectangle specified by `left`, `bottom`, `right`, and `top`.
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
    FitBH { top: f32 },
    /// Display the page like `/FitV`, but use the bounding box of the page’s contents,
    /// rather than the crop box.
    FitBV { left: f32 },
}

impl DestinationKind {
    #[allow(non_upper_case_globals)]
    pub(crate) fn from_link_dest(dst: fz_link_dest) -> Self {
        match dst.type_ {
            fz_link_dest_type_FZ_LINK_DEST_FIT => Self::Fit,
            fz_link_dest_type_FZ_LINK_DEST_FIT_B => Self::FitB,
            fz_link_dest_type_FZ_LINK_DEST_FIT_H => Self::FitH { top: dst.y },
            fz_link_dest_type_FZ_LINK_DEST_FIT_BH => Self::FitBH { top: dst.y },
            fz_link_dest_type_FZ_LINK_DEST_FIT_V => Self::FitV { left: dst.x },
            fz_link_dest_type_FZ_LINK_DEST_FIT_BV => Self::FitBV { left: dst.x },
            fz_link_dest_type_FZ_LINK_DEST_XYZ => Self::XYZ {
                left: Some(dst.x),
                top: Some(dst.y),
                zoom: Some(dst.zoom),
            },
            fz_link_dest_type_FZ_LINK_DEST_FIT_R => Self::FitR {
                left: dst.x,
                bottom: dst.y,
                right: dst.x + dst.w,
                top: dst.y + dst.h,
            },
            _ => unreachable!(),
        }
    }

    pub fn transform(self, matrix: &Matrix) -> Self {
        match self {
            Self::Fit => Self::Fit,
            Self::FitB => Self::FitB,
            Self::FitH { top } => {
                let p = Point::new(0.0, top).transform(matrix);
                Self::FitH { top: p.y }
            }
            Self::FitBH { top } => {
                let p = Point::new(0.0, top).transform(matrix);
                Self::FitBH { top: p.y }
            }
            Self::FitV { left } => {
                let p = Point::new(left, 0.0).transform(matrix);
                Self::FitV { left: p.x }
            }
            Self::FitBV { left } => {
                let p = Point::new(left, 0.0).transform(matrix);
                Self::FitBV { left: p.x }
            }
            Self::XYZ { left, top, zoom } => {
                let p =
                    Point::new(left.unwrap_or_default(), top.unwrap_or_default()).transform(matrix);
                Self::XYZ {
                    left: Some(p.x),
                    top: Some(p.y),
                    zoom,
                }
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

impl Destination {
    pub(crate) fn new(page: PdfObject, kind: DestinationKind) -> Self {
        Self { page, kind }
    }

    /// Encode destination into a PDF array.
    pub(crate) fn encode_into(self, array: &mut PdfObject) -> Result<(), Error> {
        debug_assert_eq!(array.len()?, 0);

        array.array_push(self.page)?;
        match self.kind {
            DestinationKind::Fit => array.array_push(PdfObject::new_name("Fit")?)?,
            DestinationKind::FitH { top } => {
                array.array_push(PdfObject::new_name("FitH")?)?;
                array.array_push(PdfObject::new_real(top)?)?;
            }
            DestinationKind::FitV { left } => {
                array.array_push(PdfObject::new_name("FitV")?)?;
                array.array_push(PdfObject::new_real(left)?)?;
            }
            DestinationKind::XYZ { left, top, zoom } => {
                array.array_push(PdfObject::new_name("XYZ")?)?;
                array.array_push(
                    left.map(PdfObject::new_real)
                        .transpose()?
                        .unwrap_or(PdfObject::new_null()),
                )?;
                array.array_push(
                    top.map(PdfObject::new_real)
                        .transpose()?
                        .unwrap_or(PdfObject::new_null()),
                )?;
                array.array_push(
                    zoom.map(PdfObject::new_real)
                        .transpose()?
                        .unwrap_or(PdfObject::new_null()),
                )?;
            }
            DestinationKind::FitR {
                left,
                bottom,
                right,
                top,
            } => {
                array.array_push(PdfObject::new_name("FitR")?)?;
                array.array_push(PdfObject::new_real(left)?)?;
                array.array_push(PdfObject::new_real(bottom)?)?;
                array.array_push(PdfObject::new_real(right)?)?;
                array.array_push(PdfObject::new_real(top)?)?;
            }
            DestinationKind::FitB => array.array_push(PdfObject::new_name("FitB")?)?,
            DestinationKind::FitBH { top } => {
                array.array_push(PdfObject::new_name("FitBH")?)?;
                array.array_push(PdfObject::new_real(top)?)?;
            }
            DestinationKind::FitBV { left } => {
                array.array_push(PdfObject::new_name("FitBV")?)?;
                array.array_push(PdfObject::new_real(left)?)?;
            }
        }

        Ok(())
    }
}

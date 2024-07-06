use crate::pdf::PdfObject;
use crate::Error;

#[derive(Debug, Clone)]
pub struct Destination {
    /// Indirect reference to page object.
    pub page: PdfObject,
    pub kind: DestinationKind,
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
    XYZ { left: f32, top: f32, zoom: f32 },
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

impl Destination {
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
                array.array_push(PdfObject::new_real(left)?)?;
                array.array_push(PdfObject::new_real(top)?)?;
                array.array_push(PdfObject::new_real(zoom)?)?;
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

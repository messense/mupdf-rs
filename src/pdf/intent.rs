use mupdf_sys::*;

use crate::from_enum;

from_enum! { pdf_intent => pdf_intent,
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Intent {
        Default = PDF_ANNOT_IT_DEFAULT,
        FreetextCallout = PDF_ANNOT_IT_FREETEXT_CALLOUT,
        FreetextTypewriter = PDF_ANNOT_IT_FREETEXT_TYPEWRITER,
        LineArrow = PDF_ANNOT_IT_LINE_ARROW,
        LineDimension = PDF_ANNOT_IT_LINE_DIMENSION,
        PolylineDimension = PDF_ANNOT_IT_POLYLINE_DIMENSION,
        PolygonCloud = PDF_ANNOT_IT_POLYGON_CLOUD,
        PolygonDimension = PDF_ANNOT_IT_POLYGON_DIMENSION,
        StampImage = PDF_ANNOT_IT_STAMP_IMAGE,
        StampSnapshot = PDF_ANNOT_IT_STAMP_SNAPSHOT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intents_convert() {
        let intents = [
            PDF_ANNOT_IT_DEFAULT,
            PDF_ANNOT_IT_FREETEXT_CALLOUT,
            PDF_ANNOT_IT_FREETEXT_TYPEWRITER,
            PDF_ANNOT_IT_LINE_ARROW,
            PDF_ANNOT_IT_LINE_DIMENSION,
            PDF_ANNOT_IT_POLYLINE_DIMENSION,
            PDF_ANNOT_IT_POLYGON_CLOUD,
            PDF_ANNOT_IT_POLYGON_DIMENSION,
            PDF_ANNOT_IT_STAMP_IMAGE,
            PDF_ANNOT_IT_STAMP_SNAPSHOT,
        ];

        for i in intents {
            let converted = Intent::try_from(i).unwrap();
            let back = pdf_intent::from(converted);

            assert_eq!(back, i);
        }
    }
}

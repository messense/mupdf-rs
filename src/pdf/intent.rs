use mupdf_sys::*;

pub enum Intent {
    Default,
    FreetextCallout,
    FreetextTypewriter,
    LineArrow,
    LineDimension,
    PolylineDimension,
    PolygonCloud,
    PolygonDimension,
    StampImage,
    StampSnapshot,
}

impl From<Intent> for pdf_intent {
    fn from(value: Intent) -> Self {
        match value {
            Intent::Default => PDF_ANNOT_IT_DEFAULT,
            Intent::FreetextCallout => PDF_ANNOT_IT_FREETEXT_CALLOUT,
            Intent::FreetextTypewriter => PDF_ANNOT_IT_FREETEXT_TYPEWRITER,
            Intent::LineArrow => PDF_ANNOT_IT_LINE_ARROW,
            Intent::LineDimension => PDF_ANNOT_IT_LINE_DIMENSION,
            Intent::PolylineDimension => PDF_ANNOT_IT_POLYLINE_DIMENSION,
            Intent::PolygonCloud => PDF_ANNOT_IT_POLYGON_CLOUD,
            Intent::PolygonDimension => PDF_ANNOT_IT_POLYGON_DIMENSION,
            Intent::StampImage => PDF_ANNOT_IT_STAMP_IMAGE,
            Intent::StampSnapshot => PDF_ANNOT_IT_STAMP_SNAPSHOT
        }
    }
}

#[derive(Debug)]
pub struct OutOfRange;

impl std::fmt::Display for OutOfRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "The provided value was out of the allowed range")
    }
}

impl std::error::Error for OutOfRange {}

impl TryFrom<pdf_intent> for Intent {
    type Error = OutOfRange;

    fn try_from(value: pdf_intent) -> Result<Self, Self::Error> {
        match value {
            PDF_ANNOT_IT_DEFAULT => Ok(Self::Default),
            PDF_ANNOT_IT_FREETEXT_CALLOUT => Ok(Self::FreetextCallout),
            PDF_ANNOT_IT_FREETEXT_TYPEWRITER => Ok(Self::FreetextTypewriter),
            PDF_ANNOT_IT_LINE_ARROW => Ok(Self::LineArrow),
            PDF_ANNOT_IT_LINE_DIMENSION => Ok(Self::LineDimension),
            PDF_ANNOT_IT_POLYLINE_DIMENSION => Ok(Self::PolylineDimension),
            PDF_ANNOT_IT_POLYGON_CLOUD => Ok(Self::PolygonCloud),
            PDF_ANNOT_IT_POLYGON_DIMENSION => Ok(Self::PolygonDimension),
            PDF_ANNOT_IT_STAMP_IMAGE => Ok(Self::StampImage),
            PDF_ANNOT_IT_STAMP_SNAPSHOT => Ok(Self::StampSnapshot),
            _ => Err(OutOfRange)
        }
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

#![forbid(unsafe_code)]

// These are MuPDF's modified/subset CFF resources derived from Charis SIL. The
// OFL license reserves the original font names, so expose the runtime family
// under a non-reserved name.
const FAMILY_NAME: &str = "MuPDF Serif";

#[derive(Clone, Copy, Debug)]
pub struct Font {
    pub name: &'static str,
    pub data: &'static [u8],
    pub index: i32,
}

const MUPDF_SERIF: Font = Font {
    name: FAMILY_NAME,
    data: include_bytes!("../fonts/CharisSIL.cff"),
    index: 0,
};

const MUPDF_SERIF_BOLD: Font = Font {
    name: FAMILY_NAME,
    data: include_bytes!("../fonts/CharisSIL-Bold.cff"),
    index: 0,
};

const MUPDF_SERIF_ITALIC: Font = Font {
    name: FAMILY_NAME,
    data: include_bytes!("../fonts/CharisSIL-Italic.cff"),
    index: 0,
};

const MUPDF_SERIF_BOLD_ITALIC: Font = Font {
    name: FAMILY_NAME,
    data: include_bytes!("../fonts/CharisSIL-BoldItalic.cff"),
    index: 0,
};

pub fn find_by_name(name: &str, bold: bool, italic: bool) -> Option<Font> {
    if !eq_font_name(name, FAMILY_NAME) && !eq_font_name(name, "MuPDFSerif") {
        return None;
    }

    match (bold, italic) {
        (false, false) => Some(MUPDF_SERIF),
        (true, false) => Some(MUPDF_SERIF_BOLD),
        (false, true) => Some(MUPDF_SERIF_ITALIC),
        (true, true) => Some(MUPDF_SERIF_BOLD_ITALIC),
    }
}

fn eq_font_name(a: &str, b: &str) -> bool {
    let mut a = normalized_bytes(a);
    let mut b = normalized_bytes(b);

    loop {
        match (a.next(), b.next()) {
            (None, None) => return true,
            (Some(x), Some(y)) if x == y => {}
            _ => return false,
        }
    }
}

fn normalized_bytes(s: &str) -> impl Iterator<Item = u8> + '_ {
    s.bytes()
        .filter(|b| b.is_ascii_alphanumeric())
        .map(|b| b.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_font_payload(font: Font) {
        assert!(
            font.data.len() > 1024,
            "{} payload is suspiciously small: {} bytes",
            font.name,
            font.data.len()
        );
        assert!(
            font.data.starts_with(b"OTTO")
                || font.data.starts_with(b"\0\x01\0\0")
                || font.data.starts_with(b"ttcf")
                || font.data.starts_with(&[1, 0]),
            "{} payload does not look like font data",
            font.name
        );
    }

    #[test]
    fn finds_mupdf_serif_by_family_name() {
        assert_eq!(
            find_by_name("MuPDF Serif", false, false).unwrap().data,
            MUPDF_SERIF.data
        );
        assert_eq!(
            find_by_name("MuPDFSerif", true, true).unwrap().data,
            MUPDF_SERIF_BOLD_ITALIC.data
        );
        assert!(find_by_name("Charis SIL", false, false).is_none());
        assert!(find_by_name("Times", false, false).is_none());
    }

    #[test]
    fn bundled_font_payloads_are_real_font_bytes() {
        assert_font_payload(MUPDF_SERIF);
        assert_font_payload(MUPDF_SERIF_BOLD);
        assert_font_payload(MUPDF_SERIF_ITALIC);
        assert_font_payload(MUPDF_SERIF_BOLD_ITALIC);
    }
}

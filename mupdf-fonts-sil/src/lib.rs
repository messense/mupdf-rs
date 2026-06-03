#![forbid(unsafe_code)]

#[derive(Clone, Copy, Debug)]
pub struct Font {
    pub name: &'static str,
    pub data: &'static [u8],
    pub index: i32,
}

const CHARIS_SIL: Font = Font {
    name: "Charis SIL",
    data: include_bytes!("../fonts/CharisSIL.cff"),
    index: 0,
};

const CHARIS_SIL_BOLD: Font = Font {
    name: "Charis SIL",
    data: include_bytes!("../fonts/CharisSIL-Bold.cff"),
    index: 0,
};

const CHARIS_SIL_ITALIC: Font = Font {
    name: "Charis SIL",
    data: include_bytes!("../fonts/CharisSIL-Italic.cff"),
    index: 0,
};

const CHARIS_SIL_BOLD_ITALIC: Font = Font {
    name: "Charis SIL",
    data: include_bytes!("../fonts/CharisSIL-BoldItalic.cff"),
    index: 0,
};

pub fn find_by_name(name: &str, bold: bool, italic: bool) -> Option<Font> {
    if !eq_font_name(name, "Charis SIL") && !eq_font_name(name, "CharisSIL") {
        return None;
    }

    match (bold, italic) {
        (false, false) => Some(CHARIS_SIL),
        (true, false) => Some(CHARIS_SIL_BOLD),
        (false, true) => Some(CHARIS_SIL_ITALIC),
        (true, true) => Some(CHARIS_SIL_BOLD_ITALIC),
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

    #[test]
    fn finds_charis_sil_by_family_name() {
        assert_eq!(
            find_by_name("Charis SIL", false, false).unwrap().data,
            CHARIS_SIL.data
        );
        assert_eq!(
            find_by_name("CharisSIL", true, true).unwrap().data,
            CHARIS_SIL_BOLD_ITALIC.data
        );
        assert!(find_by_name("Times", false, false).is_none());
    }
}

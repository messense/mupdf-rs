#![forbid(unsafe_code)]

#[derive(Clone, Copy, Debug)]
pub struct Font {
    pub name: &'static str,
    pub data: &'static [u8],
    pub index: i32,
}

const DROID_SANS_FALLBACK: Font = Font {
    name: "Droid Sans Fallback",
    data: include_bytes!("../fonts/DroidSansFallback.ttf"),
    index: 0,
};

const DROID_SANS_FALLBACK_FULL: Font = Font {
    name: "Droid Sans Fallback Full",
    data: include_bytes!("../fonts/DroidSansFallbackFull.ttf"),
    index: 0,
};

pub fn find_by_name(name: &str, bold: bool, italic: bool) -> Option<Font> {
    if bold || italic {
        return None;
    }

    if eq_font_name(name, DROID_SANS_FALLBACK_FULL.name)
        || eq_font_name(name, "DroidSansFallbackFull")
    {
        Some(DROID_SANS_FALLBACK_FULL)
    } else if eq_font_name(name, DROID_SANS_FALLBACK.name)
        || eq_font_name(name, "DroidSansFallback")
    {
        Some(DROID_SANS_FALLBACK)
    } else {
        None
    }
}

pub fn cjk_font(_ordering: i32, _serif: bool) -> Option<Font> {
    Some(DROID_SANS_FALLBACK_FULL)
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
    fn finds_droid_fonts_by_family_name() {
        assert_eq!(
            find_by_name("Droid Sans Fallback", false, false)
                .unwrap()
                .name,
            "Droid Sans Fallback"
        );
        assert_eq!(
            find_by_name("DroidSansFallbackFull", false, false)
                .unwrap()
                .name,
            "Droid Sans Fallback Full"
        );
        assert!(find_by_name("Droid Sans Fallback", true, false).is_none());
    }

    #[test]
    fn bundled_font_payloads_are_real_font_bytes() {
        assert_font_payload(DROID_SANS_FALLBACK);
        assert_font_payload(DROID_SANS_FALLBACK_FULL);
    }
}

#[cfg(feature = "bundled-fonts-noto")]
use std::ffi::CStr;
use std::os::raw::c_int;
use std::ptr;

use mupdf_sys::*;

use crate::font_loader::{FontHints, FontLoader};
use crate::{context, CjkFontOrdering, Font};

/// Serves the statically bundled font crates (`mupdf-fonts-*`) through the
/// [`FontLoader`] chain. Font bytes have static storage duration and are
/// handed to MuPDF without copying.
pub(crate) struct BundledFontLoader;

impl FontLoader for BundledFontLoader {
    fn load_font(&self, name: &str, hints: FontHints) -> Option<Font> {
        find_by_name(name, hints.bold, hints.italic, hints.needs_exact_metrics).and_then(load_font)
    }

    fn load_cjk_font(&self, _name: &str, ordering: CjkFontOrdering, serif: bool) -> Option<Font> {
        find_cjk_font(ordering as c_int, serif).and_then(load_font)
    }

    fn load_fallback_font(&self, script: u32, language: u32, hints: FontHints) -> Option<Font> {
        find_fallback_font(script as c_int, language as c_int, hints.serif).and_then(load_font)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct FontData {
    name: &'static str,
    data: &'static [u8],
    index: i32,
}

pub(crate) fn find_by_name(
    name: &str,
    bold: bool,
    italic: bool,
    needs_exact_metrics: bool,
) -> Option<FontData> {
    #[cfg(feature = "bundled-fonts-sil")]
    if let Some(font) = mupdf_fonts_sil::find_by_name(name, bold, italic) {
        return Some(FontData {
            name: font.name,
            data: font.data,
            index: font.index,
        });
    }

    #[cfg(feature = "bundled-fonts-noto")]
    {
        let (bold, italic) = if needs_exact_metrics {
            (bold, italic)
        } else {
            (false, false)
        };

        if let Some(font) = mupdf_fonts_noto::find_by_name(name, bold, italic) {
            return Some(FontData {
                name: font.name,
                data: font.data,
                index: font.index,
            });
        }
    }

    #[cfg(feature = "bundled-fonts-droid")]
    {
        let (bold, italic) = if needs_exact_metrics {
            (bold, italic)
        } else {
            (false, false)
        };

        if let Some(font) = mupdf_fonts_droid::find_by_name(name, bold, italic) {
            return Some(FontData {
                name: font.name,
                data: font.data,
                index: font.index,
            });
        }
    }

    let _ = (bold, italic, needs_exact_metrics);
    None
}

fn find_cjk_font(ordering: c_int, serif: bool) -> Option<FontData> {
    #[cfg(feature = "bundled-fonts-droid")]
    if let Some(font) = mupdf_fonts_droid::cjk_font(ordering, serif) {
        return Some(FontData {
            name: font.name,
            data: font.data,
            index: font.index,
        });
    }

    let _ = (ordering, serif);
    None
}

fn find_fallback_font(script: c_int, language: c_int, serif: bool) -> Option<FontData> {
    #[cfg(feature = "bundled-fonts-droid")]
    if is_cjk_script(script) {
        if let Some(font) = mupdf_fonts_droid::cjk_font(FZ_ADOBE_JAPAN as i32, serif) {
            return Some(FontData {
                name: font.name,
                data: font.data,
                index: font.index,
            });
        }
    }

    #[cfg(feature = "bundled-fonts-noto")]
    {
        let stem = if script == UCDN_SCRIPT_ARABIC as i32
            && (language == FZ_LANG_ur as i32 || language == FZ_LANG_urd as i32)
        {
            Some("NastaliqUrdu")
        } else {
            // SAFETY: MuPDF returns either NULL or a static NUL-terminated string.
            let stem = unsafe { fz_lookup_noto_stem_from_script(context(), script, language) };
            if stem.is_null() {
                None
            } else {
                // SAFETY: Non-null value returned by MuPDF is a NUL-terminated C string.
                unsafe { CStr::from_ptr(stem) }.to_str().ok()
            }
        };

        if let Some(stem) = stem {
            if let Some(font) = mupdf_fonts_noto::find_by_stem(stem, serif) {
                return Some(FontData {
                    name: font.name,
                    data: font.data,
                    index: font.index,
                });
            }
        }
    }

    let _ = (script, language, serif);
    None
}

pub(crate) fn load_font(font: FontData) -> Option<Font> {
    let len = c_int::try_from(font.data.len()).ok()?;
    let name = std::ffi::CString::new(font.name).ok()?;
    let ctx = context();

    let mut err: *mut mupdf_error_t = ptr::null_mut();
    // SAFETY: The font bytes have static storage duration. The wrapper catches MuPDF exceptions
    // before returning to Rust.
    let font_ptr = unsafe {
        mupdf_new_font_from_memory(
            ctx,
            name.as_ptr(),
            font.index,
            font.data.as_ptr(),
            len,
            (&mut err) as *mut *mut mupdf_error_t,
        )
    };

    if !err.is_null() {
        // SAFETY: Non-null error pointers returned by mupdf wrappers are owned by the caller.
        unsafe { mupdf_drop_error(err) };
        return None;
    }

    if font_ptr.is_null() {
        return None;
    }

    // Match MuPDF's built-in Noto behavior: externally bundled fonts may be embedded.
    // SAFETY: `font_ptr` is a valid font owned by us.
    unsafe { fz_set_font_embedding(ctx, font_ptr, 1) };

    // SAFETY: `font_ptr` is a valid font with an owned reference.
    Some(unsafe { Font::from_raw(font_ptr) })
}

#[cfg(feature = "bundled-fonts-droid")]
fn is_cjk_script(script: c_int) -> bool {
    script == UCDN_SCRIPT_HAN as i32
        || script == UCDN_SCRIPT_HANGUL as i32
        || script == UCDN_SCRIPT_HIRAGANA as i32
        || script == UCDN_SCRIPT_KATAKANA as i32
        || script == UCDN_SCRIPT_BOPOMOFO as i32
}

#[cfg(all(
    test,
    any(feature = "bundled-fonts-noto", feature = "bundled-fonts-droid")
))]
mod tests {
    use super::*;

    #[cfg(feature = "bundled-fonts-noto")]
    #[test]
    fn noto_regular_font_satisfies_non_exact_bold_italic_requests() {
        assert_eq!(
            find_by_name("Noto Sans", true, true, false).unwrap().name,
            "Noto Sans"
        );
        assert!(find_by_name("Noto Sans", true, true, true).is_none());
    }

    #[cfg(feature = "bundled-fonts-droid")]
    #[test]
    fn droid_regular_font_satisfies_non_exact_bold_italic_requests() {
        assert_eq!(
            find_by_name("Droid Sans Fallback", true, true, false)
                .unwrap()
                .name,
            "Droid Sans Fallback"
        );
        assert!(find_by_name("Droid Sans Fallback", true, true, true).is_none());
    }

    /// Drift protection: every script stem MuPDF's `fz_lookup_noto_stem_from_script`
    /// can return must resolve to a bundled Noto font, so a MuPDF submodule bump
    /// that adds new scripts fails loudly here instead of silently rendering tofu.
    #[cfg(feature = "bundled-fonts-noto")]
    #[test]
    fn noto_crate_covers_all_mupdf_script_stems() {
        // Stems that are intentionally not served by the Noto crate: MuPDF ships
        // no Noto CJK fonts; CJK scripts are handled by the Droid provider in
        // `find_fallback_font` instead.
        const UNBUNDLED_STEMS: &[&str] = &["JP", "KR", "SC", "TC"];

        let ctx = crate::context();
        let mut missing = Vec::new();

        for script in 0..=UCDN_LAST_SCRIPT as i32 {
            // SAFETY: `ctx` is the process-global MuPDF context. MuPDF returns
            // either NULL or a static NUL-terminated string.
            let stem =
                unsafe { fz_lookup_noto_stem_from_script(ctx, script, FZ_LANG_UNSET as i32) };
            if stem.is_null() {
                continue;
            }
            // SAFETY: Non-null value returned by MuPDF is a NUL-terminated C string.
            let stem = unsafe { CStr::from_ptr(stem) }.to_str().unwrap();
            if UNBUNDLED_STEMS.contains(&stem) {
                continue;
            }
            if mupdf_fonts_noto::find_by_stem(stem, false).is_none() {
                missing.push((script, stem));
            }
        }

        assert!(
            missing.is_empty(),
            "mupdf-fonts-noto has no font for MuPDF script stems {missing:?}; \
             add the fonts to the crate or, if MuPDF does not ship them, extend UNBUNDLED_STEMS"
        );

        // The Urdu special case in `find_fallback_font` bypasses
        // `fz_lookup_noto_stem_from_script`; keep its hardcoded stem covered too.
        assert!(mupdf_fonts_noto::find_by_stem("NastaliqUrdu", false).is_some());
    }
}

#[cfg(feature = "bundled-fonts-noto")]
use std::ffi::CStr;
use std::os::raw::c_int;
use std::ptr;

use mupdf_sys::*;

#[derive(Clone, Copy)]
pub(crate) struct FontData {
    name: &'static str,
    data: &'static [u8],
    index: i32,
}

pub(crate) fn find_by_name(
    name: &str,
    bold: c_int,
    italic: c_int,
    needs_exact_metrics: c_int,
) -> Option<FontData> {
    let bold = bold != 0;
    let italic = italic != 0;
    let needs_exact_metrics = needs_exact_metrics != 0;

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

pub(crate) fn find_cjk_font(ordering: c_int, serif: c_int) -> Option<FontData> {
    #[cfg(feature = "bundled-fonts-droid")]
    if let Some(font) = mupdf_fonts_droid::cjk_font(ordering, serif != 0) {
        return Some(FontData {
            name: font.name,
            data: font.data,
            index: font.index,
        });
    }

    let _ = (ordering, serif);
    None
}

pub(crate) unsafe fn find_fallback_font(
    ctx: *mut fz_context,
    script: c_int,
    language: c_int,
    serif: c_int,
    _bold: c_int,
    _italic: c_int,
) -> Option<FontData> {
    #[cfg(feature = "bundled-fonts-droid")]
    if is_cjk_script(script) {
        if let Some(font) = mupdf_fonts_droid::cjk_font(FZ_ADOBE_JAPAN as i32, serif != 0) {
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
            let stem = unsafe { fz_lookup_noto_stem_from_script(ctx, script, language) };
            if stem.is_null() {
                None
            } else {
                // SAFETY: Non-null value returned by MuPDF is a NUL-terminated C string.
                unsafe { CStr::from_ptr(stem) }.to_str().ok()
            }
        };

        if let Some(stem) = stem {
            if let Some(font) = mupdf_fonts_noto::find_by_stem(stem, serif != 0) {
                return Some(FontData {
                    name: font.name,
                    data: font.data,
                    index: font.index,
                });
            }
        }
    }

    let _ = (ctx, script, language, serif);
    None
}

pub(crate) unsafe fn load_font(ctx: *mut fz_context, font: FontData) -> *mut fz_font {
    let Ok(len) = c_int::try_from(font.data.len()) else {
        return ptr::null_mut();
    };

    let Ok(name) = std::ffi::CString::new(font.name) else {
        return ptr::null_mut();
    };

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
        return ptr::null_mut();
    }

    if !font_ptr.is_null() {
        // Match MuPDF's built-in Noto behavior: externally bundled fonts may be embedded.
        unsafe { fz_set_font_embedding(ctx, font_ptr, 1) };
    }

    font_ptr
}

pub(crate) unsafe fn load_by_name(
    ctx: *mut fz_context,
    name: &str,
    bold: c_int,
    italic: c_int,
    needs_exact_metrics: c_int,
) -> *mut fz_font {
    if let Some(font) = find_by_name(name, bold, italic, needs_exact_metrics) {
        // SAFETY: The caller guarantees that `ctx` is a valid MuPDF context.
        unsafe { load_font(ctx, font) }
    } else {
        ptr::null_mut()
    }
}

pub(crate) unsafe fn load_cjk_font(
    ctx: *mut fz_context,
    ordering: c_int,
    serif: c_int,
) -> *mut fz_font {
    if let Some(font) = find_cjk_font(ordering, serif) {
        // SAFETY: The caller guarantees that `ctx` is a valid MuPDF context.
        unsafe { load_font(ctx, font) }
    } else {
        ptr::null_mut()
    }
}

pub(crate) unsafe fn load_fallback_font(
    ctx: *mut fz_context,
    script: c_int,
    language: c_int,
    serif: c_int,
    bold: c_int,
    italic: c_int,
) -> *mut fz_font {
    // SAFETY: The caller guarantees that `ctx` is a valid MuPDF context.
    if let Some(font) = unsafe { find_fallback_font(ctx, script, language, serif, bold, italic) } {
        // SAFETY: The caller guarantees that `ctx` is a valid MuPDF context.
        unsafe { load_font(ctx, font) }
    } else {
        ptr::null_mut()
    }
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
            find_by_name("Noto Sans", 1, 1, 0).unwrap().name,
            "Noto Sans"
        );
        assert!(find_by_name("Noto Sans", 1, 1, 1).is_none());
    }

    #[cfg(feature = "bundled-fonts-droid")]
    #[test]
    fn droid_regular_font_satisfies_non_exact_bold_italic_requests() {
        assert_eq!(
            find_by_name("Droid Sans Fallback", 1, 1, 0).unwrap().name,
            "Droid Sans Fallback"
        );
        assert!(find_by_name("Droid Sans Fallback", 1, 1, 1).is_none());
    }
}

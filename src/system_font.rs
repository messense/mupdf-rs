use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::ptr;

use font_kit::family_name::FamilyName;
use font_kit::handle::Handle;
use font_kit::properties::{Properties, Style, Weight};
use font_kit::source::SystemSource;

use crate::font::Font;
#[cfg(windows)]
use crate::CjkFontOrdering;

use mupdf_sys::*;

pub(crate) unsafe extern "C" fn load_system_font(
    ctx: *mut fz_context,
    name: *const c_char,
    bold: c_int,
    italic: c_int,
    needs_exact_metrics: c_int,
) -> *mut fz_font {
    if let Ok(mut name) = CStr::from_ptr(name).to_str() {
        let font_source = SystemSource::new();
        let handle = match font_source.select_by_postscript_name(name) {
            Ok(handle) => Ok(handle),
            Err(_) => {
                for suffix in &["MT", "PS", "IdentityH"] {
                    if name.ends_with(suffix) {
                        name = name.strip_suffix(suffix).unwrap_or(name);
                    }
                }
                let mut properties = Properties::new();
                let properties = properties
                    .weight(if bold == 1 {
                        Weight::BOLD
                    } else {
                        Weight::NORMAL
                    })
                    .style(if italic == 1 {
                        Style::Italic
                    } else {
                        Style::Normal
                    });
                font_source.select_best_match(&[FamilyName::Title(name.to_string())], properties)
            }
        };
        if let Ok(handle) = handle {
            let font_index = match handle {
                Handle::Path { font_index, .. } => font_index,
                Handle::Memory { font_index, .. } => font_index,
            };
            let font = match handle.load() {
                Ok(f) => Font::from_bytes_with_index(
                    &f.family_name(),
                    font_index as i32,
                    &f.copy_font_data().unwrap(),
                ),
                Err(_) => return ptr::null_mut(),
            };
            if let Ok(font) = font {
                if needs_exact_metrics == 1
                    && ((bold == 1 && !font.is_bold()) || (italic == 1 && !font.is_italic()))
                {
                    return ptr::null_mut();
                }
                fz_keep_font(ctx, font.inner);
                return font.inner;
            }
        }
    }
    ptr::null_mut()
}

#[cfg(windows)]
unsafe fn load_font_by_names(ctx: *mut fz_context, names: &[&str]) -> *mut fz_font {
    use std::ffi::CString;

    for name in names {
        let c_name = CString::new(*name).unwrap();
        let font = load_system_font(ctx, c_name.as_ptr(), 0, 0, 0);
        if !font.is_null() {
            return font;
        }
    }
    ptr::null_mut()
}

#[cfg(windows)]
pub unsafe extern "C" fn load_system_cjk_font(
    ctx: *mut fz_context,
    name: *const c_char,
    ordering: c_int,
    serif: c_int,
) -> *mut fz_font {
    // Try name first
    let font = load_system_font(ctx, name, 0, 0, 0);
    if !font.is_null() {
        return font;
    }
    if serif == 1 {
        match CjkFontOrdering::try_from(ordering) {
            Ok(CjkFontOrdering::AdobeCns) => {
                return load_font_by_names(ctx, &["MingLiU"]);
            }
            Ok(CjkFontOrdering::AdobeGb) => {
                return load_font_by_names(ctx, &["SimSun"]);
            }
            Ok(CjkFontOrdering::AdobeJapan) => {
                return load_font_by_names(ctx, &["MS-Mincho"]);
            }
            Ok(CjkFontOrdering::AdobeKorea) => {
                return load_font_by_names(ctx, &["Batang"]);
            }
            Err(_) => {}
        }
    } else {
        match CjkFontOrdering::try_from(ordering) {
            Ok(CjkFontOrdering::AdobeCns) => {
                return load_font_by_names(ctx, &["DFKaiShu-SB-Estd-BF"]);
            }
            Ok(CjkFontOrdering::AdobeGb) => {
                return load_font_by_names(ctx, &["KaiTi", "KaiTi_GB2312"]);
            }
            Ok(CjkFontOrdering::AdobeJapan) => {
                return load_font_by_names(ctx, &["MS-Gothic"]);
            }
            Ok(CjkFontOrdering::AdobeKorea) => {
                return load_font_by_names(ctx, &["Gulim"]);
            }
            Err(_) => {}
        }
    }
    ptr::null_mut()
}

#[cfg(not(windows))]
pub(crate) unsafe extern "C" fn load_system_cjk_font(
    ctx: *mut fz_context,
    name: *const c_char,
    _ordering: c_int,
    _serif: c_int,
) -> *mut fz_font {
    // Try name first
    load_system_font(ctx, name, 0, 0, 0)
}

pub(crate) unsafe extern "C" fn load_system_fallback_font(
    _ctx: *mut fz_context,
    _script: c_int,
    _language: c_int,
    _serif: c_int,
    _bold: c_int,
    _italic: c_int,
) -> *mut fz_font {
    ptr::null_mut()
}

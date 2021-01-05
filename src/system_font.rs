use std::convert::TryFrom;
use std::ffi::{CStr, CString};
use std::fs;
use std::os::raw::{c_char, c_int};
use std::ptr;

use font_kit::family_name::FamilyName;
use font_kit::handle::Handle;
use font_kit::properties::{Properties, Style, Weight};
use font_kit::source::SystemSource;
use num_enum::TryFromPrimitive;

use crate::font::Font;
use mupdf_sys::*;

pub unsafe extern "C" fn load_system_font(
    ctx: *mut fz_context,
    name: *const c_char,
    bold: c_int,
    italic: c_int,
    needs_exact_metrics: c_int,
) -> *mut fz_font {
    if let Ok(name) = CStr::from_ptr(name).to_str() {
        let name = if name == "SimSun" { "SimSong" } else { name };
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
        let handle = SystemSource::new()
            .select_best_match(&[FamilyName::Title(name.to_string())], &properties);
        if let Ok(handle) = handle {
            let font = match handle {
                Handle::Path { path, font_index } => match fs::read(path) {
                    Ok(bytes) => Font::from_bytes_with_index(name, font_index as _, &bytes),
                    Err(_) => return ptr::null_mut(),
                },
                Handle::Memory { bytes, font_index } => {
                    Font::from_bytes_with_index(name, font_index as _, &bytes)
                }
            };
            match font {
                Ok(font) => {
                    if needs_exact_metrics == 1 && font.name() != name {
                        return ptr::null_mut();
                    }
                    fz_keep_font(ctx, font.inner);
                    return font.inner;
                }
                Err(_) => {}
            }
        }
    }
    ptr::null_mut()
}

#[derive(TryFromPrimitive)]
#[repr(u32)]
enum Ordering {
    AdobeCns = FZ_ADOBE_CNS as u32,
    AdobeGb = FZ_ADOBE_GB as u32,
    AdobeJapan = FZ_ADOBE_JAPAN as u32,
    AdobeKorea = FZ_ADOBE_KOREA as u32,
}

unsafe fn load_font_by_names(ctx: *mut fz_context, names: &[&str]) -> *mut fz_font {
    for name in names {
        let c_name = CString::new(*name).unwrap();
        let font = load_system_font(ctx, c_name.as_ptr(), 0, 0, 0);
        if !font.is_null() {
            return font;
        }
    }
    ptr::null_mut()
}

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
        match Ordering::try_from(ordering as u32) {
            Ok(Ordering::AdobeCns) => {
                return if cfg!(target_os = "linux") {
                    load_font_by_names(ctx, &[])
                } else if cfg!(target_os = "macos") {
                    load_font_by_names(ctx, &[])
                } else if cfg!(target_os = "windows") {
                    load_font_by_names(ctx, &["MingLiU"])
                } else {
                    load_font_by_names(ctx, &[])
                }
            }
            Ok(Ordering::AdobeGb) => {
                return if cfg!(target_os = "linux") {
                    load_font_by_names(ctx, &[])
                } else if cfg!(target_os = "macos") {
                    load_font_by_names(ctx, &[])
                } else if cfg!(target_os = "windows") {
                    load_font_by_names(ctx, &["SimSun"])
                } else {
                    load_font_by_names(ctx, &[])
                }
            }
            Ok(Ordering::AdobeJapan) => {
                return if cfg!(target_os = "linux") {
                    load_font_by_names(ctx, &[])
                } else if cfg!(target_os = "macos") {
                    load_font_by_names(ctx, &[])
                } else if cfg!(target_os = "windows") {
                    load_font_by_names(ctx, &["MS-Mincho"])
                } else {
                    load_font_by_names(ctx, &[])
                }
            }
            Ok(Ordering::AdobeKorea) => {
                return if cfg!(target_os = "linux") {
                    load_font_by_names(ctx, &[])
                } else if cfg!(target_os = "macos") {
                    load_font_by_names(ctx, &[])
                } else if cfg!(target_os = "windows") {
                    load_font_by_names(ctx, &["Batang"])
                } else {
                    load_font_by_names(ctx, &[])
                }
            }
            Err(_) => {}
        }
    } else {
        match Ordering::try_from(ordering as u32) {
            Ok(Ordering::AdobeCns) => {
                return if cfg!(target_os = "linux") {
                    load_font_by_names(ctx, &[])
                } else if cfg!(target_os = "macos") {
                    load_font_by_names(ctx, &[])
                } else if cfg!(target_os = "windows") {
                    load_font_by_names(ctx, &["DFKaiShu-SB-Estd-BF"])
                } else {
                    load_font_by_names(ctx, &[])
                }
            }
            Ok(Ordering::AdobeGb) => {
                return if cfg!(target_os = "linux") {
                    load_font_by_names(ctx, &[])
                } else if cfg!(target_os = "macos") {
                    load_font_by_names(ctx, &[])
                } else if cfg!(target_os = "windows") {
                    load_font_by_names(ctx, &["KaiTi", "KaiTi_GB2312"])
                } else {
                    load_font_by_names(ctx, &[])
                }
            }
            Ok(Ordering::AdobeJapan) => {
                return if cfg!(target_os = "linux") {
                    load_font_by_names(ctx, &[])
                } else if cfg!(target_os = "macos") {
                    load_font_by_names(ctx, &[])
                } else if cfg!(target_os = "windows") {
                    load_font_by_names(ctx, &["MS-Gothic"])
                } else {
                    load_font_by_names(ctx, &[])
                }
            }
            Ok(Ordering::AdobeKorea) => {
                return if cfg!(target_os = "linux") {
                    load_font_by_names(ctx, &[])
                } else if cfg!(target_os = "macos") {
                    load_font_by_names(ctx, &[])
                } else if cfg!(target_os = "windows") {
                    load_font_by_names(ctx, &["Gulim"])
                } else {
                    load_font_by_names(ctx, &[])
                }
            }
            Err(_) => {}
        }
    }
    ptr::null_mut()
}

pub unsafe extern "C" fn load_system_fallback_font(
    _ctx: *mut fz_context,
    _script: c_int,
    _language: c_int,
    _serif: c_int,
    _bold: c_int,
    _italic: c_int,
) -> *mut fz_font {
    ptr::null_mut()
}

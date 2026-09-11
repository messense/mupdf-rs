//! The `extern "C"` shims installed via `fz_install_load_system_font_funcs`.
//!
//! Each shim translates MuPDF's raw callback arguments into safe types and
//! dispatches to the [`FontLoader`](crate::FontLoader) chain (the registered
//! user loader first, then the built-in loaders). On non-Android, non-wasm
//! targets, `SystemFontLoader` is the built-in loader backed by `font-kit` for
//! fonts installed on the system.

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::ptr;

use mupdf_sys::*;

use crate::font_loader::{self, FontHints};
use crate::{CjkFontOrdering, Font};

/// Hand a `Font` over to MuPDF: return a pointer carrying one owned
/// reference for the caller, releasing our own when `font` drops.
fn font_into_mupdf(ctx: *mut fz_context, font: Font) -> *mut fz_font {
    // SAFETY: `ctx` and `font.inner` are valid; font reference counting in
    // MuPDF is thread-safe across cloned contexts.
    unsafe { fz_keep_font(ctx, font.inner) };
    font.inner
}

/// Font lookups run arbitrary user `FontLoader` code inside an `extern "C"`
/// callback, where unwinding would abort the process. Treat a panic as a
/// failed lookup instead so MuPDF can fall back to its built-in handling.
fn catch_panic(f: impl FnOnce() -> Option<Font>) -> Option<Font> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or(None)
}

pub(crate) unsafe extern "C" fn load_system_font(
    ctx: *mut fz_context,
    name: *const c_char,
    bold: c_int,
    italic: c_int,
    needs_exact_metrics: c_int,
) -> *mut fz_font {
    if name.is_null() {
        return ptr::null_mut();
    }

    let Ok(name) = unsafe { CStr::from_ptr(name) }.to_str() else {
        return ptr::null_mut();
    };

    let hints = FontHints {
        bold: bold != 0,
        italic: italic != 0,
        serif: false,
        needs_exact_metrics: needs_exact_metrics != 0,
    };
    match catch_panic(|| font_loader::dispatch(|loader| loader.load_font(name, hints))) {
        Some(font) => font_into_mupdf(ctx, font),
        None => ptr::null_mut(),
    }
}

pub(crate) unsafe extern "C" fn load_system_cjk_font(
    ctx: *mut fz_context,
    name: *const c_char,
    ordering: c_int,
    serif: c_int,
) -> *mut fz_font {
    let name = if name.is_null() {
        ""
    } else {
        // SAFETY: MuPDF passes a valid NUL-terminated string.
        unsafe { CStr::from_ptr(name) }.to_str().unwrap_or("")
    };

    let ordering = CjkFontOrdering::try_from(ordering).ok();
    match catch_panic(|| font_loader::dispatch_cjk(name, ordering, serif != 0)) {
        Some(font) => font_into_mupdf(ctx, font),
        None => ptr::null_mut(),
    }
}

pub(crate) unsafe extern "C" fn load_system_fallback_font(
    ctx: *mut fz_context,
    script: c_int,
    language: c_int,
    serif: c_int,
    bold: c_int,
    italic: c_int,
) -> *mut fz_font {
    let hints = FontHints {
        bold: bold != 0,
        italic: italic != 0,
        serif: serif != 0,
        needs_exact_metrics: false,
    };
    match catch_panic(|| {
        font_loader::dispatch(|loader| {
            loader.load_fallback_font(script as u32, language as u32, hints)
        })
    }) {
        Some(font) => font_into_mupdf(ctx, font),
        None => ptr::null_mut(),
    }
}

/// Looks up fonts installed on the system via `font-kit`.
// `font-kit` is only a dependency on non-Android, non-wasm targets.
#[cfg(all(
    feature = "system-fonts",
    not(target_arch = "wasm32"),
    not(target_os = "android")
))]
pub(crate) struct SystemFontLoader;

/// Names for which the system font database has actually been queried, in
/// order. Tests run in parallel and share this, so a test must only count
/// entries for a name it alone uses.
#[cfg(all(
    test,
    feature = "system-fonts",
    not(target_arch = "wasm32"),
    not(target_os = "android")
))]
static SYSTEM_LOOKUPS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

#[cfg(all(
    feature = "system-fonts",
    not(target_arch = "wasm32"),
    not(target_os = "android")
))]
impl font_loader::FontLoader for SystemFontLoader {
    fn load_font(&self, name: &str, hints: FontHints) -> Option<Font> {
        let cached = system_font_cache::lookup(name, hints)?;
        let font =
            Font::from_static_bytes_with_index(&cached.name, cached.index, cached.data).ok()?;

        if hints.needs_exact_metrics
            && ((hints.bold && !font.is_bold()) || (hints.italic && !font.is_italic()))
        {
            return None;
        }
        Some(font)
    }

    #[cfg(windows)]
    fn load_cjk_font(&self, _name: &str, ordering: CjkFontOrdering, serif: bool) -> Option<Font> {
        let names: &[&str] = if serif {
            match ordering {
                CjkFontOrdering::AdobeCns => &["MingLiU"],
                CjkFontOrdering::AdobeGb => &["SimSun"],
                CjkFontOrdering::AdobeJapan => &["MS-Mincho"],
                CjkFontOrdering::AdobeKorea => &["Batang"],
            }
        } else {
            match ordering {
                CjkFontOrdering::AdobeCns => &["DFKaiShu-SB-Estd-BF"],
                CjkFontOrdering::AdobeGb => &["KaiTi", "KaiTi_GB2312"],
                CjkFontOrdering::AdobeJapan => &["MS-Gothic"],
                CjkFontOrdering::AdobeKorea => &["Gulim"],
            }
        };
        names
            .iter()
            .find_map(|name| self.load_font(name, FontHints::default()))
    }
}

/// Process-wide cache in front of the `font-kit` system font lookup.
///
/// MuPDF asks the system font hook for every non-embedded font of every
/// document it opens (base-14 names like `Helvetica` included) and only
/// remembers the answer per document. A `font-kit` query is expensive: on
/// macOS it is a synchronous XPC round trip to the font daemon, which also
/// serialises concurrent callers; on Linux it is a fontconfig match. Documents
/// reference the same handful of names over and over, so the answer for each
/// name, hit or miss, is kept for the life of the process.
///
/// Font data is leaked to `'static` on first load so that every `Font` built
/// from it can share the bytes with MuPDF without copying. The cache is
/// bounded by the number of distinct fonts a process ever asks for.
#[cfg(all(
    feature = "system-fonts",
    not(target_arch = "wasm32"),
    not(target_os = "android")
))]
mod system_font_cache {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use font_kit::family_name::FamilyName;
    use font_kit::handle::Handle;
    use font_kit::properties::{Properties, Style, Weight};
    use font_kit::source::SystemSource;

    use crate::font_loader::FontHints;

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct Key {
        name: String,
        bold: bool,
        italic: bool,
    }

    /// A font found on the system, ready to hand to MuPDF.
    pub(super) struct CachedFont {
        pub(super) name: String,
        pub(super) index: i32,
        pub(super) data: &'static [u8],
    }

    static CACHE: Mutex<Option<HashMap<Key, Option<Arc<CachedFont>>>>> = Mutex::new(None);

    /// The system font matching `name` and the bold/italic hints, or `None`
    /// if there is none. `needs_exact_metrics` is not part of the key: it is
    /// a check on the returned font, applied by the caller.
    pub(super) fn lookup(name: &str, hints: FontHints) -> Option<Arc<CachedFont>> {
        let key = Key {
            name: name.to_owned(),
            bold: hints.bold,
            italic: hints.italic,
        };
        // A panic inside `font-kit` poisons the lock; the map is still valid.
        let mut guard = CACHE.lock().unwrap_or_else(|e| e.into_inner());
        let cache = guard.get_or_insert_with(HashMap::new);
        if let Some(cached) = cache.get(&key) {
            return cached.clone();
        }
        // The lock is held across the query on purpose: concurrent lookups
        // of the same name would otherwise all pay for it.
        let found = query_system(name, hints).map(Arc::new);
        cache.insert(key, found.clone());
        found
    }

    fn query_system(name: &str, hints: FontHints) -> Option<CachedFont> {
        #[cfg(test)]
        super::SYSTEM_LOOKUPS.lock().unwrap().push(name.to_owned());

        let mut name = name;
        let font_source = SystemSource::new();
        let handle = match font_source.select_by_postscript_name(name) {
            Ok(handle) => handle,
            Err(_) => {
                for suffix in &["MT", "PS", "IdentityH"] {
                    if name.ends_with(suffix) {
                        name = name.strip_suffix(suffix).unwrap_or(name);
                    }
                }
                let mut properties = Properties::new();
                let properties = properties
                    .weight(if hints.bold {
                        Weight::BOLD
                    } else {
                        Weight::NORMAL
                    })
                    .style(if hints.italic {
                        Style::Italic
                    } else {
                        Style::Normal
                    });
                font_source
                    .select_best_match(&[FamilyName::Title(name.to_string())], properties)
                    .ok()?
            }
        };

        let index = match handle {
            Handle::Path { font_index, .. } => font_index,
            Handle::Memory { font_index, .. } => font_index,
        };
        let loaded = handle.load().ok()?;
        let data = loaded.copy_font_data()?;
        let data: &'static [u8] = match Arc::try_unwrap(data) {
            Ok(vec) => Box::leak(vec.into_boxed_slice()),
            Err(shared) => Box::leak(shared.as_slice().to_vec().into_boxed_slice()),
        };
        Some(CachedFont {
            name: loaded.family_name(),
            index: index as i32,
            data,
        })
    }
}

#[cfg(all(
    test,
    feature = "system-fonts",
    not(target_arch = "wasm32"),
    not(target_os = "android")
))]
mod system_font_cache_tests {
    use super::*;
    use crate::font_loader::FontLoader;

    fn lookups_of(name: &str) -> usize {
        SYSTEM_LOOKUPS
            .lock()
            .unwrap()
            .iter()
            .filter(|n| n == &name)
            .count()
    }

    /// MuPDF asks the system font hook for every non-embedded font of every
    /// document it opens, and only caches the answer per document. Each
    /// font-kit query is expensive (a synchronous XPC round trip to the font
    /// daemon on macOS, a fontconfig match elsewhere), so repeated lookups of
    /// the same name must be served from a process-wide cache, including
    /// lookups that found nothing.
    #[test]
    fn repeated_lookups_query_the_system_once() {
        let hints = FontHints {
            bold: true,
            ..FontHints::default()
        };
        // A name no font on any system has, so the result is a miss and the
        // test does not depend on installed fonts.
        let name = "MupdfRsSystemFontCacheProbe";

        assert!(SystemFontLoader.load_font(name, hints).is_none());
        assert_eq!(lookups_of(name), 1, "first lookup must query the system");

        assert!(SystemFontLoader.load_font(name, hints).is_none());
        assert_eq!(
            lookups_of(name),
            1,
            "second lookup of the same name must be served from the cache"
        );
    }
}

#[cfg(all(test, feature = "bundled-fonts-droid", feature = "bundled-fonts-noto"))]
mod tests {
    use std::ffi::{CStr, CString};

    use super::*;

    #[test]
    fn bundled_cjk_hook_prefers_explicit_font_name() {
        let name = CString::new("Noto Sans").unwrap();
        let ctx = crate::context();
        // SAFETY: `ctx` is the process-global MuPDF context and `name` is a valid C string.
        let font = unsafe { load_system_cjk_font(ctx, name.as_ptr(), FZ_ADOBE_JAPAN as c_int, 0) };
        assert!(!font.is_null());

        // SAFETY: `font` is non-null and owned by this test until it is dropped below.
        let actual = unsafe { CStr::from_ptr(fz_font_name(ctx, font)) }
            .to_str()
            .unwrap()
            .to_owned();
        // SAFETY: `font` was returned with an owned reference from the system CJK hook.
        unsafe { fz_drop_font(ctx, font) };

        assert_eq!(actual, "Noto Sans");
    }
}

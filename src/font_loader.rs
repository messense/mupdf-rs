use std::ffi::CStr;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use mupdf_sys::*;
use once_cell::sync::Lazy;

use crate::{context, Buffer, CjkFontOrdering, Font};

/// Style hints passed by MuPDF when requesting a font.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FontHints {
    pub bold: bool,
    pub italic: bool,
    pub serif: bool,
    /// If true, the returned font must actually have the requested bold/italic
    /// metrics; synthetic styling is not acceptable.
    pub needs_exact_metrics: bool,
}

/// Provide fonts to MuPDF when a document references a font that is neither
/// embedded in the document nor built into the library.
///
/// Register an implementation with [`set_font_loader`]. The registered loader
/// is consulted *before* the built-in lookup paths (bundled fonts and system
/// fonts via `font-kit`), so it can also be used to override them.
///
/// Implementations must be `Send + Sync`: MuPDF may invoke these callbacks
/// from any thread that uses a [`Context`](crate::Context).
///
/// All methods default to returning `None`, which passes the request on to
/// the next lookup path.
pub trait FontLoader: Send + Sync + 'static {
    /// A document requested a font by name (e.g. `"Helvetica-Bold"`).
    fn load_font(&self, name: &str, hints: FontHints) -> Option<Font> {
        let _ = (name, hints);
        None
    }

    /// A document requested a CJK font for the given ROS ordering.
    fn load_cjk_font(&self, name: &str, ordering: CjkFontOrdering, serif: bool) -> Option<Font> {
        let _ = (name, ordering, serif);
        None
    }

    /// No available font contains a needed glyph; asked per script/language.
    ///
    /// `script` is a `mupdf_sys::UCDN_SCRIPT_*` value and `language` is a
    /// `mupdf_sys::FZ_LANG_*` (`fz_text_language`) value.
    fn load_fallback_font(&self, script: u32, language: u32, hints: FontHints) -> Option<Font> {
        let _ = (script, language, hints);
        None
    }
}

#[cfg(target_os = "android")]
fn default_loader() -> Option<Box<dyn FontLoader>> {
    Some(Box::new(AndroidFontLoader))
}

#[cfg(not(target_os = "android"))]
fn default_loader() -> Option<Box<dyn FontLoader>> {
    None
}

static FONT_LOADER: Lazy<RwLock<Option<Box<dyn FontLoader>>>> =
    Lazy::new(|| RwLock::new(default_loader()));

/// Register a global font loader, replacing any previously registered one
/// (including the default [`AndroidFontLoader`] on Android).
///
/// Call this once at startup, before opening documents: MuPDF caches resolved
/// fonts per context, so fonts that were already looked up are not re-queried.
pub fn set_font_loader(loader: impl FontLoader) {
    *FONT_LOADER.write().unwrap() = Some(Box::new(loader));
}

/// Built-in loaders consulted after the registered user loader, in priority
/// order.
const BUILT_IN_LOADERS: &[&dyn FontLoader] = &[
    #[cfg(feature = "bundled-fonts-runtime")]
    &crate::bundled_font::BundledFontLoader,
    #[cfg(feature = "system-fonts")]
    &crate::system_font::SystemFontLoader,
];

/// Run `f` against the registered font loader, if any.
///
/// A panic in user loader code is treated as a miss so the lookup can fall
/// back to the built-in loaders (and never unwinds into MuPDF).
fn with_user_loader(f: impl FnOnce(&dyn FontLoader) -> Option<Font>) -> Option<Font> {
    let guard = FONT_LOADER.read().ok()?;
    let loader = guard.as_ref()?.as_ref();
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(loader))).unwrap_or(None)
}

/// Run `f` against each loader in lookup order — the registered user loader
/// first, then the built-ins — returning the first hit.
pub(crate) fn dispatch(f: impl Fn(&dyn FontLoader) -> Option<Font>) -> Option<Font> {
    if let Some(font) = with_user_loader(&f) {
        return Some(font);
    }
    BUILT_IN_LOADERS.iter().find_map(|loader| f(*loader))
}

/// CJK lookup order: the user loader's CJK hook, then an exact-name lookup
/// across the whole chain (an explicitly requested font wins over generic
/// ordering-based substitutes), then the built-in loaders' CJK hooks.
pub(crate) fn dispatch_cjk(
    name: &str,
    ordering: Option<CjkFontOrdering>,
    serif: bool,
) -> Option<Font> {
    if let Some(ordering) = ordering {
        if let Some(font) = with_user_loader(|l| l.load_cjk_font(name, ordering, serif)) {
            return Some(font);
        }
    }
    if !name.is_empty() {
        if let Some(font) = dispatch(|l| l.load_font(name, FontHints::default())) {
            return Some(font);
        }
    }
    let ordering = ordering?;
    BUILT_IN_LOADERS
        .iter()
        .find_map(|l| l.load_cjk_font(name, ordering, serif))
}

/// Loads fonts from `/system/fonts` using the Noto/Roboto/Droid file naming
/// conventions used by Android, replacing MuPDF's need for bundled fonts.
///
/// This is the default font loader on Android targets. It is compiled on all
/// platforms so it can serve as a reference [`FontLoader`] implementation.
#[derive(Debug, Default, Clone, Copy)]
pub struct AndroidFontLoader;

// Face indices of the language-specific faces in the NotoSerifCJK/NotoSansCJK
// TrueType collections shipped on Android.
const JP: i32 = 0;
const KR: i32 = 1;
const SC: i32 = 2;
const TC: i32 = 3;

fn probe(a: &str, b: &str, c: &str) -> Option<PathBuf> {
    for ext in ["ttf", "otf", "ttc"] {
        let path = PathBuf::from(format!("/system/fonts/{a}{b}{c}.{ext}"));
        if path.exists() {
            return Some(path);
        }
    }
    None
}

/// Load a font file with no explicit name, so MuPDF derives the name from the
/// font data itself (mirrors `fz_new_font_from_file(ctx, NULL, ...)`).
fn font_from_file(path: &Path, index: i32) -> Option<Font> {
    let data = std::fs::read(path).ok()?;
    let buffer = Buffer::from_bytes(&data).ok()?;
    // SAFETY: `context()` is a valid context, `buffer.inner` is a valid
    // buffer and a NULL name is allowed by `fz_new_font_from_buffer`.
    let inner = unsafe {
        ffi_try!(mupdf_new_font_from_buffer(
            context(),
            std::ptr::null(),
            index,
            buffer.inner
        ))
    }
    .ok()?;
    // SAFETY: `inner` is a valid font with an owned reference.
    Some(unsafe { Font::from_raw(inner) })
}

fn load_noto(a: &str, b: &str, c: &str, index: i32) -> Option<Font> {
    font_from_file(&probe(a, b, c)?, index)
}

fn load_noto_cjk(lang: i32) -> Option<Font> {
    load_noto("NotoSerif", "CJK", "-Regular", lang)
        .or_else(|| load_noto("NotoSans", "CJK", "-Regular", lang))
        .or_else(|| load_noto("DroidSans", "Fallback", "", 0))
}

fn load_noto_arabic() -> Option<Font> {
    load_noto("Noto", "Naskh", "-Regular", 0)
        .or_else(|| load_noto("Noto", "NaskhArabic", "-Regular", 0))
        .or_else(|| load_noto("Droid", "Naskh", "-Regular", 0))
        .or_else(|| load_noto("NotoSerif", "Arabic", "-Regular", 0))
        .or_else(|| load_noto("NotoSans", "Arabic", "-Regular", 0))
        .or_else(|| load_noto("DroidSans", "Arabic", "-Regular", 0))
}

fn load_noto_try(stem: &str) -> Option<Font> {
    load_noto("NotoSerif", stem, "-Regular", 0)
        .or_else(|| load_noto("NotoSans", stem, "-Regular", 0))
        .or_else(|| load_noto("DroidSans", stem, "-Regular", 0))
}

impl FontLoader for AndroidFontLoader {
    fn load_font(&self, name: &str, hints: FontHints) -> Option<Font> {
        let style = match (hints.bold, hints.italic) {
            (true, true) => "-BoldItalic",
            (true, false) => "-Bold",
            (false, true) => "-Italic",
            (false, false) => "-Regular",
        };

        if name.eq_ignore_ascii_case("Helvetica")
            || name.eq_ignore_ascii_case("Arial")
            || name.contains("Helvetica")
            || name.contains("Arial")
        {
            return load_noto("Roboto", "", style, 0)
                .or_else(|| load_noto("NotoSans", "", style, 0))
                .or_else(|| load_noto("DroidSans", "", style, 0));
        }

        if name.eq_ignore_ascii_case("Times")
            || name.eq_ignore_ascii_case("Times-Roman")
            || name.contains("Times")
        {
            return load_noto("NotoSerif", "", style, 0)
                .or_else(|| load_noto("RobotoSerif", "", style, 0))
                .or_else(|| load_noto("DroidSerif", "", style, 0));
        }

        if name.eq_ignore_ascii_case("Courier") || name.contains("Courier") {
            return load_noto("DroidSans", "Mono", "", 0)
                .or_else(|| load_noto("NotoSans", "Mono", "-Regular", 0));
        }

        if name.eq_ignore_ascii_case("Symbol")
            || name.eq_ignore_ascii_case("ZapfDingbats")
            || name.contains("Symbol")
            || name.contains("Dingbats")
        {
            return load_noto("NotoSans", "Symbols", "-Regular", 0)
                .or_else(|| load_noto("NotoSans", "Symbols2", "-Regular", 0));
        }

        None
    }

    fn load_cjk_font(&self, _name: &str, ordering: CjkFontOrdering, _serif: bool) -> Option<Font> {
        match ordering {
            CjkFontOrdering::AdobeCns => load_noto_cjk(TC),
            CjkFontOrdering::AdobeGb => load_noto_cjk(SC),
            CjkFontOrdering::AdobeJapan => load_noto_cjk(JP),
            CjkFontOrdering::AdobeKorea => load_noto_cjk(KR),
        }
    }

    fn load_fallback_font(&self, script: u32, language: u32, _hints: FontHints) -> Option<Font> {
        // MuPDF's stem table is the same mapping the old androidfonts.c
        // switch hardcoded, so reuse it instead of duplicating it here.
        // SAFETY: MuPDF returns either NULL or a static NUL-terminated string.
        let stem =
            unsafe { fz_lookup_noto_stem_from_script(context(), script as i32, language as i32) };
        if stem.is_null() {
            return None;
        }
        // SAFETY: Non-null value returned by MuPDF is a NUL-terminated C string.
        let stem = unsafe { CStr::from_ptr(stem) }.to_str().ok()?;

        match stem {
            "JP" => load_noto_cjk(JP),
            "KR" => load_noto_cjk(KR),
            "SC" => load_noto_cjk(SC),
            "TC" => load_noto_cjk(TC),
            "Naskh" => load_noto_arabic(),
            stem => load_noto_try(stem),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use super::*;

    /// Only responds to names no other test uses, so registering it
    /// globally cannot interfere with concurrently running tests.
    struct TestLoader;

    impl FontLoader for TestLoader {
        fn load_font(&self, name: &str, hints: FontHints) -> Option<Font> {
            if name == "MupdfRsFontLoaderPanic" {
                panic!("font loader panicked");
            }
            (name == "MupdfRsFontLoaderTest" && hints.bold && !hints.italic).then(|| {
                // Independent of the base14/bundled/system font features.
                let data = include_bytes!("../tests/files/custom.ttf");
                Font::from_bytes("MupdfRsFontLoaderTest", data).unwrap()
            })
        }
    }

    #[test]
    fn user_font_loader_is_consulted_first() {
        set_font_loader(TestLoader);

        let ctx = crate::context();
        let name = CString::new("MupdfRsFontLoaderTest").unwrap();
        // SAFETY: `ctx` is the process-global MuPDF context and `name` is a valid C string.
        let font = unsafe { crate::system_font::load_system_font(ctx, name.as_ptr(), 1, 0, 0) };
        assert!(!font.is_null());

        // SAFETY: `font` is non-null and owned by this test until it is dropped below.
        let actual = unsafe { CStr::from_ptr(fz_font_name(ctx, font)) }
            .to_str()
            .unwrap()
            .to_owned();
        // SAFETY: `font` was returned with an owned reference from the hook.
        unsafe { fz_drop_font(ctx, font) };
        assert_eq!(actual, "MupdfRsFontLoaderTest");

        // A panicking loader must be treated as a miss instead of unwinding
        // out of the extern "C" shim (which would abort the process and thus
        // fail this whole test binary). The lookup may still produce a font
        // from the built-in loaders (e.g. a font-kit fallback match).
        let name = CString::new("MupdfRsFontLoaderPanic").unwrap();
        // SAFETY: as above.
        let font = unsafe { crate::system_font::load_system_font(ctx, name.as_ptr(), 1, 0, 0) };
        if !font.is_null() {
            // SAFETY: `font` was returned with an owned reference from the hook.
            unsafe { fz_drop_font(ctx, font) };
        }
    }
}

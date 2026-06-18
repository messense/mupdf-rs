use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::ptr;
use std::sync::Mutex;

use mupdf_sys::*;
use once_cell::sync::Lazy;

use crate::Error;

static BASE_CONTEXT: Lazy<Mutex<BaseContext>> = Lazy::new(|| {
    let ctx = unsafe {
        let base_ctx = mupdf_new_base_context();
        // Resolves fonts via the registered `FontLoader` (see
        // `crate::font_loader::set_font_loader`) and the built-in
        // bundled/system font lookup paths.
        fz_install_load_system_font_funcs(
            base_ctx,
            Some(crate::system_font::load_system_font),
            Some(crate::system_font::load_system_cjk_font),
            Some(crate::system_font::load_system_fallback_font),
        );
        base_ctx
    };
    Mutex::new(BaseContext(ctx))
});

thread_local! {
    static LOCAL_CONTEXT: RefCell<RawContext> = const { RefCell::new(RawContext(ptr::null_mut())) };
}

#[derive(Debug)]
struct BaseContext(*mut fz_context);

unsafe impl Send for BaseContext {}

impl Drop for BaseContext {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                mupdf_drop_base_context(self.0);
            }
        }
    }
}

#[derive(Debug)]
struct RawContext(*mut fz_context);

impl Drop for RawContext {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                fz_drop_context(self.0);
            }
        }
    }
}

#[derive(Debug)]
pub struct Context {
    pub(crate) inner: *mut fz_context,
}

impl Context {
    pub fn get() -> Self {
        LOCAL_CONTEXT.with(|ctx| {
            {
                let local = ctx.borrow();
                if !local.0.is_null() {
                    return Self { inner: local.0 };
                }
            }
            let base_ctx = BASE_CONTEXT.lock().unwrap();
            let new_ctx = unsafe { fz_clone_context(base_ctx.0) };
            if new_ctx.is_null() {
                panic!("failed to new fz_context");
            }
            *ctx.borrow_mut() = RawContext(new_ctx);
            Self { inner: new_ctx }
        })
    }

    pub fn enable_icc(&mut self) {
        unsafe {
            fz_enable_icc(self.inner);
        }
    }

    pub fn disable_icc(&mut self) {
        unsafe {
            fz_disable_icc(self.inner);
        }
    }

    pub fn aa_level(&self) -> i32 {
        unsafe { fz_aa_level(self.inner) }
    }

    pub fn set_aa_level(&mut self, bits: i32) {
        unsafe {
            fz_set_aa_level(self.inner, bits);
        }
    }

    pub fn text_aa_level(&self) -> i32 {
        unsafe { fz_text_aa_level(self.inner) }
    }

    pub fn set_text_aa_level(&mut self, bits: i32) {
        unsafe {
            fz_set_text_aa_level(self.inner, bits);
        }
    }

    pub fn graphics_aa_level(&self) -> i32 {
        unsafe { fz_graphics_aa_level(self.inner) }
    }

    pub fn set_graphics_aa_level(&mut self, bits: i32) {
        unsafe {
            fz_set_graphics_aa_level(self.inner, bits);
        }
    }

    pub fn graphics_min_line_width(&self) -> f32 {
        unsafe { fz_graphics_min_line_width(self.inner) }
    }

    pub fn set_graphics_min_line_width(&mut self, min_line_width: f32) {
        unsafe {
            fz_set_graphics_min_line_width(self.inner, min_line_width);
        }
    }

    pub fn use_document_css(&self) -> bool {
        unsafe { fz_use_document_css(self.inner) > 0 }
    }

    pub fn set_use_document_css(&mut self, should_use: bool) {
        let flag = if should_use { 1 } else { 0 };
        unsafe {
            fz_set_use_document_css(self.inner, flag);
        }
    }

    /// The user CSS string currently set on the context, if any.
    ///
    /// Returns an owned `String` because the underlying C string lives in
    /// context-owned storage that a sibling `Context` handle can free at any
    /// time via `set_user_css` (all `Context::get()` handles alias the same
    /// thread-local `fz_context`). Returning a borrow would be a use-after-free.
    pub fn user_css(&self) -> Option<String> {
        let css = unsafe { fz_user_css(self.inner) };
        if css.is_null() {
            return None;
        }
        let c_css = unsafe { CStr::from_ptr(css) };
        c_css.to_str().ok().map(str::to_owned)
    }

    pub fn set_user_css(&mut self, css: &str) -> Result<(), Error> {
        let c_css = CString::new(css)?;
        unsafe {
            fz_set_user_css(self.inner, c_css.as_ptr());
        }
        Ok(())
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::get()
    }
}

pub(crate) fn context() -> *mut fz_context {
    Context::get().inner
}

#[cfg(test)]
mod test {
    use std::sync::Mutex;

    use super::Context;
    use mupdf_sys::fz_set_user_css;

    // `user_css` lives in the shared `fz_style_context`, so a `set_user_css` on
    // any handle is visible process-wide. Serialize the tests that touch it.
    static USER_CSS_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_context() {
        let ctx = Context::get();
        assert_eq!(ctx.aa_level(), 8);
        assert_eq!(ctx.text_aa_level(), 8);
        assert_eq!(ctx.graphics_aa_level(), 8);
        assert_eq!(ctx.graphics_min_line_width(), 0.0);
        assert!(ctx.use_document_css());
        let _guard = USER_CSS_LOCK.lock().unwrap();
        assert!(ctx.user_css().is_none());
    }

    /// `user_css()` returns an owned `String` (not a borrow) because a sibling
    /// `Context::get()` handle can free the underlying C string via
    /// `set_user_css`. The owned copy must outlive such a call.
    #[test]
    fn user_css_owned_survives_set() {
        let _guard = USER_CSS_LOCK.lock().unwrap();
        let mut c = Context::get();
        c.set_user_css("body { color: red; }").unwrap();
        let owned = c.user_css().unwrap();
        Context::get()
            .set_user_css("body { color: blue; }")
            .unwrap();
        assert_eq!(owned, "body { color: red; }");
        // Restore the shared style to its default (no user CSS) for other tests.
        unsafe { fz_set_user_css(super::context(), std::ptr::null()) };
    }
}

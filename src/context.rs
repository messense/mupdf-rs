use std::cell::RefCell;
use std::ffi::{c_void, CStr, CString};
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{LazyLock, Mutex};

use mupdf_sys::*;

use crate::Error;

/// The maximum size (in bytes) of MuPDF's resource store. Applied when the
/// context is first created. `0` implies "use default".
static STORE_MAX: AtomicUsize = AtomicUsize::new(0);

static BASE_CONTEXT: LazyLock<Mutex<BaseContext>> = LazyLock::new(|| {
    let store_max = STORE_MAX.load(Ordering::Acquire);
    // SAFETY: creating a base context has no preconditions.
    let ctx = unsafe { new_base_context(store_max) };
    if ctx.is_null() {
        panic!("failed to create MuPDF base context");
    }
    Mutex::new(BaseContext(ctx))
});

/// Create a base context (its own family with its own lock set and store)
/// with the crate's font hooks installed. Returns null on failure.
unsafe fn new_base_context(store_max: usize) -> *mut fz_context {
    let ctx = unsafe { mupdf_new_base_context(store_max) };
    if !ctx.is_null() {
        // Resolves fonts via the registered `FontLoader` (see
        // `crate::font_loader::set_font_loader`) and the built-in
        // bundled/system font lookup paths.
        unsafe {
            fz_install_load_system_font_funcs(
                ctx,
                Some(crate::system_font::load_system_font),
                Some(crate::system_font::load_system_cjk_font),
                Some(crate::system_font::load_system_fallback_font),
            )
        };
    }
    ctx
}

thread_local! {
    static LOCAL_CONTEXT: RefCell<LocalContext> = const { RefCell::new(LocalContext::Unset) };
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

/// The context the current thread uses for every MuPDF call.
#[derive(Debug)]
enum LocalContext {
    /// The thread has not used MuPDF yet.
    Unset,
    /// A clone of the process-wide base context: shares its store, caches
    /// and lock set with every other thread in this state.
    Clone(*mut fz_context),
    /// A base context of the thread's own, created by [`init_thread_context`].
    Independent(*mut fz_context),
}

impl LocalContext {
    fn ptr(&self) -> *mut fz_context {
        match self {
            LocalContext::Unset => ptr::null_mut(),
            LocalContext::Clone(ctx) | LocalContext::Independent(ctx) => *ctx,
        }
    }
}

impl Drop for LocalContext {
    fn drop(&mut self) {
        // SAFETY: each pointer is owned by this thread-local and dropped once.
        unsafe {
            match *self {
                LocalContext::Unset => {}
                LocalContext::Clone(ctx) => fz_drop_context(ctx),
                // Also frees the family's lock set: a base context created
                // here never has clones.
                LocalContext::Independent(ctx) => mupdf_drop_base_context(ctx),
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
                let local = ctx.borrow().ptr();
                if !local.is_null() {
                    return Self { inner: local };
                }
            }
            let base_ctx = BASE_CONTEXT.lock().unwrap();
            let new_ctx = unsafe { fz_clone_context(base_ctx.0) };
            if new_ctx.is_null() {
                panic!("failed to new fz_context");
            }
            *ctx.borrow_mut() = LocalContext::Clone(new_ctx);
            Self { inner: new_ctx }
        })
    }

    /// Identifies the family (a base context and its clones) this context
    /// belongs to. Objects may only be handed to MuPDF under a context of
    /// the family that created them.
    pub(crate) fn family(&self) -> *const c_void {
        unsafe { mupdf_context_lock_set(self.inner) }
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

/// The family of the current thread's context; see [`Context::family`].
pub(crate) fn current_family() -> *const c_void {
    Context::get().family()
}

/// Give the calling thread a MuPDF context of its own instead of a clone of
/// the process-wide one.
///
/// By default every thread works on a clone of one base context. Clones share
/// that context's resource store and caches, and therefore its locks: MuPDF
/// takes a lock on every allocation and reference-count change, so threads
/// sharing a base context do not run MuPDF work in parallel. A thread that
/// calls this function gets a base context of its own, with its own store,
/// caches and locks, and never contends with other threads.
///
/// Call it on a thread before that thread uses MuPDF in any way, for example
/// at the start of a worker loop or from a thread pool's start handler.
/// `store_max` is the maximum size in bytes of this thread's resource store;
/// `None` uses the value given to [`set_store_max_size`], or MuPDF's default.
///
/// Costs and differences from the default:
///
/// - Each such thread holds its own store, glyph cache and loaded fonts, so
///   memory use grows with the number of threads and one thread never
///   benefits from another's cache.
/// - Context settings such as user CSS and antialiasing levels are per
///   family; values set on the shared context are not visible here.
/// - A [`DisplayList`](crate::DisplayList) recorded on one family cannot be
///   used on another: its methods return [`Error::ForeignContext`] and
///   dropping it elsewhere leaks it. Types that are not `Send` cannot leave
///   their thread in the first place.
///
/// # Errors
///
/// [`Error::AlreadyInitialized`] if this thread has already used MuPDF, and
/// [`Error::UnexpectedNullPtr`] if MuPDF could not create the context.
pub fn init_thread_context(store_max: Option<usize>) -> Result<(), Error> {
    LOCAL_CONTEXT.with(|local| {
        let mut local = local.borrow_mut();
        if !matches!(*local, LocalContext::Unset) {
            return Err(Error::AlreadyInitialized);
        }
        let store_max = store_max.unwrap_or_else(|| STORE_MAX.load(Ordering::Acquire));
        // SAFETY: creating a base context has no preconditions.
        let ctx = unsafe { new_base_context(store_max) };
        if ctx.is_null() {
            return Err(Error::UnexpectedNullPtr);
        }
        *local = LocalContext::Independent(ctx);
        Ok(())
    })
}

/// Set the maximum size (in bytes) of MuPDF's resource store.
///
/// Limits the size of MuPDF's internal cache.
/// This corresponds to the `max_store` argument of MuPDF's `fz_new_context`.
///
/// # Errors
///
/// If the context was initialised before or during this call.
pub fn set_store_max_size(bytes: usize) -> Result<(), Error> {
    if LazyLock::get(&BASE_CONTEXT).is_some() {
        return Err(Error::AlreadyInitialized);
    }
    STORE_MAX.store(bytes, Ordering::Release);
    if LazyLock::get(&BASE_CONTEXT).is_some() {
        return Err(Error::AlreadyInitialized);
    }
    Ok(())
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

    /// Each base context must own its lock set. If all of them share one
    /// process-wide set, work never runs in parallel across families, and
    /// dropping any base context tears the mutexes down under the others
    /// (see issue #260).
    #[test]
    fn base_contexts_own_their_lock_sets() {
        use mupdf_sys::{
            fz_clone_context, fz_drop_context, mupdf_context_lock_set, mupdf_drop_base_context,
            mupdf_new_base_context,
        };

        unsafe {
            let a = mupdf_new_base_context(0);
            let b = mupdf_new_base_context(0);
            assert!(!a.is_null() && !b.is_null());

            let locks_a = mupdf_context_lock_set(a);
            let locks_b = mupdf_context_lock_set(b);
            assert!(!locks_a.is_null(), "a base context must carry a lock set");
            assert_ne!(locks_a, locks_b, "base contexts must not share a lock set");

            // A clone belongs to its base context's family.
            let a2 = fz_clone_context(a);
            assert!(!a2.is_null());
            assert_eq!(mupdf_context_lock_set(a2), locks_a);
            fz_drop_context(a2);

            // Dropping one family leaves the other fully usable.
            mupdf_drop_base_context(a);
            let b2 = fz_clone_context(b);
            assert!(!b2.is_null());
            fz_drop_context(b2);
            mupdf_drop_base_context(b);

            // NULL is a no-op rather than a teardown of live mutexes.
            mupdf_drop_base_context(std::ptr::null_mut());
        }
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn init_thread_context_gives_the_thread_its_own_family() {
        let shared = Context::get().family() as usize;
        std::thread::spawn(move || {
            super::init_thread_context(None).unwrap();
            let mine = Context::get().family() as usize;
            assert_ne!(mine, shared, "must not clone the shared base context");
            assert_eq!(Context::get().family() as usize, mine, "must be stable");
            assert!(matches!(
                super::init_thread_context(None),
                Err(crate::Error::AlreadyInitialized)
            ));
            // The context is fully usable.
            let doc = crate::Document::open(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/files/dummy.pdf"
            ))
            .unwrap();
            assert_eq!(doc.page_count().unwrap(), 1);
        })
        .join()
        .unwrap();
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn init_thread_context_after_first_use_errors() {
        std::thread::spawn(|| {
            let _ = Context::get();
            assert!(matches!(
                super::init_thread_context(None),
                Err(crate::Error::AlreadyInitialized)
            ));
        })
        .join()
        .unwrap();
    }

    #[test]
    fn set_store_max_size_after_init_errors() {
        // Ensure the process-wide base context is initialized.
        let _ = Context::get();
        assert!(matches!(
            super::set_store_max_size(64 << 20),
            Err(crate::Error::AlreadyInitialized)
        ));
    }
}

use std::cell::RefCell;
use std::ptr;

use mupdf_sys::*;

thread_local! {
    static LOCAL_CONTEXT: RefCell<RawContext> = RefCell::new(RawContext(ptr::null_mut()));
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
            let new_ctx = unsafe {
                fz_new_context(ptr::null_mut(), ptr::null_mut(), FZ_STORE_DEFAULT as usize)
            };
            if new_ctx.is_null() {
                panic!("failed to new fz_context");
            }
            *ctx.borrow_mut() = RawContext(new_ctx);
            unsafe { fz_register_document_handlers(new_ctx) };
            Self { inner: new_ctx }
        })
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
    use super::Context;

    #[test]
    fn test_context() {
        let _ctx = Context::get();
    }
}

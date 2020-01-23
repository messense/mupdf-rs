use std::ptr;
use std::cell::RefCell;

use mupdf_sys::*;


thread_local! {
    static LOCAL_CONTEXT: RefCell<*mut fz_context> = RefCell::new(ptr::null_mut());
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
                if !local.is_null() {
                    return Self { inner: *local };
                }
            }
            let new_ctx = unsafe { 
                fz_new_context(ptr::null_mut(), ptr::null_mut(), FZ_STORE_DEFAULT as usize)
            };
            if new_ctx.is_null() {
                panic!("failed to new fz_context");
            }
            *ctx.borrow_mut() = new_ctx;
            Self { inner: new_ctx }
        })
    }
}

// FIXME: Add back Drop impl

impl Default for Context {
    fn default() -> Self {
        Self::get()
    }
}

#[cfg(test)]
mod test {
    use super::Context;

    #[test]
    fn test_context() {
        let _ctx = Context::get();
    }
}

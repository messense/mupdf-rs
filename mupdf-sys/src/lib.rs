#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub unsafe extern "C" fn fz_new_context(
    alloc: *const fz_alloc_context,
    locks: *const fz_locks_context,
    max_store: usize,
) -> *mut fz_context {
    fz_new_context_imp(alloc, locks, max_store, FZ_VERSION.as_ptr() as _)
}

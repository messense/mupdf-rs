use std::{
    ffi::c_void,
    ops::{Deref, DerefMut},
    ptr::{self, NonNull},
    slice,
};

use mupdf_sys::fz_free;

use crate::context;

/// Essentially a [`Box`]`<[T], A>` with `fz_calloc` as the allocator. Necessary until the
/// allocator API is stable, as we need to be able to free this with `fz_free` instead of the
/// system allocator.
// An important note about `fz_calloc`: If a function which allocates from `fz_calloc` gives you a
// pointer, allocated from `fz_calloc`, but says the length of items behind it is 0, you still have
// to `fz_free` that pointer. It's probably lying about the length behind it being 0 - it was
// probably part of an allocation bigger than 0, but of which 0 bytes were actually written to.
#[derive(Default)]
pub struct FzArray<T> {
    ptr: Option<NonNull<T>>,
    len: usize,
}

impl<T> FzArray<T> {
    /// # Safety
    ///
    /// * `ptr` must point to an array of at least `len` instances of `T`. There may be more than
    ///   `len` instances, but only `len` will be accessed by this struct.
    ///
    /// * If `len > 0`, the memory it points to also must be allocated by `fz_calloc` inside a
    ///   mupdf FFI call. `ptr` may be dangling or not well-aligned if `len == 0`
    pub(crate) unsafe fn from_parts(ptr: NonNull<T>, len: usize) -> Self {
        Self {
            ptr: Some(ptr),
            len,
        }
    }

    /// # Safety
    ///
    /// * It must be valid to call [`FzArray::from_parts`] with the ptr stored inside `self` and
    ///   the `len` specified in this call
    pub(crate) unsafe fn set_len(&mut self, len: usize) {
        self.len = len;
    }
}

impl<T> Deref for FzArray<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        match self.ptr {
            // SAFETY: `self.ptr.as_ptr()` is not non-null (as it's a NonNull) and the creator has
            // promised us that it does point to a valid slice. Also, if it does point to a
            Some(ptr) => unsafe { slice::from_raw_parts(ptr.as_ptr(), self.len) },
            None => &[],
        }
    }
}

impl<T> DerefMut for FzArray<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self.ptr.as_mut() {
            Some(ptr) => {
                let ptr = unsafe { ptr.as_mut() };
                unsafe { slice::from_raw_parts_mut(ptr, self.len) }
            }
            None => &mut [],
        }
    }
}

impl<T> Drop for FzArray<T> {
    fn drop(&mut self) {
        if let Some(ptr) = self.ptr {
            // SAFETY: Upheld by constructor - this must point to something allocated by fz_calloc
            unsafe { fz_free(context(), ptr.as_ptr() as *mut c_void) };
        }
    }
}

impl<T> IntoIterator for FzArray<T> {
    type IntoIter = FzIter<T>;
    type Item = T;
    fn into_iter(self) -> Self::IntoIter {
        let next_item_and_end = self.ptr.map(|p| {
            // Would be nice to use `.addr()` but it seems CI doesn't have 1.84 yet, and that's
            // what stabilized the strict provenance APIs
            let end = unsafe { p.add(self.len) }.as_ptr() as usize;
            (p, end)
        });
        FzIter {
            _kept_to_be_dropped: self,
            next_item_and_end,
        }
    }
}

pub struct FzIter<T> {
    _kept_to_be_dropped: FzArray<T>,
    next_item_and_end: Option<(NonNull<T>, usize)>,
}

impl<T> Iterator for FzIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        let (next_item, end_addr) = self.next_item_and_end.as_mut()?;

        // Same thing as above with `.addr()`
        if next_item.as_ptr() as usize == *end_addr {
            return None;
        }

        let ret = unsafe { ptr::read(next_item.as_ptr()) };
        *next_item = unsafe { next_item.add(1) };
        Some(ret)
    }
}

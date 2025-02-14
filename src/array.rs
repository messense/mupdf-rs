use std::{
    ffi::c_void,
    num::NonZero,
    ops::{Deref, DerefMut},
    ptr::{self, NonNull},
    slice,
};

use mupdf_sys::fz_free;

use crate::context;

/// Essentially a [`Box`]`<[T], A>` with `fz_calloc` as the allocator. Necessary until the
/// allocator API is stable, as we need to be able to free this with `fz_free` instead of the
/// system allocator.
pub struct FzArray<T> {
    pub(crate) ptr: NonNull<T>,
    pub(crate) len: usize,
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
        Self { ptr, len }
    }
}

impl<T> Default for FzArray<T> {
    fn default() -> Self {
        Self {
            ptr: NonNull::dangling(),
            len: 0,
        }
    }
}

impl<T> Deref for FzArray<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        // SAFETY: `self.ptr.as_ptr()` is not non-null (as it's a NonNull) and the creator has
        // promised us that it does point to a valid slice. Also, if it does point to a
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> DerefMut for FzArray<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let ptr = unsafe { self.ptr.as_mut() };
        unsafe { slice::from_raw_parts_mut(ptr, self.len) }
    }
}

impl<T> Drop for FzArray<T> {
    fn drop(&mut self) {
        if self.len != 0 {
            // SAFETY: Upheld by constructor - this must point to something allocated by fz_calloc
            unsafe { fz_free(context(), self.ptr.as_ptr() as *mut c_void) };
        }
    }
}

impl<T> IntoIterator for FzArray<T> {
    type IntoIter = FzIter<T>;
    type Item = T;
    fn into_iter(self) -> Self::IntoIter {
        let next_item = self.ptr;
        let end_addr = unsafe { self.ptr.add(self.len) }.addr();
        FzIter {
            _kept_to_be_dropped: self,
            next_item,
            end_addr,
        }
    }
}

pub struct FzIter<T> {
    _kept_to_be_dropped: FzArray<T>,
    next_item: NonNull<T>,
    end_addr: NonZero<usize>,
}

impl<T> Iterator for FzIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.next_item.addr() == self.end_addr {
            return None;
        }

        let ret = unsafe { ptr::read(self.next_item.as_ptr()) };
        self.next_item = unsafe { self.next_item.add(1) };
        Some(ret)
    }
}

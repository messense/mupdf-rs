//! Owned, NUL-terminated byte vector for PDF name and binary data.

use core::cmp::Ordering;
use core::ffi::{c_char, CStr};
use core::fmt;
use core::hash::{Hash, Hasher};
use core::num::NonZero;
use core::ops::Deref;
use std::borrow::Borrow;

use crate::error::NulError;
use crate::repr::Repr;

/// An owned, C-string-compatible byte buffer with a compact 24-byte representation.
///
/// `CompactCBytes` always maintains a trailing NUL terminator and rejects interior `\0`
/// bytes, allowing its contents to be viewed as a [`CStr`] without allocation.
///
/// It is built with several memory and performance optimizations:
///
/// * Stores up to 23 payload bytes inline, avoiding heap allocation for small values,
///   and spills longer payloads to a reference-counted [`ecow::EcoVec<u8>`].
///
/// * Maintains an invisible trailing NUL terminator, allowing instant,
///   allocation-free conversion to `&CStr` via [`as_cstr`](Self::as_cstr).
///
/// * Allows fast cloning, performed either by copying the 24-byte inline
///   representation or by incrementing the reference count of the spilled storage.
///
/// * Enables niche optimization, so `Option<CompactCBytes>` has the same size as `CompactCBytes`.
#[derive(Default, Clone)]
#[repr(transparent)]
pub struct CompactCBytes(pub(super) Repr);

impl CompactCBytes {
    /// Creates a `CompactCBytes` from a raw C string pointer.
    ///
    /// # Safety
    ///
    /// `ptr` must be a valid, NUL-terminated pointer.
    #[inline]
    pub unsafe fn from_raw_c_unchecked(ptr: *const c_char) -> Self {
        let c_str = unsafe { CStr::from_ptr(ptr) };
        Self::from(c_str)
    }

    /// Constructs a `CompactCBytes` from raw bytes without checking for interior NULs.
    ///
    /// # Safety
    ///
    /// `bytes` must not contain any `0` byte.
    #[inline]
    pub unsafe fn from_bytes_unchecked(bytes: &[u8]) -> Self {
        // SAFETY: forwarded from caller.
        Self(unsafe { Repr::from_slice_unchecked(bytes) })
    }

    /// Creates an inline `CompactCBytes` from a NUL-free byte payload.
    ///
    /// Returns `None` if the payload length is greater than or equal to
    /// [`INLINE_TOTAL`](super::INLINE_TOTAL), or if it contains a NUL byte.
    #[inline(always)]
    pub const fn try_inline(bytes: &[u8]) -> Option<Self> {
        match Repr::try_inline(bytes) {
            repr @ Some(_) => return Some(Self(repr.unwrap())),
            // Avoid const-evaluating the destructor for `Option<Repr>`.
            // This is `None`, so there is no initialized `Repr` to drop.
            repr @ None => core::mem::forget(repr),
        }
        None
    }

    /// Creates a new inline `CompactCBytes` from a byte slice.
    ///
    /// This constructor is `const` and never allocates. Therefore, the input must
    /// fit into the inline representation.
    ///
    /// # Panics
    ///
    /// Panics if the payload length is greater than or equal to
    /// [`INLINE_TOTAL`](super::INLINE_TOTAL), or if it contains a NUL byte.
    #[inline(always)]
    pub const fn new_inline(bytes: &[u8]) -> Self {
        Self::try_inline(bytes).expect("payload must be at most 23 bytes and contain no NUL bytes")
    }

    /// Returns `true` if the data is heap-allocated.
    #[inline]
    pub const fn is_heap(&self) -> bool {
        self.0.is_heap()
    }

    /// Returns `true` if the data is stored inline.
    #[inline]
    pub const fn is_inline(&self) -> bool {
        !self.is_heap()
    }

    /// Returns the payload length in bytes (without the NUL terminator).
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the payload is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the payload bytes without the trailing NUL.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        self
    }

    /// Returns the content as a `&CStr` without allocation.
    #[inline]
    pub fn as_cstr(&self) -> &CStr {
        self.0.as_cstr()
    }
}

impl TryFrom<&[u8]> for CompactCBytes {
    type Error = NulError;

    /// Validates that `value` contains no NUL bytes, then copies it into
    /// inline storage or spills it to heap storage.
    #[inline]
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Repr::from_slice(value).map(Self)
    }
}

impl TryFrom<&Vec<u8>> for CompactCBytes {
    type Error = NulError;
    #[inline]
    fn try_from(bytes: &Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(bytes.as_slice())
    }
}

impl From<&CStr> for CompactCBytes {
    #[inline]
    fn from(c_str: &CStr) -> Self {
        Self(Repr::from_cstr(c_str))
    }
}

impl From<&[NonZero<u8>]> for CompactCBytes {
    #[inline]
    fn from(bytes: &[NonZero<u8>]) -> Self {
        // SAFETY: `NonZero<u8>` has the same layout as `u8`, and all bytes are non-zero.
        let raw: &[u8] =
            unsafe { core::slice::from_raw_parts(bytes.as_ptr().cast::<u8>(), bytes.len()) };
        // SAFETY: `raw` is guaranteed to be NUL-free.
        Self(unsafe { Repr::from_slice_unchecked(raw) })
    }
}

impl From<&Vec<NonZero<u8>>> for CompactCBytes {
    #[inline]
    fn from(bytes: &Vec<NonZero<u8>>) -> Self {
        Self::from(bytes.as_slice())
    }
}

impl Deref for CompactCBytes {
    type Target = [u8];

    /// Returns the payload bytes without the trailing NUL.
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0.to_bytes()
    }
}

impl AsRef<[u8]> for CompactCBytes {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self
    }
}

impl AsRef<CStr> for CompactCBytes {
    #[inline]
    fn as_ref(&self) -> &CStr {
        self.as_cstr()
    }
}

impl Borrow<[u8]> for CompactCBytes {
    #[inline]
    fn borrow(&self) -> &[u8] {
        self
    }
}

impl<T: AsRef<[u8]> + ?Sized> PartialEq<T> for CompactCBytes {
    #[inline]
    fn eq(&self, other: &T) -> bool {
        self.as_slice() == other.as_ref()
    }
}

impl Eq for CompactCBytes {}

impl_partial_eq! {
    impl PartialEq<CompactCBytes>     for &CompactCBytes  { |self, other| self.as_slice() == other.as_slice() }

    impl PartialEq<CompactCBytes>     for Vec<u8>  { |self, other| self.as_slice() == other.as_slice() }
    impl PartialEq<&CompactCBytes>    for Vec<u8>  { |self, other| self.as_slice() == other.as_slice() }
    impl PartialEq<CompactCBytes>     for &Vec<u8> { |self, other| self.as_slice() == other.as_slice() }
    impl PartialEq<Vec<u8>>    for &CompactCBytes  { |self, other| self.as_slice() == other.as_slice() }

    impl PartialEq<CompactCBytes>     for [u8]     { |self, other| self   == other.as_slice() }
    impl PartialEq<&CompactCBytes>    for [u8]     { |self, other| self   == other.as_slice() }
    impl PartialEq<CompactCBytes>     for &[u8]    { |self, other| *self  == other.as_slice() }
    impl PartialEq<CompactCBytes>     for &&[u8]   { |self, other| **self == other.as_slice() }
}

impl PartialOrd for CompactCBytes {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CompactCBytes {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_slice().cmp(other.as_slice())
    }
}

impl Hash for CompactCBytes {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

impl fmt::Debug for CompactCBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CompactCBytes")
            .field(&self.as_slice())
            .finish()
    }
}

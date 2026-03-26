//! Owned, NUL-terminated, UTF-8 validated string for PDF text values.

use core::ffi::{c_char, CStr};
use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::Deref;
use core::str::{FromStr, Utf8Error};
use std::borrow::{Borrow, Cow};
use std::ffi::CString;

use crate::error::{FromBytesError, NulError};
use crate::vector::CompactCBytes;

/// An owned, UTF-8 validated string with a compact 24-byte representation.
///
/// `CompactCString` is a wrapper around [`CompactCBytes`] that additionally guarantees
/// that the payload is valid UTF-8.
#[derive(Debug, Default, Clone, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct CompactCString(CompactCBytes);

impl CompactCString {
    /// Creates a `CompactCString` from a `CompactCBytes` without checking UTF-8 validity.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `vec` contains valid UTF-8.
    #[inline]
    pub unsafe fn from_utf8_unchecked(vec: CompactCBytes) -> Self {
        Self(vec)
    }

    /// Constructs a `CompactCString` from a `CStr` without checking UTF-8 validity.
    ///
    /// # Safety
    ///
    /// The contents of `c_str` must be valid UTF-8.
    #[inline]
    pub unsafe fn from_utf8_cstr_unchecked(c_str: &CStr) -> Self {
        Self(CompactCBytes::from(c_str))
    }

    /// Constructs a `CompactCString` from raw bytes without validating UTF-8 or NULs.
    ///
    /// # Safety
    ///
    /// `bytes` must be valid UTF-8 and must not contain any NUL (`0`) bytes.
    #[inline]
    pub unsafe fn from_bytes_unchecked(bytes: &[u8]) -> Self {
        // SAFETY: the caller guarantees that `bytes` is valid UTF-8 and NUL-free.
        Self(unsafe { CompactCBytes::from_bytes_unchecked(bytes) })
    }

    /// Creates an inline `CompactCString` from a NUL-free byte payload.
    ///
    /// Returns `None` if the payload length is greater than or equal to
    /// [`INLINE_TOTAL`](super::INLINE_TOTAL), or if it contains a NUL byte.
    #[inline(always)]
    pub const fn try_inline(s: &str) -> Option<Self> {
        match CompactCBytes::try_inline(s.as_bytes()) {
            bytes @ Some(_) => return Some(Self(bytes.unwrap())),
            // Avoid const-evaluating the destructor for `Option<CompactCBytes>`.
            // This is `None`, so there is no initialized `CompactCBytes` to drop.
            bytes @ None => core::mem::forget(bytes),
        }
        None
    }

    /// Creates an inline `CompactCString` from a NUL-free string payload.
    ///
    /// # Panics
    ///
    /// Panics if the payload length is greater than or equal to
    /// [`INLINE_TOTAL`](super::INLINE_TOTAL), or if it contains a NUL byte.
    #[inline(always)]
    pub const fn new_inline(s: &str) -> Self {
        Self(CompactCBytes::new_inline(s.as_bytes()))
    }

    /// Returns `true` if the data is heap-allocated.
    #[inline]
    pub fn is_heap(&self) -> bool {
        self.0.is_heap()
    }

    /// Returns `true` if the data is stored inline.
    #[inline]
    pub fn is_inline(&self) -> bool {
        self.0.is_inline()
    }

    /// Returns the length in bytes (not chars).
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the string is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the raw UTF-8 bytes without the trailing NUL.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }

    /// Returns the content as a `&CStr` without any allocation.
    #[inline]
    pub fn as_cstr(&self) -> &CStr {
        self.0.as_cstr()
    }

    /// Returns a raw pointer to the NUL-terminated C string.
    #[inline]
    pub fn as_ptr(&self) -> *const c_char {
        self.as_cstr().as_ptr()
    }

    /// Returns the content as a `&str`.
    #[inline]
    pub fn as_str(&self) -> &str {
        self
    }

    /// Converts `self` into a [`String`].
    ///
    /// This allocates a new `String` and copies the UTF-8 payload.
    #[inline]
    pub fn into_string(self) -> String {
        self.as_str().to_owned()
    }

    /// Consumes `self` and returns the underlying [`CompactCBytes`].
    #[inline]
    pub fn into_bytes(self) -> CompactCBytes {
        self.0
    }
}

impl TryFrom<&str> for CompactCString {
    type Error = NulError;

    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let vec = CompactCBytes::try_from(s.as_bytes())?;
        // SAFETY: `s` is valid UTF-8, and `CompactCBytes::try_from` rejects any NUL bytes.
        Ok(unsafe { Self::from_utf8_unchecked(vec) })
    }
}

impl TryFrom<&String> for CompactCString {
    type Error = NulError;

    #[inline]
    fn try_from(s: &String) -> Result<Self, Self::Error> {
        Self::try_from(s.as_str())
    }
}

impl FromStr for CompactCString {
    type Err = NulError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

impl TryFrom<&CStr> for CompactCString {
    type Error = Utf8Error;

    #[inline]
    fn try_from(c_str: &CStr) -> Result<Self, Self::Error> {
        c_str.to_str()?;
        // SAFETY: `c_str` is a valid C string, and `to_str` verified its contents as UTF-8.
        Ok(unsafe { Self::from_utf8_cstr_unchecked(c_str) })
    }
}

impl TryFrom<&CString> for CompactCString {
    type Error = Utf8Error;

    #[inline]
    fn try_from(s: &CString) -> Result<Self, Self::Error> {
        Self::try_from(s.as_c_str())
    }
}

impl TryFrom<&[u8]> for CompactCString {
    type Error = FromBytesError;

    #[inline]
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let vec = CompactCBytes::try_from(bytes)?;
        Self::try_from(vec).map_err(FromBytesError::from)
    }
}

impl TryFrom<CompactCBytes> for CompactCString {
    type Error = Utf8Error;

    #[inline]
    fn try_from(value: CompactCBytes) -> Result<Self, Self::Error> {
        str::from_utf8(&value)?;
        Ok(CompactCString(value))
    }
}

impl From<CompactCString> for String {
    #[inline]
    fn from(s: CompactCString) -> Self {
        s.into_string()
    }
}

impl<'a> From<&'a CompactCString> for &'a str {
    #[inline]
    fn from(s: &'a CompactCString) -> Self {
        s.as_str()
    }
}

impl fmt::Display for CompactCString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Deref for CompactCString {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        // SAFETY: CompactCString is always constructed from validated UTF-8.
        unsafe { str::from_utf8_unchecked(self.as_bytes()) }
    }
}

impl Hash for CompactCString {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl AsRef<[u8]> for CompactCString {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl AsRef<CStr> for CompactCString {
    #[inline]
    fn as_ref(&self) -> &CStr {
        self.as_cstr()
    }
}

impl AsRef<str> for CompactCString {
    #[inline]
    fn as_ref(&self) -> &str {
        self
    }
}

impl Borrow<str> for CompactCString {
    #[inline]
    fn borrow(&self) -> &str {
        self
    }
}

impl<T: AsRef<str> + ?Sized> PartialEq<T> for CompactCString {
    #[inline]
    fn eq(&self, other: &T) -> bool {
        self.as_str() == other.as_ref()
    }
}

impl_partial_eq! {
    impl PartialEq<CompactCString>     for &CompactCString    { |self, other| self.as_str() == other.as_str() }

    impl PartialEq<CompactCString>     for String        { |self, other| self.as_str() == other.as_str() }
    impl PartialEq<&CompactCString>    for String        { |self, other| self.as_str() == other.as_str() }
    impl PartialEq<CompactCString>     for &String       { |self, other| self.as_str() == other.as_str() }
    impl PartialEq<String>        for &CompactCString    { |self, other| self.as_str() == other.as_str() }

    impl PartialEq<CompactCString>     for str           { |self, other| self   == other.as_str() }
    impl PartialEq<&CompactCString>    for str           { |self, other| self   == other.as_str() }
    impl PartialEq<CompactCString>     for &str          { |self, other| *self  == other.as_str() }
    impl PartialEq<CompactCString>     for &&str         { |self, other| **self == other.as_str() }

    impl PartialEq<CompactCString>     for Cow<'_, str>  { |self, other| self  == other.as_str() }
    impl PartialEq<CompactCString>     for &Cow<'_, str> { |self, other| *self == other.as_str() }
    impl PartialEq<Cow<'_, str>>  for &CompactCString    { |self, other| self.as_str() == other  }
}

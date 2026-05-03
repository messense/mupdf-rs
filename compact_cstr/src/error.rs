//! Error types returned by fallible constructors.

use core::error::Error;
use core::fmt;
use core::str::Utf8Error;

/// The supplied bytes contained a NUL (`b'\0'`) byte.
///
/// [`CompactCBytes`](crate::CompactCBytes) and [`CompactCString`](crate::CompactCString)  are internally
/// NUL-terminated, so payloads cannot contain NUL bytes.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct NulError {
    /// Byte index of the first interior NUL.
    pub position: usize,
}

impl NulError {
    /// Returns the byte index of the first interior NUL byte found.
    #[inline]
    pub const fn position(&self) -> usize {
        self.position
    }

    #[inline]
    pub(crate) const fn at(position: usize) -> Self {
        Self { position }
    }
}

impl fmt::Display for NulError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "interior NUL byte at position {}", self.position)
    }
}

impl Error for NulError {}

/// Error returned when constructing a [`CompactCString`](crate::CompactCString) from raw
/// bytes that may be neither NUL-free nor valid UTF-8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FromBytesError {
    /// The input contained an interior NUL byte.
    InteriorNul(NulError),
    /// The input was not valid UTF-8.
    InvalidUtf8(Utf8Error),
}

impl fmt::Display for FromBytesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InteriorNul(e) => write!(f, "{e}"),
            Self::InvalidUtf8(e) => write!(f, "{e}"),
        }
    }
}

impl Error for FromBytesError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InteriorNul(e) => Some(e),
            Self::InvalidUtf8(e) => Some(e),
        }
    }
}

impl From<NulError> for FromBytesError {
    #[inline]
    fn from(e: NulError) -> Self {
        Self::InteriorNul(e)
    }
}

impl From<Utf8Error> for FromBytesError {
    #[inline]
    fn from(e: Utf8Error) -> Self {
        Self::InvalidUtf8(e)
    }
}

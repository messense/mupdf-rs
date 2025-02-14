use std::fmt;
use std::io;
use std::ffi::NulError;
use std::num::TryFromIntError;
use std::ptr::NonNull;

use mupdf_sys::*;

#[derive(Debug, Clone)]
pub struct MuPdfError {
    pub code: i32,
    pub message: String,
}

impl fmt::Display for MuPdfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MuPDF error, code: {}, message: {}",
            self.code, &self.message
        )
    }
}

impl std::error::Error for MuPdfError {}

/// # Safety
///
/// * `ptr` must point to a valid, well-aligned instance of [`mupdf_error_t`].
///
/// * The pointers stored in this [`mupdf_error_t`] must also be non-null, well-aligned, and point
///   to valid instances of what they claim to represent.
///
/// * The [`field@mupdf_error_t::message`] ptr in `ptr` must point to a null-terminated c-string
pub unsafe fn ffi_error(ptr: NonNull<mupdf_error_t>) -> MuPdfError {
    use std::ffi::CStr;

    // SAFETY: Upheld by caller
    let err = unsafe { *ptr.as_ptr() };
    let code = err.type_;
    let c_msg = err.message;

    // SAFETY: Upheld by caller
    let c_str = unsafe { CStr::from_ptr(c_msg) };
    let message = c_str.to_string_lossy().to_string();

    // SAFETY: Upheld by caller; if it's pointing to a valid instance then it can be dropped
    unsafe { mupdf_drop_error(ptr.as_ptr()) };
    MuPdfError { code, message }
}

macro_rules! ffi_try {
    ($func:ident($($arg:expr),+)) => ({
        use std::ptr;
        let mut err = ptr::null_mut();
        // SAFETY: Upheld by the caller of the macro
        let res = $func($($arg),+, (&mut err) as *mut *mut ::mupdf_sys::mupdf_error_t);
        if let Some(err) = ::core::ptr::NonNull::new(err) {
            // SAFETY: We're trusting the FFI call to provide us with a valid ptr if it is not
            // null.
            return Err($crate::ffi_error(err).into());
        }
        res
    });
}

#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    Io(io::Error),
    InvalidLanguage(String),
    InvalidPdfDocument,
    MuPdf(MuPdfError),
    Nul(NulError),
    IntConversion(TryFromIntError),
    UnexpectedNullPtr
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::Io(ref err) => err.fmt(f),
            Error::InvalidLanguage(ref lang) => write!(f, "invalid language {}", lang),
            Error::InvalidPdfDocument => write!(f, "invalid pdf document"),
            Error::MuPdf(ref err) => err.fmt(f),
            Error::Nul(ref err) => err.fmt(f),
            Error::IntConversion(ref err) => err.fmt(f),
            Error::UnexpectedNullPtr => write!(f, "An FFI function call returned a null ptr when we expected a non-null ptr")
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<MuPdfError> for Error {
    fn from(err: MuPdfError) -> Self {
        Self::MuPdf(err)
    }
}

impl From<NulError> for Error {
    fn from(err: NulError) -> Self {
        Self::Nul(err)
    }
}

impl From<TryFromIntError> for Error {
    fn from(value: TryFromIntError) -> Self {
        Self::IntConversion(value)
    }
}

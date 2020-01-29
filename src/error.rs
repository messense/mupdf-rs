use std::fmt;
use std::io;
use std::ffi::NulError;

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

pub unsafe fn ffi_error(err: *mut mupdf_error_t) -> MuPdfError {
    use std::ffi::CStr;

    let code = (*err).type_;
    let c_msg = (*err).message;
    let c_str = CStr::from_ptr(c_msg);
    let message = format!("{}", c_str.to_string_lossy());
    mupdf_drop_error(err);
    MuPdfError { code, message }
}

macro_rules! ffi_try {
    ($func:ident($($arg:expr),+)) => ({
        use std::ptr;
        let mut err = ptr::null_mut();
        let res = $func($($arg),+, &mut err);
        if !err.is_null() {
            return Err($crate::ffi_error(err).into());
        }
        res
    });
    ($func:ident()) => ({
        use std::ptr;
        let mut err = ptr::null_mut();
        let res = $func(&mut err);
        if !err.is_null() {
            return Err($crate::ffi_error(err).into());
        }
        res
    })
}

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    InvalidLanguage(String),
    MuPdf(MuPdfError),
    Nul(NulError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::Io(ref err) => err.fmt(f),
            Error::InvalidLanguage(ref lang) => write!(f, "invalid language {}", lang),
            Error::MuPdf(ref err) => err.fmt(f),
            Error::Nul(ref err) => err.fmt(f),
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

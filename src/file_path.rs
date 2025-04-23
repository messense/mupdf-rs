use std::mem::transmute;

use std::ffi::OsStr;

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(target_os = "wasi")]
use std::os::wasi::ffi::OsStrExt;

/// Path to a file, required to be UTF-8 on windows
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct FilePath(#[cfg(windows)] str, #[cfg(not(windows))] [u8]);

impl FilePath {
    pub fn new<P: AsRef<FilePath>>(p: &P) -> &Self {
        p.as_ref()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.as_ref()
    }
}

impl AsRef<FilePath> for str {
    fn as_ref(&self) -> &FilePath {
        #[cfg(windows)]
        // SAFETY: On windows FilePath is a str. As `self` is a str as well
        // and FilePath is repr(transparent) this is sound.
        unsafe {
            transmute(self)
        }

        #[cfg(not(windows))]
        self.as_bytes().as_ref()
    }
}

impl AsRef<FilePath> for String {
    fn as_ref(&self) -> &FilePath {
        self.as_str().as_ref()
    }
}

#[cfg(not(windows))]
impl AsRef<FilePath> for [u8] {
    fn as_ref(&self) -> &FilePath {
        // SAFETY: On non-windows FilePath is a byte slice. As `self` is a byte slice as well
        // and FilePath is repr(transparent) this is sound.
        unsafe { transmute(self) }
    }
}

#[cfg(any(unix, target_os = "wasi"))]
impl AsRef<FilePath> for OsStr {
    fn as_ref(&self) -> &FilePath {
        self.as_bytes().as_ref()
    }
}

#[cfg(any(unix, target_os = "wasi"))]
impl AsRef<FilePath> for std::path::Path {
    fn as_ref(&self) -> &FilePath {
        self.as_os_str().as_ref()
    }
}

impl AsRef<[u8]> for FilePath {
    fn as_ref(&self) -> &[u8] {
        #[cfg(windows)]
        {
            self.0.as_ref()
        }

        #[cfg(not(windows))]
        {
            &self.0
        }
    }
}

#[cfg(any(windows, unix, target_os = "wasi"))]
impl AsRef<OsStr> for FilePath {
    fn as_ref(&self) -> &OsStr {
        #[cfg(any(windows))]
        {
            self.0.as_ref()
        }

        #[cfg(any(unix, target_os = "wasi"))]
        {
            OsStr::from_bytes(&self.0)
        }
    }
}

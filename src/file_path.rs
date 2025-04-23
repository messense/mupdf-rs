use std::fmt::{self, Debug, Formatter};
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

impl Debug for FilePath {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        #[cfg(windows)]
        {
            Debug::fmt(&self.0, f)
        }

        #[cfg(not(windows))]
        {
            use std::fmt::Write;

            f.write_char('"')?;
            for chunk in self.0.utf8_chunks() {
                write!(f, "{}", chunk.valid().escape_debug())?;
                write!(f, "{}", chunk.invalid().escape_ascii())?;
            }
            f.write_char('"')
        }
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

#[cfg(any(windows, unix, target_os = "wasi"))]
impl AsRef<std::path::Path> for FilePath {
    fn as_ref(&self) -> &std::path::Path {
        std::path::Path::new(self)
    }
}

#[cfg(test)]
mod test {
    use super::FilePath;

    fn assert_debug<P: AsRef<FilePath> + ?Sized>(name: &P, debug: &str) {
        assert_eq!(format!("{:?}", name.as_ref()), debug);
    }

    #[test]
    fn test_debug() {
        assert_debug("abc def.txt", r#""abc def.txt""#);
        assert_debug("path/to/a/file.pdf", r#""path/to/a/file.pdf""#);
        assert_debug("bell\x0b.wav", r#""bell\u{b}.wav""#);

        #[cfg(not(windows))]
        assert_debug(
            b"a non utf-\x9d path".as_slice(),
            r#""a non utf-\x9d path""#,
        );
    }
}

use std::fmt::{self, Debug, Formatter, Write};
use std::mem::transmute;

#[cfg(any(windows, unix, target_os = "wasi"))]
use std::{ffi::OsStr, path::Path};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(target_os = "wasi")]
use std::os::wasi::ffi::OsStrExt;

use crate::Error;

/// Path to a file, required to be UTF-8 on windows
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct FilePath(#[cfg(windows)] str, #[cfg(not(windows))] [u8]);

impl FilePath {
    pub fn new<P: AsRef<FilePath> + ?Sized>(p: &P) -> &Self {
        p.as_ref()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.as_ref()
    }
}

impl Debug for FilePath {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_char('"')?;

        #[cfg(windows)]
        write!(f, "{}", self.0.escape_debug())?;

        #[cfg(not(windows))]
        for chunk in self.0.utf8_chunks() {
            write!(f, "{}", chunk.valid().escape_debug())?;
            write!(f, "{}", chunk.invalid().escape_ascii())?;
        }

        f.write_char('"')
    }
}

impl AsRef<FilePath> for str {
    fn as_ref(&self) -> &FilePath {
        #[cfg(windows)]
        // SAFETY: On windows FilePath is a str. As `self` is a str as well
        // and FilePath is repr(transparent) this is sound.
        unsafe {
            transmute::<&str, &FilePath>(self)
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
        unsafe { transmute::<&[u8], &FilePath>(self) }
    }
}

#[cfg(any(unix, target_os = "wasi"))]
impl AsRef<FilePath> for OsStr {
    fn as_ref(&self) -> &FilePath {
        self.as_bytes().as_ref()
    }
}

#[cfg(any(unix, target_os = "wasi"))]
impl AsRef<FilePath> for Path {
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
        #[cfg(windows)]
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
impl AsRef<Path> for FilePath {
    fn as_ref(&self) -> &Path {
        Path::new(self)
    }
}

impl<'a> TryFrom<&'a OsStr> for &'a FilePath {
    type Error = Error;
    fn try_from(value: &'a OsStr) -> Result<Self, Self::Error> {
        #[cfg(not(any(unix, target_os = "wasi")))]
        {
            Ok(value.to_str().ok_or(Error::InvalidUtf8)?.as_ref())
        }

        #[cfg(any(unix, target_os = "wasi"))]
        {
            Ok(value.as_ref())
        }
    }
}

impl<'a> TryFrom<&'a Path> for &'a FilePath {
    type Error = Error;
    fn try_from(value: &'a Path) -> Result<Self, Self::Error> {
        value.as_os_str().try_into()
    }
}

#[cfg(test)]
mod test {
    use std::ffi::OsStr;

    use super::FilePath;

    #[cfg(unix)]
    use std::os::unix::ffi::OsStrExt;
    #[cfg(target_os = "wasi")]
    use std::os::wasi::ffi::OsStrExt;

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

    #[test]
    fn test_try_from() {
        assert_eq!(
            <&FilePath>::try_from(OsStr::new("document.pdf")).unwrap(),
            FilePath::new("document.pdf")
        );

        let non_utf8_path = <&FilePath>::try_from(OsStr::from_bytes(b"non utf-8 \x9d path"));

        #[cfg(any(unix, target_os = "wasi"))]
        assert_eq!(
            non_utf8_path.unwrap(),
            FilePath::new(b"non utf-8 \x9d path".as_slice())
        );

        #[cfg(not(any(unix, target_os = "wasi")))]
        assert!(non_utf8_path.is_err());
    }
}

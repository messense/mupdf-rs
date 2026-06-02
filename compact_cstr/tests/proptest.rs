//! These tests exercise the byte-layout / transmute reasoning in `repr.rs`
//! by constructing values from random inputs and asserting that what comes
//! out of the various accessors matches what went in. They are designed to
//! be fast and deterministic enough to run under Miri.

use compact_cstr::{CompactCBytes, CompactCString, FromBytesError, NulError, INLINE_TOTAL};
use core::num::NonZero;
use proptest::prelude::*;
use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::ffi::{CStr, CString};
use std::hash::{Hash, Hasher};

/// Strategy for `Vec<u8>` that may include zero bytes.
fn arbitrary_bytes() -> impl Strategy<Value = Vec<u8>> {
    proptest::collection::vec(any::<u8>(), 0..200)
}

/// Strategy for `Vec<u8>` guaranteed to contain no zero byte.
fn nul_free_bytes() -> impl Strategy<Value = Vec<u8>> {
    proptest::collection::vec(1u8..=255, 0..200)
}

/// Strategy for NUL-free `Vec<u8>` whose length always spills to heap.
fn heap_sized_nul_free_bytes() -> impl Strategy<Value = Vec<u8>> {
    proptest::collection::vec(1u8..=255, INLINE_TOTAL..200)
}

/// Strategy for valid UTF-8 strings (which may contain interior NULs).
fn arbitrary_string() -> impl Strategy<Value = String> {
    ".{0,200}".prop_map(String::from)
}

/// Strategy for valid UTF-8 strings guaranteed to contain no NUL byte.
fn nul_free_string() -> impl Strategy<Value = String> {
    r"[^\x00]{0,200}".prop_map(String::from)
}

fn hash_of<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut h = DefaultHasher::new();
    value.hash(&mut h);
    h.finish()
}

/// Number of proptest cases per test. Under Miri every operation is interpreted, so we drop the
/// count drastically; the goal under Miri is to exercise each code path a few times for UB
/// detection, not to do thorough property coverage (that's what the regular `cargo test` run is
/// for).
#[cfg(miri)]
const CASES: u32 = 2;
#[cfg(not(miri))]
const CASES: u32 = 64;

proptest! {
    #![proptest_config(ProptestConfig {
        cases: CASES,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    /// `CompactCBytes::try_from(&[u8])` accepts NUL-free input unchanged.
    #[test]
    fn compact_c_bytes_nul_free(bytes in nul_free_bytes()) {
        let v = CompactCBytes::try_from(bytes.as_slice()).unwrap();
        prop_assert_eq!(v.as_slice(), bytes.as_slice());
        prop_assert_eq!(v.as_cstr().to_bytes(), bytes.as_slice());
        prop_assert_eq!(v.len(), bytes.len());
        prop_assert_eq!(v.is_empty(), bytes.is_empty());
        prop_assert_eq!(v.is_heap(), bytes.len() >= INLINE_TOTAL);
        prop_assert_eq!(v.is_inline(), !v.is_heap());
    }

    /// Cloning is always a faithful copy (inline or heap).
    #[test]
    fn compact_c_bytes_clone(bytes in nul_free_bytes()) {
        let v = CompactCBytes::try_from(bytes.as_slice()).unwrap();
        let c = v.clone();
        prop_assert_eq!(v.as_slice(), c.as_slice());
        prop_assert_eq!(v.is_heap(), c.is_heap());
        // Drop one then check the other still reads correctly.
        drop(v);
        prop_assert_eq!(c.as_slice(), bytes.as_slice());
    }

    /// Bytes containing an interior NUL are rejected, with the correct position.
    #[test]
    fn interior_nul_rejected(prefix in nul_free_bytes(), suffix in nul_free_bytes()) {
        let mut bytes = prefix.clone();
        bytes.push(0);
        bytes.extend_from_slice(&suffix);

        let err: NulError = CompactCBytes::try_from(bytes.as_slice()).unwrap_err();
        prop_assert_eq!(err.position(), prefix.len());
    }

    /// Arbitrary byte input is accepted unchanged if it contains no NUL bytes;
    /// otherwise it is rejected at the position of the first NUL byte.
    #[test]
    fn compact_c_bytes_total(bytes in arbitrary_bytes()) {
        match CompactCBytes::try_from(bytes.as_slice()) {
            Ok(v) => {
                prop_assert!(!bytes.contains(&0));
                prop_assert_eq!(v.as_slice(), bytes.as_slice());
            }
            Err(e) => {
                let nul_pos = bytes
                    .iter()
                    .position(|&b| b == 0)
                    .expect("error implies input contains a NUL byte");

                prop_assert_eq!(e.position(), nul_pos);
            }
        }
    }

    /// `&[NonZero<u8>]` is the safe-by-construction path.
    #[test]
    fn nonzero_slice(bytes in nul_free_bytes()) {
        let nz: Vec<NonZero<u8>> =
            bytes.iter().map(|&b| NonZero::new(b).unwrap()).collect();
        let v = CompactCBytes::from(nz.as_slice());
        prop_assert_eq!(v.as_slice(), bytes.as_slice());
        prop_assert_eq!(v.as_cstr().to_bytes(), bytes.as_slice());
    }

    /// NUL-free strings are accepted unchanged. Strings containing a NUL byte are rejected.
    #[test]
    fn compact_c_string(s in arbitrary_string()) {
        let nul_pos = s.as_bytes().iter().position(|&b| b == 0);
        match CompactCString::try_from(s.as_str()) {
            Ok(p) => {
                prop_assert_eq!(nul_pos, None);
                prop_assert_eq!(p.as_str(), s.as_str());
                prop_assert_eq!(p.as_bytes(), s.as_bytes());
                prop_assert_eq!(p.as_cstr().to_bytes(), s.as_bytes());
            }
            Err(e) => prop_assert_eq!(Some(e.position()), nul_pos),
        }
    }

    /// NUL-free bytes are converted to `CompactCString` only if they are valid UTF-8.
    #[test]
    fn compact_c_string_from_bytes(bytes in nul_free_bytes()) {
        let v = CompactCBytes::try_from(bytes.as_slice()).unwrap();
        let utf8_ok = core::str::from_utf8(&bytes).is_ok();
        match CompactCString::try_from(v) {
            Ok(p) => {
                prop_assert!(utf8_ok);
                prop_assert_eq!(p.as_bytes(), bytes.as_slice());
            }
            Err(_) => prop_assert!(!utf8_ok),
        }
    }

    #[test]
    fn try_inline_bytes(bytes in proptest::collection::vec(1u8..=255, 0..30)) {
        match CompactCBytes::try_inline(bytes.as_slice()) {
            Some(v) => {
                prop_assert!(bytes.len() < INLINE_TOTAL);
                prop_assert!(v.is_inline());
                prop_assert_eq!(v.as_slice(), bytes.as_slice());
                prop_assert_eq!(v.as_cstr().to_bytes(), bytes.as_slice());
            }
            None => prop_assert!(bytes.len() >= INLINE_TOTAL),
        }
    }

    #[test]
    fn try_inline_bytes_rejects_nul(
        prefix in proptest::collection::vec(1u8..=255, 0..15),
        suffix in proptest::collection::vec(1u8..=255, 0..15),
    ) {
        let mut bytes = prefix;
        bytes.push(0);
        bytes.extend_from_slice(&suffix);

        prop_assert!(CompactCBytes::try_inline(bytes.as_slice()).is_none());
    }

    #[test]
    fn try_inline_string(s in r"[^\x00]{0,30}") {
        match CompactCString::try_inline(&s) {
            Some(v) => {
                prop_assert!(s.len() < INLINE_TOTAL);
                prop_assert!(v.is_inline());
                prop_assert_eq!(v.as_str(), s.as_str());
                prop_assert_eq!(v.as_bytes(), s.as_bytes());
            }
            None => prop_assert!(s.len() >= INLINE_TOTAL),
        }
    }

    #[test]
    fn cbytes_from_cstr(bytes in nul_free_bytes()) {
        let cstring = CString::new(bytes.clone()).unwrap();
        let v = CompactCBytes::from(cstring.as_c_str());

        prop_assert_eq!(v.as_slice(), bytes.as_slice());
        prop_assert_eq!(v.as_cstr().to_bytes_with_nul(), cstring.as_bytes_with_nul());
        prop_assert_eq!(v.is_heap(), bytes.len() >= INLINE_TOTAL);
    }

    #[test]
    fn cstring_from_cstr(s in nul_free_string()) {
        let cstring = CString::new(s.clone()).unwrap();
        let v = CompactCString::try_from(cstring.as_c_str()).unwrap();

        prop_assert_eq!(v.as_str(), s.as_str());
        prop_assert_eq!(v.as_cstr().to_bytes_with_nul(), cstring.as_bytes_with_nul());
    }

    #[test]
    fn cstring_from_cstring(s in nul_free_string()) {
        let cstring = CString::new(s.clone()).unwrap();
        let v = CompactCString::try_from(&cstring).unwrap();

        prop_assert_eq!(v.as_str(), s.as_str());
    }

    #[test]
    fn from_str_matches_try_from(s in arbitrary_string()) {
        use core::str::FromStr;

        match (
            CompactCString::try_from(s.as_str()),
            CompactCString::from_str(s.as_str()),
        ) {
            (Ok(a), Ok(b)) => prop_assert_eq!(a.as_str(), b.as_str()),
            (Err(a), Err(b)) => prop_assert_eq!(a.position(), b.position()),
            _ => prop_assert!(false, "FromStr and TryFrom<&str> disagreed"),
        }
    }

    #[test]
    fn ord_matches_payload(a in nul_free_bytes(), b in nul_free_bytes()) {
        let ca = CompactCBytes::try_from(a.as_slice()).unwrap();
        let cb = CompactCBytes::try_from(b.as_slice()).unwrap();

        prop_assert_eq!(ca.cmp(&cb), a.as_slice().cmp(b.as_slice()));
        prop_assert_eq!(ca.partial_cmp(&cb), a.as_slice().partial_cmp(b.as_slice()));
    }

    #[test]
    fn string_ord_matches_payload(a in nul_free_string(), b in nul_free_string()) {
        let ca = CompactCString::try_from(a.as_str()).unwrap();
        let cb = CompactCString::try_from(b.as_str()).unwrap();

        prop_assert_eq!(ca.cmp(&cb), a.as_str().cmp(b.as_str()));
        prop_assert_eq!(ca.partial_cmp(&cb), a.as_str().partial_cmp(b.as_str()));
    }

    #[test]
    fn hash_bytes_matches_slice(bytes in nul_free_bytes()) {
        let v = CompactCBytes::try_from(bytes.as_slice()).unwrap();

        prop_assert_eq!(hash_of(&v), hash_of(bytes.as_slice()));
    }

    #[test]
    fn hash_string_is_deterministic(s in nul_free_string()) {
        let a = CompactCString::try_from(s.as_str()).unwrap();
        let b = CompactCString::try_from(s.as_str()).unwrap();

        prop_assert_eq!(hash_of(&a), hash_of(&b));
    }

    #[test]
    fn bytes_partial_eq_matrix(a in nul_free_bytes(), b in nul_free_bytes()) {
        let ca = CompactCBytes::try_from(a.as_slice()).unwrap();
        let expected = a == b;

        prop_assert_eq!(ca == b, expected);
        prop_assert_eq!(b == ca, expected);
        prop_assert_eq!(ca == b.as_slice(), expected);
        prop_assert_eq!(b.as_slice() == ca, expected);
        prop_assert_eq!(&ca == &b, expected);
    }

    #[test]
    fn string_partial_eq_matrix(a in nul_free_string(), b in nul_free_string()) {
        let ca = CompactCString::try_from(a.as_str()).unwrap();
        let cow: Cow<'_, str> = Cow::Borrowed(b.as_str());
        let expected = a == b;

        prop_assert_eq!(ca == b, expected);
        prop_assert_eq!(b == ca, expected);
        prop_assert_eq!(ca == b.as_str(), expected);
        prop_assert_eq!(b.as_str() == ca, expected);
        prop_assert_eq!(ca == cow, expected);
        prop_assert_eq!(Cow::Borrowed(b.as_str()) == ca, expected);
    }

    #[test]
    fn into_bytes_preserves_payload(s in nul_free_string()) {
        let cs = CompactCString::try_from(s.as_str()).unwrap();
        let bytes = cs.into_bytes();

        prop_assert_eq!(bytes.as_slice(), s.as_bytes());
    }

    #[test]
    fn into_string_preserves_payload(s in nul_free_string()) {
        let cs = CompactCString::try_from(s.as_str()).unwrap();

        prop_assert_eq!(cs.into_string(), s);
    }

    #[test]
    fn as_ptr_is_nul_terminated(s in nul_free_string()) {
        let cs = CompactCString::try_from(s.as_str()).unwrap();

        // SAFETY: `cs` is alive, and `CompactCString` guarantees a valid NUL-terminated buffer.
        let read = unsafe { CStr::from_ptr(cs.as_ptr()) };

        prop_assert_eq!(read.to_bytes(), s.as_bytes());
    }

    #[test]
    fn cstring_try_from_bytes_total(bytes in arbitrary_bytes()) {
        let nul_pos = bytes.iter().position(|&b| b == 0);
        let utf8_err = core::str::from_utf8(&bytes).err();

        match CompactCString::try_from(bytes.as_slice()) {
            Ok(cs) => {
                prop_assert_eq!(nul_pos, None);
                prop_assert!(utf8_err.is_none());
                prop_assert_eq!(cs.as_bytes(), bytes.as_slice());
            }
            Err(FromBytesError::InteriorNul(e)) => {
                prop_assert_eq!(Some(e.position()), nul_pos);
            }
            Err(FromBytesError::InvalidUtf8(e)) => {
                prop_assert_eq!(nul_pos, None);
                prop_assert_eq!(Some(e), utf8_err);
            }
        }
    }

    #[test]
    fn heap_clone_outlives_original(bytes in heap_sized_nul_free_bytes()) {
        let v = CompactCBytes::try_from(bytes.as_slice()).unwrap();
        prop_assert!(v.is_heap());

        let c = v.clone();
        prop_assert!(c.is_heap());

        drop(v);

        prop_assert_eq!(c.as_slice(), bytes.as_slice());
        prop_assert_eq!(c.as_cstr().to_bytes(), bytes.as_slice());
    }

    #[test]
    fn cstring_rejects_injected_nul(
        prefix in nul_free_string(),
        suffix in nul_free_string(),
    ) {
        let mut s = prefix.clone();
        s.push('\0');
        s.push_str(&suffix);

        let err = CompactCString::try_from(s.as_str()).unwrap_err();

        prop_assert_eq!(err.position(), prefix.len());
    }
}

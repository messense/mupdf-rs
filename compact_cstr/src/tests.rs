use super::*;
use core::error::Error;
use core::num::NonZero;
use std::borrow::Cow;
use std::ffi::{CStr, CString};

#[test]
fn compact_c_string_layout_boundaries() {
    let default = CompactCString::default();
    assert_eq!(default.len(), 0);
    assert_eq!(default.as_str(), "");
    assert_eq!(default.as_bytes(), b"");
    let cstring = CString::new("").unwrap();
    assert_eq!(
        default.as_cstr().to_bytes_with_nul(),
        cstring.to_bytes_with_nul()
    );

    for len in [0, 3, 23, 24, 100] {
        let s = "x".repeat(len);
        let cstring = CString::new(s.clone()).unwrap();
        let value = CompactCString::try_from(cstring.as_c_str()).unwrap();

        assert_eq!(value.len(), len);
        assert_eq!(value.as_str(), s);
        assert_eq!(value.as_bytes(), s.as_bytes());
        assert_eq!(
            value.as_cstr().to_bytes_with_nul(),
            cstring.to_bytes_with_nul()
        );

        if len < 24 {
            assert!(value.is_inline());
        } else {
            assert!(value.is_heap());
        }
    }
}

#[test]
fn compact_bytes_layout_boundaries() {
    let default = CompactCBytes::default();
    assert_eq!(default.len(), 0);
    assert_eq!(default, b"");
    let cstring = CString::new("").unwrap();
    assert_eq!(
        default.as_cstr().to_bytes_with_nul(),
        cstring.to_bytes_with_nul()
    );

    for len in [0, 1, 22, 23, 24, 25, 100] {
        let bytes = vec![b'a'; len];
        let value = CompactCBytes::try_from(bytes.as_slice()).unwrap();

        assert_eq!(value.len(), len);
        assert_eq!(value.as_slice(), bytes.as_slice());
        assert_eq!(value.as_cstr().to_bytes(), bytes.as_slice());

        if len < 24 {
            assert!(value.is_inline());
        } else {
            assert!(value.is_heap());
        }
    }
}

#[test]
fn compact_bytes_equality() {
    let short = CompactCBytes::try_from("hello".as_bytes()).unwrap();
    let long = CompactCBytes::try_from([b'x'; 30].as_slice()).unwrap();
    let other = CompactCBytes::try_from("world".as_bytes()).unwrap();

    assert!(short.is_inline());
    assert!(long.is_heap());

    assert_eq!(short, short.clone());
    assert_eq!(long, long.clone());
    assert_ne!(short, other);

    assert_eq!(short, b"hello".as_slice());
    assert_ne!(short, b"world".as_slice());

    assert_eq!(short, b"hello".to_vec());
    assert_ne!(short, b"world".to_vec());

    // CompactCBytes == &str  (str: AsRef<[u8]>)
    assert_eq!(short, "hello");
    assert_ne!(short, "world");

    assert_eq!(&short, short.clone());
    assert_ne!(&short, other.clone());

    assert_eq!(b"hello".to_vec(), short);
    assert_ne!(b"world".to_vec(), short);

    assert_eq!(b"hello".to_vec(), &short);
    assert_ne!(b"world".to_vec(), &short);

    let v = b"hello".to_vec();
    assert_eq!(&v, short);

    let hello_bytes: &[u8] = b"hello";
    assert_eq!(*hello_bytes, short);

    assert_eq!(*hello_bytes, &short);

    assert_eq!(hello_bytes, short);
    assert_ne!(b"world".as_slice(), short);

    let s_u8: &&[u8] = &b"hello".as_slice();
    assert_eq!(s_u8, short);
    assert_eq!(short, s_u8);

    let s_u8: &&&[u8] = &&b"hello".as_slice();
    assert_eq!(short, s_u8);

    let long_bytes = vec![b'x'; 30];
    assert_eq!(long, long_bytes);
    assert_eq!(long_bytes, long);
    assert_eq!(&long, long.clone());
    assert_eq!(long_bytes.as_slice(), long);
}

#[test]
#[allow(clippy::needless_borrow)]
fn compact_c_string_equality() {
    let short = CompactCString::try_from("hello").unwrap();
    let long = CompactCString::try_from(&"y".repeat(30)).unwrap();
    let other = CompactCString::try_from("world").unwrap();

    assert!(short.is_inline());
    assert!(long.is_heap());

    assert_eq!(short, short.clone());
    assert_eq!(long, long.clone());
    assert_ne!(short, other);

    assert_eq!(short, "hello");
    assert_ne!(short, "world");

    assert_eq!(short, String::from("hello"));
    assert_ne!(short, String::from("world"));

    let cow_borrowed: Cow<str> = Cow::Borrowed("hello");
    let cow_owned: Cow<str> = Cow::Owned(String::from("hello"));
    assert_eq!(short, cow_borrowed);
    assert_eq!(short, cow_owned);

    assert_eq!(&short, short.clone());
    assert_ne!(&short, other.clone());

    assert_eq!(String::from("hello"), short);
    assert_ne!(String::from("world"), short);

    assert_eq!(String::from("hello"), &short);
    assert_ne!(String::from("world"), &short);

    let s = String::from("hello");
    assert_eq!(&s, short);

    assert_eq!(*"hello", short);

    assert_eq!(*"hello", &short);

    assert_eq!("hello", short);
    assert_ne!("world", short);

    let r: &&str = &&"hello";
    assert_eq!(r, short);

    assert_eq!(cow_borrowed, short);
    assert_eq!(cow_owned, short);

    assert_eq!(&cow_borrowed, short);

    assert_eq!(&short, String::from("hello"));

    assert_eq!(&short, cow_borrowed);

    let long_str = "y".repeat(30);
    assert_eq!(long, long_str.as_str());
    assert_eq!(long_str, long);
    assert_eq!(&long, long.clone());
}

#[test]
fn rejects_interior_nul() {
    assert!(CompactCBytes::try_from(b"a\0b".as_slice()).is_err());
    assert!(CompactCString::try_from("a\0b").is_err());

    // Position is reported.
    let err = CompactCBytes::try_from(b"abc\0def".as_slice()).unwrap_err();
    assert_eq!(err.position(), 3);
}

#[test]
fn infallible_byte_conversions() {
    let bytes: Vec<NonZero<u8>> = b"hello".iter().map(|&b| NonZero::new(b).unwrap()).collect();

    let value = CompactCBytes::from(bytes.as_slice());
    assert_eq!(value.as_slice(), b"hello");
    assert_eq!(value.as_cstr().to_bytes(), b"hello");

    let cstring = CString::new("world").unwrap();
    let value = CompactCBytes::from(cstring.as_c_str());
    assert_eq!(value.as_slice(), b"world");
    assert_eq!(value.as_cstr().to_bytes(), b"world");
}

#[test]
fn inline_constructors() {
    assert!(CompactCBytes::try_inline(b"").is_some());
    assert!(CompactCBytes::try_inline(&[b'x'; 22]).is_some());
    assert!(CompactCBytes::try_inline(&[b'x'; 23]).is_some());
    assert!(CompactCBytes::try_inline(&[b'x'; 24]).is_none());

    assert!(CompactCBytes::try_inline(b"\0").is_none());
    assert!(CompactCBytes::try_inline(b"abc\0def").is_none());
    assert!(CompactCBytes::try_inline(b"abc\0").is_none());

    let v = CompactCBytes::try_inline(b"hello").unwrap();
    assert!(v.is_inline());
    assert_eq!(v.as_slice(), b"hello");

    assert_eq!(CompactCBytes::new_inline(b"hello").as_slice(), b"hello");
    assert_eq!(CompactCString::new_inline("hello").as_str(), "hello");
    assert_eq!(
        CompactCString::try_inline("hello").unwrap().as_str(),
        "hello"
    );
}

#[test]
fn owned_input_conversions() {
    let vec = vec![1, 2, 3];
    let bytes: CompactCBytes = (&vec).try_into().unwrap();
    assert_eq!(bytes.as_slice(), &[1, 2, 3]);

    let vec = vec![NonZero::new(1).unwrap(), NonZero::new(2).unwrap()];
    let bytes: CompactCBytes = (&vec).into();
    assert_eq!(bytes.as_slice(), &[1u8, 2]);

    let string = String::from("hello");
    let cs: CompactCString = (&string).try_into().unwrap();
    assert_eq!(cs.as_str(), "hello");

    let cstring = CString::new("hello").unwrap();
    let cs: CompactCString = (&cstring).try_into().unwrap();
    assert_eq!(cs.as_str(), "hello");
}

#[test]
fn from_str_and_string_conversions() {
    use core::str::FromStr;

    let cs = CompactCString::from_str("world").unwrap();
    assert_eq!(cs.as_str(), "world");

    assert!(CompactCString::from_str("a\0b").is_err());

    let cs = CompactCString::try_from("hello").unwrap();

    let string: String = cs.clone().into();
    assert_eq!(string, "hello");

    let str: &str = (&cs).into();
    assert_eq!(str, "hello");
}

#[test]
fn into_and_as_ptr() {
    let cs = CompactCString::new_inline("payload");

    assert_eq!(cs.clone().into_string(), "payload");
    assert_eq!(cs.clone().into_bytes().as_slice(), b"payload");

    let read = unsafe { CStr::from_ptr(cs.as_ptr()) };
    assert_eq!(read.to_bytes(), b"payload");

    let string = "y".repeat(40);
    let cs = CompactCString::try_from(string.as_str()).unwrap();
    assert!(cs.is_heap());

    let read = unsafe { CStr::from_ptr(cs.as_ptr()) };
    assert_eq!(read.to_bytes(), string.as_bytes());
}

#[test]
fn as_ref_and_borrow_views() {
    use std::borrow::Borrow;

    let cb = CompactCBytes::new_inline(b"hello");

    assert_eq!(AsRef::<[u8]>::as_ref(&cb), b"hello");
    assert_eq!(AsRef::<CStr>::as_ref(&cb).to_bytes(), b"hello");
    assert_eq!(Borrow::<[u8]>::borrow(&cb), b"hello");

    let cs = CompactCString::new_inline("hello");

    assert_eq!(AsRef::<[u8]>::as_ref(&cs), b"hello");
    assert_eq!(AsRef::<str>::as_ref(&cs), "hello");
    assert_eq!(AsRef::<CStr>::as_ref(&cs).to_bytes(), b"hello");
    assert_eq!(Borrow::<str>::borrow(&cs), "hello");
}

#[test]
fn display_and_debug() {
    let bytes = b"hello";
    let cb = CompactCBytes::new_inline(bytes.as_slice());
    let cs = CompactCString::new_inline("hello");

    assert_eq!(format!("{cs}"), "hello");

    let expected_cb = format!("CompactCBytes({bytes:?})");
    let expected_cs = format!("CompactCString({expected_cb})");

    assert_eq!(format!("{cb:?}"), expected_cb);
    assert_eq!(format!("{cs:?}"), expected_cs);
}

#[test]
fn error_variants_and_sources() {
    use crate::NulError;

    let nul_err = CompactCBytes::try_from(b"a\0b".as_slice()).unwrap_err();
    assert_eq!(format!("{nul_err}"), "interior NUL byte at position 1");

    let nul_via_bytes = CompactCString::try_from(b"a\0b".as_slice()).unwrap_err();
    let err = NulError { position: 1 };
    assert_eq!(nul_via_bytes, FromBytesError::InteriorNul(err));

    let source = Error::source(&nul_via_bytes).unwrap();
    let source = source.downcast_ref::<NulError>().unwrap();
    assert_eq!(source.position(), 1);

    let bad_utf8 = [0xffu8, 0xfe];
    let utf8_err = CompactCString::try_from(bad_utf8.as_slice()).unwrap_err();
    assert!(matches!(utf8_err, FromBytesError::InvalidUtf8(_)));
    assert!(Error::source(&utf8_err).is_some());
}

#[test]
fn unsafe_constructors() {
    let cs_raw = CString::new("raw").unwrap();

    let v = unsafe { CompactCBytes::from_raw_c_unchecked(cs_raw.as_ptr()) };
    assert_eq!(v.as_slice(), b"raw");

    let v = unsafe { CompactCBytes::from_bytes_unchecked(b"unchecked") };
    assert_eq!(v.as_slice(), b"unchecked");

    let s = unsafe { CompactCString::from_bytes_unchecked(b"utf8raw") };
    assert_eq!(s.as_str(), "utf8raw");

    let bytes = CompactCBytes::try_from(b"valid".as_slice()).unwrap();

    let s = unsafe { CompactCString::from_utf8_unchecked(bytes) };
    assert_eq!(s.as_str(), "valid");

    let s = unsafe { CompactCString::from_utf8_cstr_unchecked(cs_raw.as_c_str()) };
    assert_eq!(s.as_str(), "raw");
}

#[test]
fn heap_clone_drop_order() {
    let big = "x".repeat(40);
    let v = CompactCBytes::try_from(big.as_bytes()).unwrap();

    assert!(v.is_heap());

    let c1 = v.clone();
    let c2 = c1.clone();

    drop(v);
    assert_eq!(c1.as_slice(), big.as_bytes());
    assert_eq!(c2.as_slice(), big.as_bytes());

    drop(c1);
    assert_eq!(c2.as_slice(), big.as_bytes());
    drop(c2);
}

#[test]
fn test_in_collections() {
    use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

    let bytes_inline: &[u8] = b"hello";
    let bytes_heap = vec![b'x'; 30];

    let mut map = HashMap::new();
    map.insert(CompactCBytes::try_from(bytes_inline).unwrap(), 42);
    map.insert(CompactCBytes::try_from(bytes_heap.as_slice()).unwrap(), 7);

    assert_eq!(map.get(bytes_inline), Some(&42));
    assert_eq!(map.get(bytes_heap.as_slice()), Some(&7));

    let mut set = HashSet::new();
    set.insert(CompactCBytes::try_from(bytes_inline).unwrap());
    set.insert(CompactCBytes::try_from(bytes_heap.as_slice()).unwrap());

    assert!(set.contains(bytes_inline));
    assert!(set.contains(bytes_heap.as_slice()));

    let mut tree_map = BTreeMap::new();
    tree_map.insert(CompactCBytes::try_from(bytes_inline).unwrap(), 42);
    tree_map.insert(CompactCBytes::try_from(bytes_heap.as_slice()).unwrap(), 7);

    assert_eq!(tree_map.get(bytes_inline), Some(&42));
    assert_eq!(tree_map.get(bytes_heap.as_slice()), Some(&7));

    let mut tree_set: BTreeSet<CompactCBytes> = BTreeSet::new();
    tree_set.insert(CompactCBytes::try_from(bytes_inline).unwrap());
    tree_set.insert(CompactCBytes::try_from(bytes_heap.as_slice()).unwrap());

    assert!(tree_set.contains(bytes_inline));
    assert!(tree_set.contains(bytes_heap.as_slice()));

    let str_inline = "hello";
    let str_heap = "y".repeat(30);

    let mut map = HashMap::new();
    map.insert(CompactCString::try_from(str_inline).unwrap(), 42);
    map.insert(CompactCString::try_from(str_heap.as_str()).unwrap(), 7);

    assert_eq!(map.get(str_inline), Some(&42));
    assert_eq!(map.get(str_heap.as_str()), Some(&7));

    let mut set = HashSet::new();
    set.insert(CompactCString::try_from(str_inline).unwrap());
    set.insert(CompactCString::try_from(str_heap.as_str()).unwrap());

    assert!(set.contains(str_inline));
    assert!(set.contains(str_heap.as_str()));

    let mut tree_map = BTreeMap::new();
    tree_map.insert(CompactCString::try_from(str_inline).unwrap(), 42);
    tree_map.insert(CompactCString::try_from(str_heap.as_str()).unwrap(), 7);

    assert_eq!(tree_map.get(str_inline), Some(&42));
    assert_eq!(tree_map.get(str_heap.as_str()), Some(&7));

    let mut tree_set = BTreeSet::new();
    tree_set.insert(CompactCString::try_from(str_inline).unwrap());
    tree_set.insert(CompactCString::try_from(str_heap.as_str()).unwrap());

    assert!(tree_set.contains(str_inline));
    assert!(tree_set.contains(str_heap.as_str()));
}

#[test]
fn new_inline_panics() {
    use std::panic::catch_unwind;

    assert!(catch_unwind(|| CompactCBytes::new_inline(&[b'x'; 24])).is_err());
    assert!(catch_unwind(|| CompactCBytes::new_inline(b"abc\0def")).is_err());

    assert!(catch_unwind(|| CompactCString::new_inline("xxxxxxxxxxxxxxxxxxxxxxxxxx")).is_err());
    assert!(catch_unwind(|| CompactCString::new_inline("a\0b")).is_err());
}

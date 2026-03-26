//! Owned, NUL-terminated byte/string types for safe MuPDF FFI.
//!
//! [`CompactCBytes`] and [`CompactCString`] use a compact 24-byte representation: up to
//! 23 payload bytes are stored inline, while larger payloads spill to a
//! reference-counted [`ecow::EcoVec<u8>`].
//!
//! Both types maintain a trailing NUL for allocation-free `&CStr` access and
//! reject payload NUL bytes during checked construction. [`CompactCString`] also
//! guarantees valid UTF-8.
//!
//! Their layout supports niche optimization, so [`Option<CompactCBytes>`] and
//! [`Option<CompactCString>`] have no extra size overhead.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(rustdoc::broken_intra_doc_links)]

use core::mem::{align_of, size_of};

mod error;
mod repr;

/// Generates `PartialEq` implementations.
macro_rules! impl_partial_eq {
    ($(impl PartialEq<$rhs:ty> for $lhs:ty { |$self_:ident, $other:ident| $body:expr })*) => {$(
        impl PartialEq<$rhs> for $lhs {
            #[inline]
            fn eq(&$self_, $other: &$rhs) -> bool {
                $body
            }
        }
    )*};
}

pub mod string;
pub mod vector;

#[cfg(test)]
mod tests;

pub use error::{FromBytesError, NulError};
pub use repr::INLINE_TOTAL;
pub use string::CompactCString;
pub use vector::CompactCBytes;

use repr::Repr;

// Size guarantees
const _: () = assert!(size_of::<CompactCString>() == INLINE_TOTAL);
const _: () = assert!(size_of::<CompactCString>() == size_of::<CompactCBytes>());

// Alignment guarantees
const _: () = assert!(align_of::<CompactCString>() == align_of::<Repr>());
const _: () = assert!(align_of::<CompactCString>() == align_of::<CompactCBytes>());

// Niche optimization guarantees
const _: () = assert!(size_of::<Option<CompactCString>>() == size_of::<CompactCString>());
const _: () = assert!(size_of::<Result<CompactCString, ()>>() == size_of::<CompactCString>());
const _: () = assert!(size_of::<Option<CompactCBytes>>() == size_of::<CompactCBytes>());
const _: () = assert!(size_of::<Result<CompactCBytes, ()>>() == size_of::<CompactCBytes>());

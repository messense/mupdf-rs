//! Core 24-byte representation shared by [`CompactCBytes`](crate::CompactCBytes) and
//! [`CompactCString`](crate::CompactCString).
//!
//! It compactly stores up to 23 bytes inline or spills to the heap. The
//! 24th byte stores either the inline remainder tag or the heap discriminant.
//! For a fully packed inline value it also serves as the trailing NUL byte.
//!
//! # Acknowledgements
//!
//! The "magic last byte as discriminant" trick is inspired by the `ecow::EcoString` implementation
//! from the https://crates.io/crates/ecow crate. The niche optimization technique (making `Option<T>` zero-cost)
//! and the **pointer-typed first field that preserves provenance** are inspired by the `compact_str::CompactString`
//! implementation from the https://crates.io/crates/compact_str.

use core::ffi::CStr;
use core::mem::{align_of, size_of, ManuallyDrop};
use core::slice;

use ecow::EcoVec;

use crate::NulError;

/// Total byte footprint of `Repr` including the discriminant byte.
pub const INLINE_TOTAL: usize = 24;

/// Maximum payload bytes in inline mode (the 24th byte is the discriminant).
const INLINE_CAPACITY: usize = INLINE_TOTAL - 1;

/// Byte value that marks heap storage.
const SPILLED_LAST_BYTE: u8 = INLINE_TOTAL as u8;

/// Length of the trailing byte tail in `Repr`, after the two pointer-sized words and before the
/// 1-byte discriminant. On 64-bit this is 7, on 32-bit it is 15.
const TAIL_LENGTH: usize = INLINE_CAPACITY - 2 * size_of::<usize>();

/// Padding between the `EcoVec` and the discriminant byte in `HeapRepr`.
const PAD_LENGTH: usize = INLINE_TOTAL - size_of::<EcoVec<u8>>() - size_of::<HeapLastByte>();

/// Canonical 24-byte storage type.
///
/// # Why the first field is pointer-typed
///
/// The first word is `*const u8` rather than `[u8; 8]` to preserve pointer
/// provenance for the heap variant.
///
/// For inline values this slot is just payload bytes and is never read as a
/// pointer. This follows the same layout idea as [`compact_str::CompactString`].
///
/// # The Magic 24th Byte
///
/// The entire memory layout revolves around the 24th byte (`last`), which
/// serves four distinct purposes at once:
///
/// 1. Inline Length Tag: For inline strings, the last byte encodes the `remaining capacity`
///    using the formula `rem = INLINE_TOTAL - (len + 1)`. If you store a 10-byte string,
///    the last byte is `24 - 10 - 1 = 13`. For a 23-byte string, the last byte becomes
///    `24 - 23 - 1 = 0`, meaning it also will serve as the NUL terminator.
///
/// 2. NUL Terminator: By storing the remainder rather than the length, a fully packed
///    23-byte inline string results in `rem = 24 - (23 + 1) = 0`. Thus, the 24th byte
///    becomes `0` (`\0`), acting as the C-string NUL terminator. For shorter strings,
///    the unused bytes are zero-padded, ensuring a NUL is always present at `bytes[len]`.
///
/// 3. Heap Marker: If the slice exceeds 23 bytes, it spills to an `EcoVec`. The 24th byte
///    is then set to `24` (`LastByte::Spilled`), which marks a heap allocation.
///
/// 4. Niche Optimization Room: Because `last` only ever takes on valid values from `0..=24`,
///    the remaining bit patterns (25..=255) are unused by `LastByte`. The Rust compiler can
///    use these for "niche" optimization, meaning `Option<Repr>` will be the same size as
///    `Repr`. This is why we don't use a `union` for the `Repr` layout, since the compiler
///    could not recognize the optimization room for unions (inspired by [`compact_str::CompactString`]).
///
/// [`compact_str::CompactString`]: https://docs.rs/compact_str/latest/compact_str/struct.CompactString.html
#[cfg(target_pointer_width = "64")]
#[repr(C, align(8))]
pub(super) struct Repr {
    /// Pointer-typed slot. Heap: `EcoVec`'s data pointer (with provenance). Inline: raw bytes.
    ptr: *const u8,
    /// Heap: `EcoVec`'s `len`. Inline: payload bytes 8..16.
    len: usize,
    /// Heap: padding before the discriminant. Inline: payload bytes 16..23.
    tail: [u8; TAIL_LENGTH],
    /// Discriminant: inline remainder, or `Spilled` for heap.
    last: LastByte,
}

#[cfg(target_pointer_width = "32")]
#[repr(C, align(4))]
pub(super) struct Repr {
    ptr: *const u8,
    len: usize,
    tail: [u8; TAIL_LENGTH],
    last: LastByte,
}

/// Byte-typed staging layout for the inline variant.
///
/// Has the same size and alignment as [`Repr`], so a `mem::transmute` between them is layout-
/// correct. Inline values are constructed into an `InlineRepr` (where every byte is plain `u8`
/// and we can write fields directly in `const fn`) and then transmuted into a `Repr`. The
/// resulting `Repr.ptr` value has no provenance, which is fine because inline access never
/// reads it as a pointer.
#[cfg(target_pointer_width = "64")]
#[repr(C, align(8))]
#[derive(Copy, Clone)]
struct InlineRepr {
    bytes: [u8; INLINE_CAPACITY],
    last: LastByte,
}

#[cfg(target_pointer_width = "32")]
#[repr(C, align(4))]
#[derive(Copy, Clone)]
struct InlineRepr {
    bytes: [u8; INLINE_CAPACITY],
    last: LastByte,
}

/// Discriminant stored in the 24th byte (index 23) of every `Repr`.
///
/// For inline variants the value encodes "remaining capacity":
/// `InlineRem23` (= 23) means the buffer is empty (0 content bytes + 1 NUL = 1 byte used),
/// `InlineRem0` (= 0) means the buffer is fully packed (23 content bytes + 1 NUL = 24 bytes).
///
/// `Spilled` (= 24) is impossible for any inline variant because the remainder can never exceed 23.
/// This unused value becomes the heap discriminant and provides a niche for `Option` optimization.
#[allow(dead_code)] // variants are layout markers, not constructed directly, so we allow dead code
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
enum LastByte {
    InlineRem0 = 0,
    InlineRem1 = 1,
    InlineRem2 = 2,
    InlineRem3 = 3,
    InlineRem4 = 4,
    InlineRem5 = 5,
    InlineRem6 = 6,
    InlineRem7 = 7,
    InlineRem8 = 8,
    InlineRem9 = 9,
    InlineRem10 = 10,
    InlineRem11 = 11,
    InlineRem12 = 12,
    InlineRem13 = 13,
    InlineRem14 = 14,
    InlineRem15 = 15,
    InlineRem16 = 16,
    InlineRem17 = 17,
    InlineRem18 = 18,
    InlineRem19 = 19,
    InlineRem20 = 20,
    InlineRem21 = 21,
    InlineRem22 = 22,
    InlineRem23 = INLINE_CAPACITY as u8,
    Spilled = SPILLED_LAST_BYTE,
}

impl LastByte {
    /// Returns `true` if this discriminant indicates heap storage.
    #[inline(always)]
    const fn is_heap(self) -> bool {
        // Use byte comparison because `PartialEq` is not stable in const functions.
        self as u8 == Self::Spilled as u8
    }

    /// Constructs an inline discriminant from the number of unused bytes.
    ///
    /// # Panics
    ///
    /// Panics if `rem > 23`.
    #[inline(always)]
    const fn from_inline_rem(rem: usize) -> Self {
        assert!(rem <= 23, "inline remainder must be in 0..=23");
        // SAFETY: `rem <= 23`, and all values in `0..=23` are valid `LastByte` variants.
        unsafe { core::mem::transmute(rem as u8) }
    }
}

/// Heap-only last-byte tag. Guarantees that byte 23 of [`HeapRepr`] is always
/// [`SPILLED_LAST_BYTE`].
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
enum HeapLastByte {
    Spilled = SPILLED_LAST_BYTE,
}

/// Layout for the heap variant.
///
/// Whenever this is constructed, we make sure to push a trailing NUL byte to the
/// end of the `EcoVec<u8>`. It is unsound to construct this without the trailing
/// NUL.
///
/// Padding aligns the discriminant to byte 23, matching `Repr::last`.
///
/// `EcoVec<u8>` is `(NonNull<u8>, usize, PhantomData)`, so its first word is pointer-typed.
/// That word lines up with [`Repr`]'s pointer-typed `ptr` field, so
/// `mem::transmute<HeapRepr, Repr>` carries the data pointer's provenance through.
#[repr(C)]
pub(super) struct HeapRepr {
    vector: ManuallyDrop<EcoVec<u8>>,
    _pad: [u8; PAD_LENGTH],
    last: HeapLastByte,
}

impl HeapRepr {
    /// Wraps an `EcoVec` into a heap repr with `Spilled` discriminant.
    ///
    /// # Safety
    ///
    /// `vector` must end with one NUL byte and contain no interior NULs.
    #[inline]
    const unsafe fn new(vector: EcoVec<u8>) -> Self {
        Self {
            vector: ManuallyDrop::new(vector),
            _pad: [0; PAD_LENGTH],
            last: HeapLastByte::Spilled,
        }
    }

    /// Borrows the underlying `EcoVec`.
    #[inline]
    fn vector(&self) -> &EcoVec<u8> {
        &self.vector
    }

    /// Drops the `EcoVec` in place. Must be called exactly once.
    ///
    /// # Safety
    ///
    /// `self.vector` must be initialized and must not be used again after this call.
    #[inline]
    unsafe fn drop_vector(&mut self) {
        // SAFETY: caller guarantees this is the first and final drop of `self.vector`
        unsafe { ManuallyDrop::drop(&mut self.vector) };
    }
}

impl Repr {
    /// Creates a `Repr` from a `CStr`, going inline if possible.
    #[inline]
    pub(super) fn from_cstr(c_str: &CStr) -> Self {
        let bytes_with_nul = c_str.to_bytes_with_nul();
        // to_bytes_with_nul returns slice with length at least 1
        let len = bytes_with_nul.len() - 1;

        // SAFETY: `CStr` payload contains no interior NULs by definition.
        unsafe {
            if len < INLINE_TOTAL {
                Self::inline_unchecked(&bytes_with_nul[..len])
            } else {
                Self::from_heap(HeapRepr::new(EcoVec::from(bytes_with_nul)))
            }
        }
    }

    /// Creates a `Repr` from a byte slice, appending a NUL terminator.
    ///
    /// Returns an error if `bytes` contains an interior NUL.
    #[inline]
    pub(super) fn from_slice(bytes: &[u8]) -> Result<Self, NulError> {
        match memchr::memchr(0, bytes) {
            Some(position) => Err(NulError::at(position)),
            // SAFETY: `bytes` contains no NUL bytes.
            None => Ok(unsafe { Self::from_slice_unchecked(bytes) }),
        }
    }

    /// Creates a `Repr` from a byte slice, appending a NUL terminator,
    /// without checking for interior NULs.
    ///
    /// # Safety
    ///
    /// `bytes` must not contain any `0` byte.
    #[inline]
    pub(super) unsafe fn from_slice_unchecked(bytes: &[u8]) -> Self {
        if bytes.len() < INLINE_TOTAL {
            // SAFETY: The caller guarantees that `bytes` contain no `0` byte.
            unsafe { Self::inline_unchecked(bytes) }
        } else {
            let mut vector = EcoVec::with_capacity(bytes.len() + 1);
            vector.extend_from_slice(bytes);
            vector.push(0);
            // SAFETY: The caller guarantees that `bytes` contain no `0` byte.
            Self::from_heap(unsafe { HeapRepr::new(vector) })
        }
    }

    #[inline(always)]
    pub(super) const fn try_inline(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > INLINE_CAPACITY {
            return None;
        }

        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == 0 {
                return None;
            }
            i += 1;
        }

        // SAFETY:
        // - bytes.len() <= INLINE_CAPACITY;
        // - bytes contains no NUL bytes.
        Some(unsafe { Self::inline_unchecked(bytes) })
    }

    /// Packs raw bytes without a NUL terminator into the inline representation.
    ///
    /// # Safety
    ///
    /// `bytes.len()` must be at most [`INLINE_CAPACITY`], and `bytes` must not
    /// contain any NUL byte.
    #[inline(always)]
    const unsafe fn inline_unchecked(bytes: &[u8]) -> Self {
        let len = bytes.len();
        debug_assert!(len < INLINE_TOTAL);

        let mut data = [0_u8; INLINE_CAPACITY];

        // SAFETY:
        // - `data.as_mut_ptr()` is non-null and points to `INLINE_CAPACITY` initialized bytes.
        // - Caller guarantees `len <= INLINE_CAPACITY`.
        let dst = unsafe { slice::from_raw_parts_mut(data.as_mut_ptr(), len) };
        dst.copy_from_slice(bytes);

        // rem is the space left after the string + its logical NUL
        let rem = INLINE_TOTAL - (len + 1);

        // The `data` was initialized with zeros, so it automatically has a NUL terminator
        // after copying from the slice for payloads under 23 bytes. For exactly 23 bytes,
        // the calculated `rem` (0) in the 24th byte serves as the NUL.
        Self::from_inline(InlineRepr {
            bytes: data,
            last: LastByte::from_inline_rem(rem),
        })
    }

    #[inline(always)]
    const fn from_inline(inline: InlineRepr) -> Self {
        // SAFETY: `Repr` and `InlineRepr` have the same size and alignment, asserted below.
        // `*const u8` accepts any bit pattern, so reinterpreting bytes as it is valid.
        unsafe { core::mem::transmute(inline) }
    }

    /// Transmutes a [`HeapRepr`] into a `Repr`, preserving pointer provenance.
    #[inline(always)]
    const fn from_heap(spilled: HeapRepr) -> Self {
        // SAFETY:
        // - `Repr` and `HeapRepr` have the same size and alignment, asserted below.
        // - The `EcoVec`'s pointer-typed first word lines up with `Repr.ptr` (also pointer-typed),
        //   so provenance is preserved.
        // - The final byte is at the same offset in both layouts, and `HeapLastByte::Spilled`
        //   has the same byte value as `LastByte::Spilled`.
        unsafe { core::mem::transmute(spilled) }
    }

    /// Returns `true` if the data is heap-allocated.
    #[inline]
    pub(super) const fn is_heap(&self) -> bool {
        self.last.is_heap()
    }

    /// Returns the payload length in bytes (without the trailing NUL).
    #[inline]
    pub(super) fn len(&self) -> usize {
        if self.is_heap() {
            // SAFETY: data is heap-allocated, as checked above.
            let heap = unsafe { self.as_heap() };
            // HeapRepr invariant: vector always ends with a trailing NUL byte.
            heap.vector().len() - 1
        } else {
            // SAFETY: data is stored inline, as checked above.
            unsafe { self.inline_len() }
        }
    }

    /// Returns the payload bytes without the trailing NUL.
    #[inline]
    pub(super) fn to_bytes(&self) -> &[u8] {
        if self.is_heap() {
            // SAFETY: data is heap-allocated, as checked above.
            let heap = unsafe { self.as_heap() };
            let slice = heap.vector().as_slice();
            debug_assert!(!slice.is_empty());
            debug_assert_eq!(slice[slice.len() - 1], 0);
            &slice[..slice.len() - 1]
        } else {
            // SAFETY: data is stored inline, as checked above.
            let len = unsafe { self.inline_len() };

            // SAFETY:
            // - inline data is stored in bytes 0..23 of `Repr`
            // - `len <= INLINE_CAPACITY = 23`, so the slice stays within the struct
            // - Repr.ptr and other bytes of structure are read as data, not as a pointer
            unsafe { slice::from_raw_parts((self as *const Self).cast::<u8>(), len) }
        }
    }

    /// Returns the content as a `&CStr` without allocation.
    #[inline]
    pub(super) fn as_cstr(&self) -> &CStr {
        if self.is_heap() {
            // SAFETY: data is heap-allocated, as checked above.
            let spilled = unsafe { self.as_heap() };
            // SAFETY: HeapRepr invariant guarantees exactly one trailing NUL and no interior NULs.
            unsafe { CStr::from_bytes_with_nul_unchecked(spilled.vector().as_slice()) }
        } else {
            // SAFETY: data is stored inline, as checked above.
            let bytes_with_nul = unsafe { self.inline_bytes_with_nul() };
            // SAFETY: inline invariant guarantees exactly one trailing NUL and no interior NULs.
            unsafe { CStr::from_bytes_with_nul_unchecked(bytes_with_nul) }
        }
    }

    /// Returns the payload length for an inline variant.
    ///
    /// # Safety
    ///
    /// Must only be called when `!is_heap()`.
    #[inline]
    const unsafe fn inline_len(&self) -> usize {
        debug_assert!(!self.is_heap());
        INLINE_CAPACITY - self.last as u8 as usize
    }

    /// Reinterprets `&self` as `&HeapRepr`.
    ///
    /// # Safety
    ///
    /// Must only be called when `is_heap()`.
    #[inline]
    const unsafe fn as_heap(&self) -> &HeapRepr {
        debug_assert!(self.is_heap());
        // SAFETY: caller guarantees this is the heap variant. `Repr` and `HeapRepr` have the same
        // size and alignment (asserted at the bottom), and `Repr.ptr` is pointer-
        // typed so the `EcoVec` pointer's provenance is intact.
        unsafe { &*(self as *const Self).cast::<HeapRepr>() }
    }

    /// Reinterprets `&mut self` as `&mut HeapRepr`.
    ///
    /// # Safety
    ///
    /// Must only be called when `is_heap()`. The returned reference must
    /// preserve all `HeapRepr` invariants.
    #[inline]
    unsafe fn as_mut_heap(&mut self) -> &mut HeapRepr {
        debug_assert!(self.is_heap());
        // SAFETY: caller guarantees this is the heap variant. The layout
        // compatibility is asserted below.
        unsafe { &mut *(self as *mut Self).cast::<HeapRepr>() }
    }

    /// Returns a slice covering `payload + NUL` for the inline variant.
    ///
    /// # Safety
    ///
    /// Must only be called when `!is_heap()`.
    #[inline]
    const unsafe fn inline_bytes_with_nul(&self) -> &[u8] {
        debug_assert!(!self.is_heap());

        // SAFETY: caller guarantees this is the inline variant
        let len_with_nul = unsafe { self.inline_len() } + 1;

        // SAFETY:
        // 1. `Repr` is `#[repr(C)]`, so its inline payload + discriminant byte form a contiguous
        //    24-byte block in memory.
        // 2. Since `self.inline_len() <= 23`, `len_with_nul <= 24`. We will never read past
        //    the bounds of the struct.
        unsafe { slice::from_raw_parts((self as *const Self).cast::<u8>(), len_with_nul) }
    }
}

/// Default is an empty inline string (0 payload bytes, remainder = 23).
impl Default for Repr {
    #[inline]
    fn default() -> Self {
        Self::from_inline(InlineRepr {
            bytes: [0u8; INLINE_CAPACITY],
            last: LastByte::InlineRem23,
        })
    }
}

impl Clone for Repr {
    /// Clones the representation. This is a fast, `O(1)` operation.
    ///
    /// - Inline: Performs a simple bitwise copy of the 24-byte struct.
    /// - Heap: Cheaply increments the reference count of the underlying [`EcoVec`].
    #[inline]
    fn clone(&self) -> Self {
        if self.is_heap() {
            // SAFETY: data is heap-allocated, as checked above.
            let spilled = unsafe { self.as_heap() };
            let cloned = spilled.vector().clone();
            // SAFETY: cloning preserves the heap invariant of the original `EcoVec`.
            Repr::from_heap(unsafe { HeapRepr::new(cloned) })
        } else {
            // Inline values own no heap resource. Copy through `InlineRepr`
            // rather than `Repr`, since `Repr` has a pointer field and `Drop`.
            //
            // SAFETY: inline variant, as checked above. `InlineRepr` has the
            // same layout and is plain byte storage plus a valid `LastByte`.
            let inline = unsafe { (self as *const Self).cast::<InlineRepr>().read() };
            Self::from_inline(inline)
        }
    }
}

impl Drop for Repr {
    #[inline]
    fn drop(&mut self) {
        if self.is_heap() {
            // SAFETY: data is heap-allocated (checked above), and this is the first
            // and final drop of `self`
            unsafe { self.as_mut_heap().drop_vector() };
        }
    }
}

// SAFETY: `EcoVec<u8>` is `Send + Sync`, and the inline variant is plain bytes
// (`Repr::ptr` is never dereferenced for inline values)
unsafe impl Send for Repr {}
unsafe impl Sync for Repr {}

// Base type guarantees
const _: () = assert!(size_of::<EcoVec<u8>>() == 2 * size_of::<usize>());
const _: () = assert!(size_of::<LastByte>() == 1);
const _: () = assert!(size_of::<HeapLastByte>() == size_of::<LastByte>());
const _: () = assert!(align_of::<HeapLastByte>() == align_of::<LastByte>());
const _: () = assert!(HeapLastByte::Spilled as u8 == LastByte::Spilled as u8);

// Size guarantees
const _: () = assert!(size_of::<Repr>() == INLINE_TOTAL);
const _: () = assert!(size_of::<InlineRepr>() == INLINE_TOTAL);
const _: () = assert!(size_of::<HeapRepr>() == INLINE_TOTAL);

// Alignment guarantees
const _: () = assert!(align_of::<Repr>() == align_of::<HeapRepr>());
const _: () = assert!(align_of::<Repr>() == align_of::<InlineRepr>());
const _: () = assert!(align_of::<HeapRepr>() == align_of::<EcoVec<u8>>());

// Field offset guarantees: ensures `Repr.ptr` lines up with `EcoVec.ptr` (offset 0), `Repr.len`
// lines up with `EcoVec.len`, and the discriminant sits at byte 23 in every layout.
const _: () = assert!(core::mem::offset_of!(Repr, ptr) == 0);
const _: () = assert!(core::mem::offset_of!(Repr, len) == size_of::<usize>());
const _: () = assert!(core::mem::offset_of!(Repr, tail) == 2 * size_of::<usize>());
const _: () = assert!(core::mem::offset_of!(Repr, last) == INLINE_TOTAL - 1);
const _: () = assert!(core::mem::offset_of!(InlineRepr, last) == INLINE_TOTAL - 1);
const _: () = assert!(core::mem::offset_of!(HeapRepr, last) == INLINE_TOTAL - 1);

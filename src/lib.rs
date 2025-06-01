#![allow(clippy::unnecessary_cast)]

/// Error types
#[rustfmt::skip] #[macro_use] pub mod error;
/// Bitmaps used for creating halftoned versions of contone buffers, and saving out
pub mod bitmap;
/// Dynamically allocated array of bytes
pub mod buffer;
/// Color params
pub mod color_params;
/// Colorspace
pub mod colorspace;
/// Context
pub mod context;
/// Provide two-way communication between application and library
pub mod cookie;
/// Destination
pub mod destination;
/// Device interface
pub mod device;
/// A way of packaging up a stream of graphical operations
pub mod display_list;
/// Common document operation interface
pub mod document;
/// Easy creation of new documents
pub mod document_writer;
/// File paths
pub mod file_path;
/// Font
pub mod font;
/// Glyph
pub mod glyph;
/// Image
pub mod image;
/// Hyperlink
pub mod link;
/// Matrix operations
pub mod matrix;
/// Outline
pub mod outline;
/// Output
pub mod output;
/// Document page
pub mod page;
/// Path type
pub mod path;
/// PDF interface
pub mod pdf;
/// 2 dimensional array of contone pixels
pub mod pixmap;
/// Point type
pub mod point;
/// A representation for a region defined by 4 points
pub mod quad;
/// Rectangle types
pub mod rect;
/// Separations
pub mod separations;
/// Shadings
pub mod shade;
/// Size type
pub mod size;
/// Stroke state
pub mod stroke_state;

/// System font loading
#[cfg(all(feature = "system-fonts", not(target_arch = "wasm32")))]
pub mod system_font;
/// Text objects
pub mod text;
/// Text page
pub mod text_page;

/// Contains a special [`array::FzArray`] type which wraps an allocation from the `fz_calloc`
/// allocation fn that mupdf uses internally. Ideally this will eventually be replaced with
/// `Box<[_], A>` once the allocator api is stabilized.
pub mod array;

use array::FzArray;
pub use bitmap::Bitmap;
pub use buffer::Buffer;
pub use color_params::{ColorParams, RenderingIntent};
pub use colorspace::Colorspace;
pub(crate) use context::context;
pub use context::Context;
pub use cookie::Cookie;
pub use destination::{Destination, DestinationKind};
pub use device::{BlendMode, Device, Function, NativeDevice};
pub use display_list::DisplayList;
pub use document::{Document, MetadataName};
pub use document_writer::DocumentWriter;
pub(crate) use error::ffi_error;
pub use error::Error;
pub use file_path::FilePath;
pub use font::{CjkFontOrdering, Font, SimpleFontEncoding, WriteMode};
pub use glyph::Glyph;
pub use image::Image;
pub use link::Link;
pub use matrix::Matrix;
pub use outline::Outline;
pub use page::Page;
pub use path::{Path, PathWalker};
pub use pixmap::{ImageFormat, Pixmap};
pub use point::Point;
pub use quad::Quad;
pub use rect::{IRect, Rect};
pub use separations::Separations;
pub use shade::Shade;
pub use size::Size;
pub use stroke_state::{LineCap, LineJoin, StrokeState};
pub use text::{Text, TextItem, TextSpan};
pub use text_page::{TextBlock, TextChar, TextLine, TextPage, TextPageFlags};

use core::{marker::PhantomData, ptr::NonNull};
use zerocopy::{FromBytes, IntoBytes};

pub(crate) trait Sealed {}

#[allow(private_bounds)]
pub trait FFIAnalogue: IntoBytes + FromBytes + Sized + Sealed {
    type FFIType: IntoBytes + FromBytes + Sized;

    fn _assert_size_eq() {
        let _assert = AssertSizeEquals::<Self, Self::FFIType>::new();
    }
}

trait FFIWrapper {
    type FFIType;
    fn as_ref(&self) -> &Self::FFIType;
    fn as_ptr(&self) -> *const Self::FFIType;
    fn as_mut_ptr(&mut self) -> *mut Self::FFIType;
}

macro_rules! unsafe_impl_ffi_wrapper {
    ($struct:ident, $ffi_type:ident, $ffi_drop_fn:ident) => {
        impl $crate::FFIWrapper for $struct {
            type FFIType = $ffi_type;
            fn as_ref(&self) -> &Self::FFIType {
                // SAFETY: Guaranteed by caller
                unsafe { self.inner.as_ref() }
            }
            fn as_ptr(&self) -> *const Self::FFIType {
                self.inner.as_ptr()
            }
            fn as_mut_ptr(&mut self) -> *mut Self::FFIType {
                unsafe { self.inner.as_mut() }
            }
        }

        impl Drop for $struct {
            fn drop(&mut self) {
                let ptr = <Self as $crate::FFIWrapper>::as_ptr(&*self) as *mut _;
                // SAFETY: Guaranteed by caller
                unsafe { $ffi_drop_fn($crate::context(), ptr) }
            }
        }
    };
}

macro_rules! impl_ffi_traits {
    ($struct:ident, $ffi_type:ident) => {
        impl $crate::Sealed for $struct {}
        impl $crate::FFIAnalogue for $struct {
            type FFIType = $ffi_type;
        }

        impl From<$ffi_type> for $struct {
            fn from(val: $ffi_type) -> $struct {
                ::zerocopy::transmute!(val)
            }
        }

        impl From<$struct> for $ffi_type {
            fn from(val: $struct) -> $ffi_type {
                ::zerocopy::transmute!(val)
            }
        }
    };
}

pub(crate) use impl_ffi_traits;
pub(crate) use unsafe_impl_ffi_wrapper;

/// # Safety
///
/// * `ptr` can be null (this function will simply return an error if that is the case), but if it
///   is non-null then it must be well-aligned and point to a valid slice of `R::FFIType` that
///   contains at least `len` consecutive instances of `R::FFIType`. If it contains more than `len`
///   instances, those following instances must be treated as inaccessible, as they will still be
///   freed when the returned [`FzArray`] is dropped.
///
/// * `ptr` must also point to memory that was allocated by `fz_calloc`
unsafe fn rust_vec_from_ffi_ptr<R: FFIAnalogue>(
    ptr: *mut R::FFIType,
    len: i32,
) -> Result<FzArray<R>, Error> {
    let Some(ptr) = NonNull::new(ptr) else {
        return Err(Error::UnexpectedNullPtr);
    };

    let rust_ty_ptr = ptr.cast::<R>();
    // SAFETY: Upheld by caller
    // This is safe because we definitely have at least 0 elements here. We just have to create
    // this before the `usize::try_from` in case they gave us a valid pointer but an invalid
    // length. if that is the case, we need to make sure we still free the memory they give us,
    // which will be ensured by the `Drop` impl for this `FzArray`
    let mut arr = unsafe { FzArray::from_parts(rust_ty_ptr, 0) };

    let len = usize::try_from(len)?;
    // SAFETY: Upheld by caller - they told us that there are at least this many elements.
    unsafe { arr.set_len(len) };
    Ok(arr)
}

fn rust_slice_to_ffi_ptr<R: FFIAnalogue>(vec: &[R]) -> Result<(*const R::FFIType, i32), Error> {
    let len = i32::try_from(vec.len())?;
    let ptr = vec.as_ptr() as *mut R::FFIType;
    if ptr.is_null() {
        return Err(Error::UnexpectedNullPtr);
    }

    Ok((ptr, len))
}

struct AssertSizeEquals<A, B> {
    _phantom: PhantomData<(A, B)>,
}

impl<A, B> AssertSizeEquals<A, B> {
    const _SIZE_OK: () = assert!(size_of::<A>() == size_of::<B>());

    fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

macro_rules! from_enum {
    (
        $c_type:ty,
        $(#[$($attr:tt)*])*
        pub enum $name:ident {
            $(
                $(#[$($field_attr:tt)*])*
                $field:ident = $value:tt,
            )*
        }
    ) => {
        $(#[$($attr)*])*
        pub enum $name {
            $(
                $(#[$($field_attr)*])*
                $field = ($value as isize),
            )*
        }

        impl TryFrom<$c_type> for $name {
            type Error = Error;

            #[allow(non_upper_case_globals)]
            fn try_from(value: $c_type) -> Result<Self, Self::Error> {
                match value {
                    $($value => Ok(Self::$field),)*
                    _ => Err(Error::UnknownEnumVariant)
                }
            }
        }
    };
}
pub(crate) use from_enum;

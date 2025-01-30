#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::all)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub unsafe fn mupdf_new_derived_device<T>(
    ctx: *mut fz_context,
    label: *const std::ffi::c_char,
) -> *mut T {
    let size = std::ffi::c_int::try_from(size_of::<T>()).unwrap();
    let device = fz_new_device_of_size(ctx, size);
    let label = Memento_label(device.cast(), label);
    label.cast()
}

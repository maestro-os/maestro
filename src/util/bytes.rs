//! Utility functions for byte representations of types.

use core::mem::size_of;
use core::slice;

/// Marker trait for a type valid for any bit representation.
///
/// This trait can be auto-implemented using `#[derive(AnyRepr)]`.
///
/// # Safety
///
/// To implement this trait, it must be ensured the type is valid for any set values in memory.
pub unsafe trait AnyRepr {}

/// Returns an immutable slice to the given value.
pub fn as_bytes<T>(val: &T) -> &[u8] {
	unsafe { slice::from_raw_parts(val as *const _ as *const u8, size_of::<T>()) }
}

/// Reinterprets the given slice of bytes as another type.
///
/// If the size or alignment of the structure is invalid, the function returns `None`.
pub fn from_bytes<T: AnyRepr>(slice: &[u8]) -> Option<&T> {
	if size_of::<T>() <= slice.len() && slice.as_ptr().is_aligned() {
		// Safe because the slice is large enough
		let val = unsafe { &*(slice.as_ptr() as *const T) };
		Some(val)
	} else {
		None
	}
}

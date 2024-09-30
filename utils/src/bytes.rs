/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! Utility functions for byte representations of types.

use core::{
	mem::{align_of, size_of, size_of_val},
	slice,
};

/// Marker trait for a type valid for any bit representation.
///
/// This trait can be auto-implemented using `#[derive(AnyRepr)]`.
///
/// # Safety
///
/// To implement this trait, it must be ensured the type is valid for any set values in memory.
pub unsafe trait AnyRepr {}

// Primitive types implementation
unsafe impl AnyRepr for i8 {}
unsafe impl AnyRepr for i16 {}
unsafe impl AnyRepr for i32 {}
unsafe impl AnyRepr for i64 {}
unsafe impl AnyRepr for u8 {}
unsafe impl AnyRepr for u16 {}
unsafe impl AnyRepr for u32 {}
unsafe impl AnyRepr for u64 {}

unsafe impl<T: AnyRepr> AnyRepr for [T] {}
unsafe impl<T: AnyRepr, const N: usize> AnyRepr for [T; N] {}

/// Returns an immutable slice to the given value.
pub fn as_bytes<T: ?Sized>(val: &T) -> &[u8] {
	unsafe { slice::from_raw_parts(val as *const _ as *const u8, size_of_val(val)) }
}

/// As as [`as_bytes`], but mutable.
pub fn as_bytes_mut<T: ?Sized + AnyRepr>(val: &mut T) -> &mut [u8] {
	unsafe { slice::from_raw_parts_mut(val as *mut _ as *mut u8, size_of_val(val)) }
}

/// Reinterprets the given slice of bytes as another type.
///
/// If the size or alignment of the structure is invalid, the function returns `None`.
pub fn from_bytes<T: AnyRepr>(slice: &[u8]) -> Option<&T> {
	let size = size_of::<T>();
	let align = align_of::<T>();
	if size <= slice.len() && slice.as_ptr().is_aligned_to(align) {
		// Safe because the slice is large enough
		let val = unsafe { &*(slice.as_ptr() as *const T) };
		Some(val)
	} else {
		None
	}
}

/// Reinterprets the given slice of bytes as a slice of another type.
///
/// If the length of `slice` is not a multiple of the size of `T`, the function truncates the
/// output slice.
///
/// If the alignment is invalid, the function returns `None`.
pub fn slice_from_bytes<T: AnyRepr>(slice: &[u8]) -> Option<&[T]> {
	let len = slice.len() / size_of::<T>();
	let align = align_of::<T>();
	if slice.as_ptr().is_aligned_to(align) {
		let val = unsafe { slice::from_raw_parts(slice.as_ptr() as _, len) };
		Some(val)
	} else {
		None
	}
}

/// Same as [`slice_from_bytes`], but mutable.
pub fn slice_from_bytes_mut<T: AnyRepr>(slice: &mut [u8]) -> Option<&mut [T]> {
	let len = slice.len() / size_of::<T>();
	let align = align_of::<T>();
	if slice.as_ptr().is_aligned_to(align) {
		let val = unsafe { slice::from_raw_parts_mut(slice.as_mut_ptr() as _, len) };
		Some(val)
	} else {
		None
	}
}

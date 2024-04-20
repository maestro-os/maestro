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

use core::{mem::size_of, mem::size_of_val, slice};

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

/// Returns an immutable slice to the given value.
pub fn as_bytes<T: ?Sized>(val: &T) -> &[u8] {
	unsafe { slice::from_raw_parts(val as *const _ as *const u8, size_of_val(val)) }
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

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

//! Utilities used everywhere in the kernel and modules.
//!
//! Some features in this module, especially *collections* are **not** usable before memory
//! allocation is initialized.
//!
//! **Note**: This crate makes use of the `alloc` crate which is part of the Rust standard
//! libraries. It is being used solely for the **global allocator** feature. The collections that
//! are provided alongside *should* not be used as they do not allow handling memory allocation
//! failures. Instead, one should use the collections provided in [`collections`].

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::tabs_in_doc_comments)]
#![allow(internal_features)]
#![allow(unsafe_op_in_unsafe_fn)]
#![feature(allocator_api)]
#![feature(associated_type_defaults)]
#![feature(coerce_unsized)]
#![feature(core_intrinsics)]
#![feature(dispatch_from_dyn)]
#![feature(fmt_internals)]
#![feature(pointer_is_aligned_to)]
#![feature(portable_simd)]
#![feature(set_ptr_value)]
#![feature(strict_provenance_lints)]
#![feature(trusted_len)]
#![feature(unsize)]
#![deny(fuzzy_provenance_casts)]

extern crate self as utils;

pub mod boxed;
pub mod bytes;
pub mod collections;
pub mod cpio;
pub mod errno;
pub mod limits;
pub mod math;
pub mod ptr;
pub mod unsafe_mut;

use crate::errno::AllocResult;
use core::{
	alloc::{AllocError, Layout},
	borrow::Borrow,
	cmp::{Ordering, min},
	ffi::{c_int, c_void},
	fmt,
	fmt::Write,
	mem::size_of,
	ops::Add,
	ptr::NonNull,
	slice, write,
};

// C functions required by LLVM
#[allow(unused)]
unsafe extern "C" {
	fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *const c_void;
	fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *const c_void;
	fn memcmp(dest: *const c_void, src: *const c_void, n: usize) -> c_int;
	fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
	fn strlen(s: *const c_void) -> usize;
}

// Global allocator functions
#[cfg(not(any(feature = "std", test)))]
unsafe extern "Rust" {
	fn __alloc(layout: Layout) -> AllocResult<NonNull<[u8]>>;
	fn __realloc(
		ptr: NonNull<u8>,
		old_layout: Layout,
		new_layout: Layout,
	) -> AllocResult<NonNull<[u8]>>;
	fn __dealloc(ptr: NonNull<u8>, layout: Layout);
}

// If the library is compiled for userspace, make use of the `alloc` crate for allocation

#[cfg(any(feature = "std", test))]
extern crate alloc as rust_alloc;

#[cfg(any(feature = "std", test))]
#[unsafe(no_mangle)]
fn __alloc(layout: Layout) -> AllocResult<NonNull<[u8]>> {
	use rust_alloc::alloc::{Allocator, Global};
	Global.allocate(layout)
}

#[cfg(any(feature = "std", test))]
#[unsafe(no_mangle)]
unsafe fn __realloc(
	ptr: NonNull<u8>,
	old_layout: Layout,
	new_layout: Layout,
) -> AllocResult<NonNull<[u8]>> {
	use core::cmp::Ordering;
	use rust_alloc::alloc::{Allocator, Global};
	match new_layout.size().cmp(&old_layout.size()) {
		Ordering::Less => Global.shrink(ptr, old_layout, new_layout),
		Ordering::Greater => Global.grow(ptr, old_layout, new_layout),
		Ordering::Equal => Ok(NonNull::slice_from_raw_parts(
			NonNull::dangling(),
			new_layout.size(),
		)),
	}
}

#[cfg(any(feature = "std", test))]
#[unsafe(no_mangle)]
unsafe fn __dealloc(ptr: NonNull<u8>, layout: Layout) {
	use rust_alloc::alloc::{Allocator, Global};
	Global.deallocate(ptr, layout);
}

/// Aligns a pointer.
///
/// The returned value shall be greater than `ptr` or equal if the pointer is already aligned.
///
/// # Safety
///
/// There is no guarantee the returned pointer will point to a valid region of memory nor a valid
/// object.
#[inline(always)]
pub unsafe fn align<T>(ptr: *const T, align: usize) -> *const T {
	ptr.byte_add(ptr.align_offset(align))
}

/// Returns the of a type in bits.
#[inline(always)]
pub const fn bit_size_of<T>() -> usize {
	size_of::<T>() * 8
}

/// Returns a slice representing a C string beginning at the given pointer.
///
/// # Safety
///
/// The caller must ensure the pointer has a valid C string. An invalid C string causes an
/// undefined behavior.
///
/// The given pointer must remain valid during the whole execution.
pub unsafe fn str_from_ptr(ptr: *const u8) -> &'static [u8] {
	let len = strlen(ptr as _);
	slice::from_raw_parts(ptr, len)
}

/// Returns the length of the string representation of the number at the
/// beginning of the given string `s`.
pub fn nbr_len(s: &[u8]) -> usize {
	s.iter()
		.enumerate()
		.find(|(_, s)| **s < b'0' || **s > b'9')
		.map(|(i, _)| i)
		.unwrap_or(s.len())
}

/// Copies from slice `src` to `dst`.
///
/// If slice are not of the same length, the function copies only up to the length of the smallest.
///
/// The function returns the number of bytes copied.
pub fn slice_copy(src: &[u8], dst: &mut [u8]) -> usize {
	let len = min(src.len(), dst.len());
	dst[..len].copy_from_slice(&src[..len]);
	len
}

/// Compares `needle` to the range starting at `start`, with a size of `size`.
///
/// If `needle` is inside of the range, the function returns [`Ordering::Equal`].
pub fn range_cmp<T: Add<Output = T> + Ord + Copy>(start: T, size: T, needle: T) -> Ordering {
	let end = start + size;
	if needle < start {
		Ordering::Less
	} else if needle >= end {
		Ordering::Greater
	} else {
		Ordering::Equal
	}
}

/// Same as the [`Clone`] trait, but the operation can fail (on memory allocation
/// failure, for example).
pub trait TryClone {
	/// The error type on failure.
	type Error = AllocError;

	/// Clones the object. On failure, the function returns [`Self::Error`].
	fn try_clone(&self) -> Result<Self, Self::Error>
	where
		Self: Sized;
}

/// Blanket implementation.
impl<T: Clone + Sized> TryClone for T {
	fn try_clone(&self) -> Result<Self, Self::Error> {
		Ok(self.clone())
	}
}

/// Same as the `ToOwned` trait, but the operation can fail (on memory allocation failure, for
/// example).
pub trait TryToOwned {
	/// The resulting type after obtaining ownership.
	type Owned: Borrow<Self>;
	/// The error type on failure.
	type Error = AllocError;

	/// Creates owned data from borrowed data. On failure, the function returns [`Self::Error`].
	fn try_to_owned(&self) -> Result<Self::Owned, Self::Error>;
}

/// Wrapper structure allowing to implement the [`fmt::Display`] trait on `&[u8]` to display it as
/// a string.
pub struct DisplayableStr<'s>(pub &'s [u8]);

impl fmt::Display for DisplayableStr<'_> {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		for b in self.0 {
			fmt.write_char(*b as char)?;
		}
		Ok(())
	}
}

impl fmt::Debug for DisplayableStr<'_> {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		// TODO Add backslashes to escape `"` and `\`
		write!(fmt, "\"{self}\"")
	}
}

/// Wrapper to store data given the given memory alignment.
#[repr(C)]
pub struct Aligned<Align, Data: ?Sized> {
	/// Alignment padding.
	pub _align: [Align; 0],
	/// The data to align.
	pub data: Data,
}

/// Includes the bytes in the file at the given path and aligns them in memory with the given
/// alignment.
#[macro_export]
macro_rules! include_bytes_aligned {
	($align:ty, $path:expr) => {
		// const block to encapsulate static
		{
			static ALIGNED: &$crate::Aligned<$align, [u8]> = &$crate::Aligned {
				_align: [],
				data: *include_bytes!($path),
			};
			&ALIGNED.data
		}
	};
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn memcpy0() {
		let mut dest: [usize; 100] = [0; 100];
		let mut src: [usize; 100] = [0; 100];

		for (i, b) in src.iter_mut().enumerate() {
			*b = i;
		}
		unsafe {
			memcpy(
				dest.as_mut_ptr() as _,
				src.as_ptr() as _,
				100 * size_of::<usize>(),
			);
		}
		for (i, b) in dest.iter().enumerate() {
			debug_assert_eq!(*b, i);
		}
	}

	#[test]
	fn memcpy1() {
		let mut dest: [usize; 100] = [0; 100];
		let mut src: [usize; 100] = [0; 100];

		for (i, b) in src[10..90].iter_mut().enumerate() {
			*b = i;
		}
		unsafe {
			memcpy(
				dest.as_mut_ptr() as _,
				src.as_ptr() as _,
				100 * size_of::<usize>(),
			);
		}
		for b in &dest[0..10] {
			debug_assert_eq!(*b, 0);
		}
		for (i, b) in dest[10..90].iter().enumerate() {
			debug_assert_eq!(*b, i);
		}
		for b in &dest[90..100] {
			debug_assert_eq!(*b, 0);
		}
	}

	#[test]
	fn memcmp0() {
		let mut b0: [u8; 100] = [0; 100];
		let mut b1: [u8; 100] = [0; 100];

		for i in 0..100 {
			b0[i] = i as _;
			b1[i] = i as _;
		}
		let val = unsafe { memcmp(b0.as_mut_ptr() as _, b1.as_ptr() as _, 100) };
		assert_eq!(val, 0);
	}

	#[test]
	fn memcmp1() {
		let mut b0: [u8; 100] = [0; 100];
		let mut b1: [u8; 100] = [0; 100];

		for i in 0..100 {
			b0[i] = i as _;
			b1[i] = 0;
		}
		let val = unsafe { memcmp(b0.as_mut_ptr() as _, b1.as_ptr() as _, 100) };
		assert_eq!(val, 1);
	}

	// TODO More tests on memcmp

	// TODO Test `memset`
}

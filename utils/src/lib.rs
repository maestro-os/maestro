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
#![allow(internal_features)]
#![allow(clippy::tabs_in_doc_comments)]
#![feature(allocator_api)]
#![feature(associated_type_defaults)]
#![feature(coerce_unsized)]
#![feature(core_intrinsics)]
#![feature(dispatch_from_dyn)]
#![feature(non_null_convenience)]
#![feature(pointer_is_aligned)]
#![feature(portable_simd)]
#![feature(trusted_len)]
#![feature(unsize)]
#![feature(set_ptr_value)]
#![feature(slice_ptr_get)]
#![feature(custom_test_frameworks)]
#![feature(is_sorted)]

extern crate alloc;

pub mod boxed;
pub mod bytes;
pub mod collections;
pub mod errno;
pub mod interrupt;
pub mod io;
pub mod lock;
pub mod math;
pub mod ptr;

use core::{
	alloc::AllocError,
	cmp::min,
	ffi::{c_int, c_void},
	fmt,
	fmt::Write,
	marker::Sized,
	mem::size_of,
	slice, write,
};

// C functions required by LLVM
#[allow(unused)]
extern "C" {
	fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *const c_void;
	fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *const c_void;
	fn memcmp(dest: *const c_void, src: *const c_void, n: usize) -> c_int;
	fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
	fn strlen(s: *const c_void) -> usize;
}

/// Aligns down a pointer.
///
/// The returned value shall be lower than `ptr` or equal if the pointer is already aligned.
#[inline(always)]
pub fn down_align<T>(ptr: *const T, n: usize) -> *const T {
	((ptr as usize) & !(n - 1)) as *const T
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
	(ptr as *const c_void).add(ptr.align_offset(align)) as _
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
/// The returned slice remains valid only as long as the pointer does.
pub unsafe fn str_from_ptr(ptr: *const u8) -> &'static [u8] {
	let len = strlen(ptr as *const _);
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
pub fn slice_copy(src: &[u8], dst: &mut [u8]) {
	let len = min(src.len(), dst.len());
	dst[..len].copy_from_slice(&src[..len]);
}

/// Same as the [`Clone`] trait, but the operation can fail (on memory allocation
/// failure, for example).
pub trait TryClone {
	/// The error type used when allocation fails.
	type Error = AllocError;

	/// Clones the object. If the clone fails, the function returns an error.
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

/// Same as the [`Default`] trait, but the operation can fail (on memory allocation
/// failure, for example).
pub trait TryDefault {
	/// The error type used when allocation fails.
	type Error = AllocError;

	/// Returns the default value. On fail, the function returns Err.
	fn try_default() -> Result<Self, Self::Error>
	where
		Self: Sized;
}

/// Blanket implementation.
impl<T: Default + Sized> TryDefault for T {
	fn try_default() -> Result<Self, Self::Error> {
		Ok(Self::default())
	}
}

/// Wrapper structure allowing to implement the [`fmt::Display`] trait on the [u8] type
/// to display it as a string.
pub struct DisplayableStr<'a>(pub &'a [u8]);

impl<'a> fmt::Display for DisplayableStr<'a> {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		for b in self.0 {
			fmt.write_char(*b as char)?;
		}
		Ok(())
	}
}

impl<'a> fmt::Debug for DisplayableStr<'a> {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		write!(fmt, "\"{self}\"")
	}
}

/// Structure used to store data given the given memory alignment.
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

//! This module contains utilities used everywhere in the kernel.
//!
//! Data structures and containers are considered being two different things:
//! - Data structures do not require memory allocations
//! - Containers require memory allocations

pub mod boxed;
pub mod container;
pub mod io;
pub mod lock;
pub mod math;
pub mod ptr;

use crate::errno::Errno;
use core::cmp::min;
use core::ffi::c_int;
use core::ffi::c_void;
use core::fmt;
use core::fmt::Write;
use core::mem::size_of;
use core::slice;

// C functions required by LLVM
extern "C" {
	fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *const c_void;
	fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *const c_void;
	fn memcmp(dest: *const c_void, src: *const c_void, n: usize) -> c_int;
	fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
	fn strlen(s: *const c_void) -> usize;
}

/// Aligns down a pointer.
///
/// The retuned value shall be lower than `ptr` or equal if the pointer is already aligned.
#[inline(always)]
pub fn down_align<T>(ptr: *const T, n: usize) -> *const T {
	((ptr as usize) & !(n - 1)) as *const T
}

/// Aligns up a pointer.
///
/// The returned value shall be greater than `ptr`.
#[inline(always)]
pub fn up_align<T>(ptr: *const T, n: usize) -> *const T {
	((down_align(ptr, n) as usize) + n) as *const T
}

/// Aligns a pointer.
///
/// The returned value shall be greater than `ptr` or equal if the pointer is already aligned.
#[inline(always)]
pub fn align<T>(ptr: *const T, n: usize) -> *const T {
	if ptr.is_aligned_to(n) {
		ptr
	} else {
		up_align(ptr, n)
	}
}

/// Returns the of a type in bits.
#[inline(always)]
pub const fn bit_size_of<T>() -> usize {
	size_of::<T>() * 8
}

/// Returns the value stored into the specified register.
#[macro_export]
macro_rules! register_get {
	($reg:expr) => {{
		let mut val: u32;
		core::arch::asm!(concat!("mov {}, ", $reg), out(reg) val);

		val
	}};
}

/// Zeroes the given object.
///
/// # Safety
///
/// The caller must ensure an object with type `T` represented with only zeros is valid.
/// If not, the behaviour is undefined.
pub unsafe fn zero_object<T>(obj: &mut T) {
	let ptr = obj as *mut T as *mut u8;
	let size = size_of::<T>();

	let slice = slice::from_raw_parts_mut(ptr, size);
	slice.fill(0);
}

/// Returns the length of the C-style string pointed to by `s`, but limited to the first `n` bytes.
///
/// # Safety
///
/// The caller must ensure the pointer points to a valid chunk of memory, ending with at least one
/// 0 byte.
pub unsafe fn strnlen(s: *const u8, n: usize) -> usize {
	let mut i = 0;

	// TODO optimize
	while i < n && *s.add(i) != b'\0' {
		i += 1;
	}

	i
}

/// Returns a slice representing a C string beginning at the given pointer.
pub unsafe fn str_from_ptr(ptr: *const u8) -> &'static [u8] {
	slice::from_raw_parts(ptr, strlen(ptr as *const _))
}

/// Returns an immutable slice to the given value.
pub fn as_slice<T>(val: &T) -> &[u8] {
	unsafe { slice::from_raw_parts(val as *const _ as *const u8, size_of::<T>()) }
}

/// Returns the length of the string representation of the number at the
/// beginning of the given string `s`.
pub fn nbr_len(s: &[u8]) -> usize {
	s.into_iter()
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

/// Reinterprets the given slice of bytes as another type.
///
/// If the type is too large in size to fit in the slice, the function returns `None`.
///
/// # Safety
///
/// Not every types are defined for every possible memory representations. Thus, some values
/// passed as input to this function might be invalid for a given type, which is undefined.
pub unsafe fn reinterpret<T>(slice: &[u8]) -> Option<&T> {
	if size_of::<T>() <= slice.len() {
		// Safe because the slice is large enough
		let val = &*(slice.as_ptr() as *const T);
		Some(val)
	} else {
		None
	}
}

/// Trait allowing to perform a clone of a structure that can possibly fail (on
/// memory allocation failure, for example).
pub trait TryClone {
	/// Clones the object. If the clone fails, the function returns an error.
	fn try_clone(&self) -> Result<Self, Errno>
	where
		Self: Sized;
}

/// Blanket implementation.
impl<T: Clone + Sized> TryClone for T {
	fn try_clone(&self) -> Result<Self, Errno> {
		Ok(self.clone())
	}
}

/// Same as the Default trait, but the operation can fail (on memory allocation failure,
/// for example).
pub trait TryDefault {
	/// Returns the default value. On fail, the function returns Err.
	fn try_default() -> Result<Self, Errno>
	where
		Self: Sized;
}

/// Blanket implementation.
impl<T: Default + Sized> TryDefault for T {
	fn try_default() -> Result<Self, Errno> {
		Ok(Self::default())
	}
}

/// Wrapper structure allowing to implement the Display trait on the [u8] type
/// to display it as a string.
pub struct DisplayableStr<'a>(pub &'a [u8]);

impl<'a> fmt::Display for DisplayableStr<'a> {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
		for b in self.0 {
			fmt.write_char(*b as char)?;
		}

		Ok(())
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

/// Includes the bytes in the file at the given path and alignes them in memory with the given
/// alignment.
#[macro_export]
macro_rules! include_bytes_aligned {
	($align:ty, $path:expr) => {
		// const block to encapsulate static
		{
			static ALIGNED: &$crate::util::Aligned<$align, [u8]> = &$crate::util::Aligned {
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

	#[test_case]
	fn memcpy0() {
		let mut dest: [usize; 100] = [0; 100];
		let mut src: [usize; 100] = [0; 100];

		for i in 0..100 {
			src[i] = i;
		}
		unsafe {
			memcpy(
				dest.as_mut_ptr() as _,
				src.as_ptr() as _,
				100 * size_of::<usize>(),
			);
		}
		for i in 0..100 {
			debug_assert_eq!(dest[i], i);
		}
	}

	#[test_case]
	fn memcpy1() {
		let mut dest: [usize; 100] = [0; 100];
		let mut src: [usize; 100] = [0; 100];

		for i in 10..90 {
			src[i] = i;
		}
		unsafe {
			memcpy(
				dest.as_mut_ptr() as _,
				src.as_ptr() as _,
				100 * size_of::<usize>(),
			);
		}
		for i in 0..10 {
			debug_assert_eq!(dest[i], 0);
		}
		for i in 10..90 {
			debug_assert_eq!(dest[i], i);
		}
		for i in 90..100 {
			debug_assert_eq!(dest[i], 0);
		}
	}

	#[test_case]
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

	#[test_case]
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

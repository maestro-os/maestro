//! This module contains utilities used everywhere in the kernel.
//!
//! Data structures and containers are considered being two different things:
//! - Data structures do not require memory allocations
//! - Containers require memory allocations

pub mod boxed;
pub mod container;
pub mod io;
pub mod list;
pub mod lock;
pub mod math;
pub mod ptr;

use crate::errno::Errno;
use core::cmp::min;
use core::ffi::c_void;
use core::fmt;
use core::mem::size_of;
use core::slice;

/// Tells if pointer `ptr` is aligned on boundary `n`.
#[inline(always)]
pub fn is_aligned<T>(ptr: *const T, n: usize) -> bool {
	((ptr as usize) & (n - 1)) == 0
}

/// Aligns down a pointer. The retuned value shall be lower than `ptr` or equal
/// if the pointer is already aligned.
#[inline(always)]
pub fn down_align<T>(ptr: *const T, n: usize) -> *const T {
	((ptr as usize) & !(n - 1)) as *const T
}

/// Aligns up a pointer. The returned value shall be greater than `ptr`.
#[inline(always)]
pub fn up_align<T>(ptr: *const T, n: usize) -> *const T {
	((down_align(ptr, n) as usize) + n) as *const T
}

/// Aligns a pointer. The returned value shall be greater than `ptr` or equal if
/// the pointer is already aligned.
#[inline(always)]
pub fn align<T>(ptr: *const T, n: usize) -> *const T {
	if is_aligned(ptr, n) {
		ptr
	} else {
		up_align(ptr, n)
	}
}

/// Returns the of a type in bits.
#[inline(always)]
pub fn bit_size_of<T>() -> usize {
	size_of::<T>() * 8
}

/// Returns the offset of the given field `field` in structure `type`.
#[macro_export]
macro_rules! offset_of {
	($type:ty, $field:ident) => {
		#[allow(unused_unsafe)]
		unsafe {
			let ptr = core::ptr::NonNull::<core::ffi::c_void>::dangling().as_ptr();
			(&(*(ptr as *const $type)).$field) as *const _ as usize - ptr as usize
		}
	};
}

/// Returns the structure of type `type` that contains the structure in field `field` at pointer
/// `ptr`. The type must be a pointer type.
#[macro_export]
macro_rules! container_of {
	($ptr:expr, $type:ty, $field:ident) => {
		(($ptr as *const _ as usize) - crate::offset_of!($type, $field)) as $type
	};
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

extern "C" {
	/// Copies the given memory area `src` to `dest` with size `n`.
	/// If the given memory areas are overlapping, the behaviour is undefined.
	pub fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
	/// Same as memcpy, except the function can handle overlapping memory areas.
	pub fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
	/// Compares strings of byte `s1` and `s2` with length `n` and returns the
	/// diffence between the first bytes that differ.
	pub fn memcmp(s1: *const c_void, s2: *const c_void, n: usize) -> i32;
	/// Fills the `n` first bytes of the memory area pointed to by `s`, with the
	/// value `c`.
	pub fn memset(s: *mut c_void, c: i32, n: usize) -> *mut c_void;

	/// Zeros the given chunk of memory `s` with the given size `n`.
	pub fn bzero(s: *mut c_void, n: usize);
}

/// Zeroes the given object.
/// The function is marked unsafe since there exist some objects for which a representation full of
/// zeros is invalid.
pub unsafe fn zero_object<T>(obj: &mut T) {
	let ptr = obj as *mut T as *mut c_void;
	let size = size_of::<T>();

	bzero(ptr, size);
}

/// Returns the length of the string `s`.
/// If the pointer or the string is invalid, the behaviour is undefined.
#[no_mangle]
pub unsafe extern "C" fn strlen(s: *const u8) -> usize {
	let mut i = 0;

	while *s.add(i) != b'\0' {
		i += 1;
	}

	i
}

/// Like `strlen`, but limited to the first `n` bytes.
/// If the pointer or the string is invalid, the behaviour is undefined.
pub unsafe fn strnlen(s: *const u8, n: usize) -> usize {
	let mut i = 0;

	while i < n && *s.add(i) != b'\0' {
		i += 1;
	}

	i
}

/// Returns a slice representing a C string beginning at the given pointer.
pub unsafe fn str_from_ptr(ptr: *const u8) -> &'static [u8] {
	slice::from_raw_parts(ptr, strlen(ptr))
}

/// Returns an immutable slice to the given value.
pub fn as_slice<'a, T>(val: &'a T) -> &'a [u8] {
	unsafe { slice::from_raw_parts(&val as *const _ as *const u8, size_of::<T>()) }
}

/// Returns the length of the string representation of the number at the beginning of the given
/// string `s`.
pub fn nbr_len(s: &[u8]) -> usize {
	let mut i = 0;

	while i < s.len() {
		if (s[i] < b'0') || (s[i] > b'9') {
			break;
		}

		i += 1;
	}

	i
}

/// Copies from slice `src` to `dst`. If one slice is smaller than the other, the function stops
/// when the end of the smallest is reached.
pub fn slice_copy(src: &[u8], dst: &mut [u8]) {
	let len = min(src.len(), dst.len());
	dst[..len].copy_from_slice(&src[..len]);
}

/// Reinterprets the given slice of bytes as another type.
pub unsafe fn reinterpret<'a, T>(slice: &'a [u8]) -> &'a T {
	&*(slice.as_ptr() as *const _)
}

/// Trait allowing to perform a clone of a structure that can possibly fail (on memory allocation
/// failure, for example).
pub trait FailableClone {
	/// Clones the object. If the clone fails, the function returns Err.
	fn failable_clone(&self) -> Result<Self, Errno>
	where
		Self: Sized;
}

/// Implements FailableClone with the default implemention for the given type. The type must
/// implement Clone.
#[macro_export]
macro_rules! failable_clone_impl {
	($type:ty) => {
		impl FailableClone for $type {
			fn failable_clone(&self) -> Result<Self, crate::errno::Errno> {
				Ok(self.clone())
			}
		}
	};
}

failable_clone_impl!(i8);
failable_clone_impl!(u8);
failable_clone_impl!(i16);
failable_clone_impl!(u16);
failable_clone_impl!(i32);
failable_clone_impl!(u32);
failable_clone_impl!(i64);
failable_clone_impl!(u64);
failable_clone_impl!(isize);
failable_clone_impl!(usize);

failable_clone_impl!(*mut c_void);
failable_clone_impl!(*const c_void);

/// Wrapper structure allowing to implement the Display trait on the [u8] type to display it as a
/// string.
pub struct DisplayableStr<'a> {
	/// The string to be displayed.
	pub s: &'a [u8],
}

impl<'a> fmt::Display for DisplayableStr<'a> {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
		for b in self.s {
			write!(fmt, "{}", *b as char)?;
		}

		Ok(())
	}
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
	fn memmove0() {
		let mut dest: [usize; 100] = [0; 100];
		let mut src: [usize; 100] = [0; 100];

		for i in 0..100 {
			src[i] = i;
		}
		unsafe {
			memmove(
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
	fn memmove1() {
		let mut buff: [usize; 100] = [0; 100];

		for i in 0..100 {
			buff[i] = i;
		}
		unsafe {
			memmove(
				buff.as_mut_ptr() as _,
				buff.as_ptr() as _,
				100 * size_of::<usize>(),
			);
		}
		for i in 0..100 {
			debug_assert_eq!(buff[i], i);
		}
	}

	// TODO More tests on memmove

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

	#[test_case]
	fn memmove0() {
		let mut buff: [u8; 100] = [0; 100];

		for i in 0..100 {
			buff[i] = i as _;
		}
		unsafe {
			bzero(buff.as_mut_ptr() as _, 100);
		}
		for i in 0..100 {
			debug_assert_eq!(buff[i], 0);
		}
	}

	// TODO More tests on memmove
}

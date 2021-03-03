/// This module contains utilities used everywhere in the kernel.
/// All the features here are guaranteed to not require memory allocators.
///
/// Data structures and containers are considered two separated things:
/// - Data structures do not require memory allocations
/// - Containers require memory allocations

pub mod boxed;
pub mod container;
pub mod list;
pub mod lock;
pub mod math;
pub mod ptr;

use core::ffi::c_void;
use core::mem::MaybeUninit;

/// Tells if pointer `ptr` is aligned on boundary `n`.
#[inline(always)]
pub fn is_aligned(ptr: *const c_void, n: usize) -> bool {
	((ptr as usize) & (n - 1)) == 0
}

/// Aligns down a pointer. The retuned value shall be lower than `ptr` or equal
/// if the pointer is already aligned.
#[inline(always)]
pub fn down_align(ptr: *const c_void, n: usize) -> *const c_void {
	((ptr as usize) & !(n - 1)) as *const c_void
}

/// Aligns up a pointer. The returned value shall be greater than `ptr`.
#[inline(always)]
pub fn up_align(ptr: *const c_void, n: usize) -> *const c_void {
	((down_align(ptr, n) as usize) + n) as *const c_void
}

/// Aligns a pointer. The returned value shall be greater than `ptr` or equal if
/// the pointer is already aligned.
#[inline(always)]
pub fn align(ptr: *const c_void, n: usize) -> *const c_void {
	if is_aligned(ptr, n) {
		ptr
	} else {
		up_align(ptr, n)
	}
}

/// Returns the of a type in bits.
#[inline(always)]
pub fn bit_size_of<T>() -> usize {
	core::mem::size_of::<T>() * 8
}

/// Returns the offset of the given field `field` in structure `type`.
#[macro_export]
macro_rules! offset_of {
	($type:ty, $field:ident) => {
		#[allow(unused_unsafe)]
		unsafe { // Dereference of raw pointer
			let ptr = core::ptr::NonNull::<c_void>::dangling().as_ptr();
			(&(*(ptr as *const $type)).$field) as *const _ as usize - ptr as usize
		}
	}
}

/// Returns the structure of type `type` that contains the structure in field `field` at pointer
/// `ptr`. The type must be a pointer type.
#[macro_export]
macro_rules! container_of {
	($ptr:expr, $type:ty, $field:ident) => {
		(($ptr as *const _ as usize) - crate::offset_of!($type, $field)) as $type
	}
}

/// Structure representing the list of registers for a context. The content of this structure
/// depends on the architecture for which the kernel is compiled.
#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct Regs
{
	pub ebp: u32,
	pub esp: u32,
	pub eip: u32,
	pub eflags: u32,
	pub eax: u32,
	pub ebx: u32,
	pub ecx: u32,
	pub edx: u32,
	pub esi: u32,
	pub edi: u32,
}

extern "C" {
	pub fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
	pub fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
	pub fn memcmp(s1: *const c_void, s2: *const c_void, n: usize) -> i32;
	pub fn memset(s: *mut c_void, c: i32, n: usize) -> *mut c_void;

	pub fn bzero(s: *mut c_void, n: usize);

	pub fn strlen(s: *const c_void) -> usize;
}

/// Zeroes the given object.
pub fn zero_object<T>(obj: &mut T) {
	let ptr = obj as *mut T as *mut c_void;
	let size = core::mem::size_of::<T>();

	unsafe {
		bzero(ptr, size);
	}
}

/// Converts the given pointer to a string of characters. The string must be valid and must end
/// with `\0`. The ownership of the string is not taken, thus the caller must drop it manually.
pub unsafe fn ptr_to_str(ptr: *const c_void) -> &'static str {
	let len = strlen(ptr);
	let slice = core::slice::from_raw_parts(ptr as *const u8, len);
	core::str::from_utf8_unchecked(slice)
}

/// Assigns value `val` to pointer `ptr` without calling Drop on it to prevent dropping garbage
/// data. If `ptr` doesn't point to a valid memory location, the behaviour is undefined.
pub unsafe fn write_ptr<T>(ptr: *mut T, val: T) {
	let next = &mut *(ptr as *mut MaybeUninit<T>);
	next.write(val);
}

#[cfg(test)]
mod test {
	use super::*;
	use core::mem::size_of;

	#[test_case]
	fn memcpy0() {
		let mut dest: [usize; 100] = [0; 100];
		let mut src: [usize; 100] = [0; 100];

		for i in 0..100 {
			src[i] = i;
		}
		unsafe { // Call to C function
			memcpy(dest.as_mut_ptr() as _, src.as_ptr() as _, 100 * size_of::<usize>());
		}
		for i in 0..100 {
			debug_assert_eq!(dest[i], i);
		}
	}

	// TODO More tests on memcpy

	#[test_case]
	fn memmove0() {
		let mut dest: [usize; 100] = [0; 100];
		let mut src: [usize; 100] = [0; 100];

		for i in 0..100 {
			src[i] = i;
		}
		unsafe { // Call to C function
			memmove(dest.as_mut_ptr() as _, src.as_ptr() as _, 100 * size_of::<usize>());
		}
		for i in 0..100 {
			debug_assert_eq!(dest[i], i);
		}
	}

	// TODO More tests on memmove

	// TODO Test `memcmp`
	// TODO Test `memset`
	// TODO Test `bzero`
	// TODO Test `strlen`
}

//! This module contains utilities used everywhere in the kernel.
//!
//! Data structures and containers are considered being two different things:
//! - Data structures do not require memory allocations
//! - Containers require memory allocations

pub mod boxed;
pub mod container;
pub mod list;
pub mod lock;
pub mod math;
pub mod ptr;

use core::ffi::c_void;
use core::fmt;
use core::mem::MaybeUninit;
use crate::errno::Errno;

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
		unsafe {
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

/// Returns the value stored into the specified register.
#[macro_export]
macro_rules! register_get {
	($reg:expr) => {{
		let mut val: u32;
		// TODO Use new syntax
		// TODO Let the compiler allocate the register it wants
		// TODO Adapt to the size of the given register
		llvm_asm!(concat!("mov %", $reg, ", %eax") : "={eax}"(val));

		val
	}};
}

extern "C" {
	pub fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
	pub fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
	pub fn memcmp(s1: *const c_void, s2: *const c_void, n: usize) -> i32;
	pub fn memset(s: *mut c_void, c: i32, n: usize) -> *mut c_void;

	pub fn bzero(s: *mut c_void, n: usize);

	pub fn strlen(s: *const c_void) -> usize;
}

/// Trait allowing to perform a clone of a structure that can possibly fail (on memory allocation
/// failure, for example).
pub trait FailableClone {
	/// Clones the object. If the clone fails, the function returns Err.
	fn failable_clone(&self) -> Result<Self, Errno> where Self: Sized;
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
	}
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

/// Structure representing the list of registers for a context. The content of this structure
/// depends on the architecture for which the kernel is compiled.
#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
//#[cfg(config_general_arch = "x86")]
pub struct Regs {
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

impl fmt::Display for Regs {
	//#[cfg(config_general_arch = "x86")]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "ebp: {:p} esp: {:p} eip: {:p} eflags: {:p} eax: {:p}\n
ebx: {:p} ecx: {:p} edx: {:p} esi: {:p} edi: {:p}\n",
			self.ebp as *const c_void,
			self.esp as *const c_void,
			self.eip as *const c_void,
			self.eflags as *const c_void,
			self.eax as *const c_void,
			self.ebx as *const c_void,
			self.ecx as *const c_void,
			self.edx as *const c_void,
			self.esi as *const c_void,
			self.edi as *const c_void)
	}
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

/// Turns the error into an empty error for the given result.
pub fn to_empty_error<T, E>(r: Result<T, E>) -> Result<T, ()> {
	if let Ok(t) = r {
		Ok(t)
	} else {
		 Err(())
	}
}

/// Returns the length of the number at the beginning of the given string `s`.
pub fn nbr_len(s: &[u8]) -> usize {
	let mut i = 0;

	while i < s.len() {
		if (s[i] < '0' as u8) || (s[i] > '9' as u8) {
			break;
		}

		i += 1;
	}

	i
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
		unsafe {
			memcpy(dest.as_mut_ptr() as _, src.as_ptr() as _, 100 * size_of::<usize>());
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
			memcpy(dest.as_mut_ptr() as _, src.as_ptr() as _, 100 * size_of::<usize>());
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
			memmove(dest.as_mut_ptr() as _, src.as_ptr() as _, 100 * size_of::<usize>());
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
			memmove(buff.as_mut_ptr() as _, buff.as_ptr() as _, 100 * size_of::<usize>());
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
		let val = unsafe {
			memcmp(b0.as_mut_ptr() as _, b1.as_ptr() as _, 100)
		};
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
		let val = unsafe {
			memcmp(b0.as_mut_ptr() as _, b1.as_ptr() as _, 100)
		};
		assert_eq!(val, 1);
	}

	// TODO More tests on memcmp

	// TODO Test `memset`

	#[test_case]
	fn memmove0() {
		let mut buff: [usize; 100] = [0; 100];

		for i in 0..100 {
			buff[i] = i;
		}
		unsafe {
			bzero(buff.as_mut_ptr() as _, 100 * size_of::<usize>());
		}
		for i in 0..100 {
			debug_assert_eq!(buff[i], 0);
		}
	}

	// TODO More tests on memmove

	// TODO Test `strlen`
}

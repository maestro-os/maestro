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

//! Userspace memory access utilities.

use crate::{memory, memory::vmem, process::mem_space::COPY_BUFFER, syscall::FromSyscallArg};
use core::{
	arch::global_asm,
	cmp::min,
	fmt,
	intrinsics::likely,
	mem::{size_of, size_of_val, MaybeUninit},
	ptr::{null_mut, NonNull},
};
use utils::{
	collections::{string::String, vec::Vec},
	errno,
	errno::EResult,
};

/// Tells whether the pointer is in bound with the userspace.
fn bound_check(ptr: *const u8, n: usize) -> bool {
	ptr as usize >= memory::PAGE_SIZE && (ptr as usize + n) < COPY_BUFFER as usize
}

// TODO
global_asm!(
	r"
.global raw_copy

raw_copy:
	ret
"
);

extern "C" {
	/// TODO doc
	fn raw_copy(src: *const u8, dst: *mut u8, n: usize) -> bool;
}

/// TODO doc
unsafe fn copy_from_user_raw(src: *const u8, dst: *mut u8, n: usize) -> EResult<()> {
	if !likely(bound_check(dst, n)) {
		return Err(errno!(EFAULT));
	}
	let res = vmem::smap_disable(|| raw_copy(src, dst, n));
	if likely(res) {
		Ok(())
	} else {
		Err(errno!(EFAULT))
	}
}

/// TODO doc
unsafe fn copy_to_user_raw(src: *const u8, dst: *mut u8, n: usize) -> EResult<()> {
	if !likely(bound_check(dst, n)) {
		return Err(errno!(EFAULT));
	}
	let res = vmem::smap_disable(|| raw_copy(src, dst, n));
	if likely(res) {
		Ok(())
	} else {
		Err(errno!(EFAULT))
	}
}

/// Wrapper for a pointer.
pub struct SyscallPtr<T: Sized + fmt::Debug>(pub Option<NonNull<T>>);

impl<T: Sized + fmt::Debug> FromSyscallArg for SyscallPtr<T> {
	fn from_syscall_arg(val: usize) -> Self {
		Self(NonNull::new(val as _))
	}
}

impl<T: Sized + fmt::Debug> SyscallPtr<T> {
	/// Returns a mutable pointer to the data.
	pub fn as_ptr(&self) -> *mut T {
		self.0.map(NonNull::as_ptr).unwrap_or(null_mut())
	}

	/// Copies the value from userspace and returns it.
	///
	/// If the pointer is null, the function returns `None`.
	///
	/// If the value is not accessible, the function returns an error.
	pub fn copy_from_user(&self) -> EResult<Option<T>> {
		let Some(ptr) = self.0 else {
			return Ok(None);
		};
		unsafe {
			let mut val = MaybeUninit::<T>::uninit();
			copy_from_user_raw(
				ptr.as_ptr() as *const _,
				val.as_mut_ptr() as *mut _,
				size_of::<T>(),
			)?;
			Ok(Some(val.assume_init()))
		}
	}

	/// Copies the value to userspace.
	///
	/// If the pointer is null, the function does nothing.
	///
	/// If the value is not accessible, the function returns an error.
	///
	/// If the value is located on lazily allocated pages, the function
	/// allocates physical pages in order to allow writing.
	pub fn copy_to_user(&self, val: T) -> EResult<()> {
		let Some(ptr) = self.0 else {
			return Ok(());
		};
		unsafe {
			copy_to_user_raw(
				&val as *const _ as *const _,
				ptr.as_ptr() as *mut _,
				size_of::<T>(),
			)?;
		}
		Ok(())
	}
}

impl<T: fmt::Debug> fmt::Debug for SyscallPtr<T> {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		let ptr = self.as_ptr();
		match self.copy_from_user() {
			Ok(Some(val)) => write!(fmt, "{ptr:p} = {val:?}"),
			Ok(None) => write!(fmt, "NULL"),
			Err(e) => write!(fmt, "{ptr:p} = (cannot read: {e})"),
		}
	}
}

/// Wrapper for a slice.
///
/// The size of the slice is required when trying to access it.
pub struct SyscallSlice<T: Sized + fmt::Debug>(pub Option<NonNull<T>>);

impl<T: Sized + fmt::Debug> FromSyscallArg for SyscallSlice<T> {
	fn from_syscall_arg(val: usize) -> Self {
		Self(NonNull::new(val as _))
	}
}

impl<T: Sized + fmt::Debug> SyscallSlice<T> {
	/// Returns a mutable pointer to the data.
	pub fn as_ptr(&self) -> *mut T {
		self.0.map(NonNull::as_ptr).unwrap_or(null_mut())
	}

	/// Copies the slice from userspace and returns it.
	///
	/// `len` is the in number of elements in the slice.
	///
	/// If the pointer is null, the function returns `None`.
	///
	/// If the slice is not accessible, the function returns an error.
	pub fn copy_from_user(&self, len: usize) -> EResult<Option<Vec<T>>> {
		let Some(ptr) = self.0 else {
			return Ok(None);
		};
		let mut buf: Vec<T> = Vec::with_capacity(len)?;
		unsafe {
			buf.set_len(len);
			copy_from_user_raw(
				ptr.as_ptr() as *const _,
				buf.as_mut_ptr() as *mut _,
				size_of::<T>() * len,
			)?;
			Ok(Some(buf))
		}
	}

	/// Copies the value to userspace.
	///
	/// `len` is the in number of elements in the slice.
	///
	/// If the pointer is null, the function does nothing.
	///
	/// If the slice is not accessible, the function returns an error.
	///
	/// If the slice is located on lazily allocated pages, the function
	/// allocates physical pages in order to allow writing.
	pub fn copy_to_user(&self, val: &[T]) -> EResult<()> {
		let Some(ptr) = self.0 else {
			return Ok(());
		};
		unsafe {
			copy_to_user_raw(
				val.as_ptr() as *const _,
				ptr.as_ptr() as *mut _,
				size_of_val(val),
			)?;
		}
		Ok(())
	}
}

impl<T: fmt::Debug> fmt::Debug for SyscallSlice<T> {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self.0 {
			Some(ptr) => write!(fmt, "{ptr:p}"),
			None => write!(fmt, "NULL"),
		}
	}
}

/// Wrapper for a C-style, nul-terminated (`\0`) string.
pub struct SyscallString(pub Option<NonNull<u8>>);

impl FromSyscallArg for SyscallString {
	fn from_syscall_arg(val: usize) -> Self {
		Self(NonNull::new(val as _))
	}
}

impl SyscallString {
	/// Returns an immutable pointer to the data.
	pub fn as_ptr(&self) -> *const u8 {
		self.0.map(NonNull::as_ptr).unwrap_or(null_mut())
	}

	/// Returns an immutable reference to the string.
	///
	/// If the string is not accessible, the function returns an error.
	pub fn copy_from_user(&self) -> EResult<Option<String>> {
		let Some(ptr) = self.0 else {
			return Ok(None);
		};
		// TODO use empirical data to find the best value, and whether an arithmetic progression is
		// the optimal solution
		const CHUNK_SIZE: usize = 128;
		let mut buf = Vec::new();
		loop {
			let buf_cursor = buf.len();
			let user_cursor = ptr.as_ptr() as usize + buf_cursor;
			let page_end = user_cursor.next_multiple_of(memory::PAGE_SIZE);
			let buf_size = min(page_end - user_cursor, CHUNK_SIZE);
			// Read the next chunk
			buf.reserve(buf_size)?;
			unsafe {
				buf.set_len(buf_cursor + buf_size);
				copy_from_user_raw(
					user_cursor as *const _,
					&mut buf[buf_cursor] as *mut _,
					buf_size,
				)?;
			}
			// Check for a nul character
			let end = buf[buf_cursor..(buf_cursor + buf_size)]
				.iter()
				.enumerate()
				.find(|(_, b)| **b == b'\0')
				.map(|(i, _)| i);
			if let Some(end) = end {
				buf.truncate(end);
				break;
			}
		}
		Ok(Some(buf.into()))
	}
}

impl fmt::Debug for SyscallString {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		let ptr = self.as_ptr();
		match self.copy_from_user() {
			Ok(Some(s)) => write!(fmt, "{ptr:p} = {s:?}"),
			Ok(None) => write!(fmt, "NULL"),
			Err(e) => write!(fmt, "{ptr:p} = (cannot read: {e})"),
		}
	}
}

/// Wrapper for a C-style, NULL-terminated string array.
pub struct SyscallArray(pub Option<NonNull<*const u8>>);

impl FromSyscallArg for SyscallArray {
	fn from_syscall_arg(val: usize) -> Self {
		Self(NonNull::new(val as _))
	}
}

impl SyscallArray {
	/// Returns an immutable pointer to the data.
	pub fn as_ptr(&self) -> *const *const u8 {
		self.0.map(NonNull::as_ptr).unwrap_or(null_mut())
	}

	/// Returns an iterator over the array's elements.
	pub fn iter(&self) -> SyscallArrayIterator {
		SyscallArrayIterator {
			arr: self,
			i: 0,
		}
	}
}

impl fmt::Debug for SyscallArray {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut list = fmt.debug_list();
		let mut list_ref = &mut list;
		for elem in self.iter() {
			list_ref = match elem {
				Ok(s) => list_ref.entry(&s),
				Err(e) => list_ref.entry(&e),
			};
		}
		list_ref.finish()
	}
}

/// Iterators over elements of [`SyscallArray`].
pub struct SyscallArrayIterator<'a> {
	/// The array.
	arr: &'a SyscallArray,
	/// The current index.
	i: usize,
}

impl<'a> Iterator for SyscallArrayIterator<'a> {
	type Item = EResult<String>;

	fn next(&mut self) -> Option<Self::Item> {
		let Some(arr) = self.arr.0 else {
			return Some(Err(errno!(EFAULT)));
		};
		let str_ptr = unsafe { arr.add(self.i).read_volatile() };
		let res = SyscallString(NonNull::new(str_ptr as _))
			.copy_from_user()
			.transpose();
		// Do not increment if reaching `NULL`
		if res.is_some() {
			self.i += 1;
		}
		res
	}
}

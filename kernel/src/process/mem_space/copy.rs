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

use crate::{memory::vmem, process::mem_space::bound_check, syscall::FromSyscallArg};
use core::{
	cmp::min,
	ffi::c_void,
	fmt,
	intrinsics::{likely, unlikely},
	mem::{size_of, size_of_val, MaybeUninit},
	ptr,
	ptr::{null_mut, NonNull},
};
use utils::{
	collections::{string::String, vec::Vec},
	errno,
	errno::EResult,
	limits::PAGE_SIZE,
};

extern "C" {
	/// Copy, with access check. On success, the function returns `true`.
	pub fn raw_copy(dst: *mut u8, src: *const u8, n: usize) -> bool;
	/// Function to be called back when a page fault occurs while using [`raw_copy`].
	pub fn copy_fault();
}

/// Low level function to copy data from userspace to kernelspace, with access check.
///
/// If the access check fails, the function returns [`EFAULT`].
unsafe fn copy_from_user_raw(src: *const u8, dst: *mut u8, n: usize) -> EResult<()> {
	if unlikely(!bound_check(src as _, n)) {
		return Err(errno!(EFAULT));
	}
	let res = vmem::smap_disable(|| raw_copy(dst, src, n));
	if likely(res) {
		Ok(())
	} else {
		Err(errno!(EFAULT))
	}
}

/// Low level function to copy data from kernelspace to userspace, with access check.
///
/// If the access check fails, the function returns [`EFAULT`].
unsafe fn copy_to_user_raw(src: *const u8, dst: *mut u8, n: usize) -> EResult<()> {
	if unlikely(!bound_check(dst as _, n)) {
		return Err(errno!(EFAULT));
	}
	let res = vmem::smap_disable(|| raw_copy(dst, src, n));
	if likely(res) {
		Ok(())
	} else {
		Err(errno!(EFAULT))
	}
}

/// Wrapper for a pointer.
pub struct SyscallPtr<T: Sized + fmt::Debug>(pub Option<NonNull<T>>);

impl<T: Sized + fmt::Debug> FromSyscallArg for SyscallPtr<T> {
	fn from_syscall_arg(ptr: usize, _compat: bool) -> Self {
		Self(NonNull::new(ptr::with_exposed_provenance_mut(ptr)))
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
	pub fn copy_to_user(&self, val: &T) -> EResult<()> {
		let Some(ptr) = self.0 else {
			return Ok(());
		};
		unsafe {
			copy_to_user_raw(
				val as *const _ as *const _,
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
	fn from_syscall_arg(ptr: usize, _compat: bool) -> Self {
		Self(NonNull::new(ptr::with_exposed_provenance_mut(ptr)))
	}
}

impl<T: Sized + fmt::Debug> SyscallSlice<T> {
	/// Returns a mutable pointer to the data.
	pub fn as_ptr(&self) -> *mut T {
		self.0.map(NonNull::as_ptr).unwrap_or(null_mut())
	}

	/// Copies the slice from userspace and returns it.
	///
	/// Arguments:
	/// - `off` is the offset relative to the beginning of the userspace slice.
	/// - `buf` is the destination slice.
	///
	/// If the pointer is null, the function returns `false`.
	///
	/// If the slice is not accessible, the function returns an error.
	pub fn copy_from_user(&self, off: usize, buf: &mut [T]) -> EResult<bool> {
		let Some(ptr) = self.0 else {
			return Ok(false);
		};
		unsafe {
			copy_from_user_raw(
				ptr.as_ptr().add(off) as *const _,
				buf.as_mut_ptr() as *mut _,
				size_of_val(buf),
			)?;
		}
		Ok(true)
	}

	/// Same as [`Self::copy_from_user`], except the function allocates and returns a [`Vec`]
	/// instead of copying to a provided buffer.
	///
	/// If the pointer is null, the function returns `None`.
	pub fn copy_from_user_vec(&self, off: usize, len: usize) -> EResult<Option<Vec<T>>> {
		let Some(ptr) = self.0 else {
			return Ok(None);
		};
		let mut buf = Vec::with_capacity(len)?;
		unsafe {
			buf.set_len(len);
			copy_from_user_raw(
				ptr.as_ptr().add(off) as *const _,
				buf.as_mut_ptr() as *mut _,
				size_of::<T>() * len,
			)?;
		}
		Ok(Some(buf))
	}

	/// Copies the value to userspace.
	///
	/// Arguments:
	/// - `off` is the byte offset in the slice to which the data is to be copied.
	/// - `val` is the source slice to copy from.
	///
	/// If the pointer is null, the function does nothing.
	///
	/// If the slice is not accessible, the function returns an error.
	///
	/// If the slice is located on lazily allocated pages, the function
	/// allocates physical pages in order to allow writing.
	pub fn copy_to_user(&self, off: usize, val: &[T]) -> EResult<()> {
		let Some(ptr) = self.0 else {
			return Ok(());
		};
		unsafe {
			copy_to_user_raw(
				val.as_ptr() as *const _,
				ptr.as_ptr().add(off) as *mut _,
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
	fn from_syscall_arg(ptr: usize, _compat: bool) -> Self {
		Self(NonNull::new(ptr::with_exposed_provenance_mut(ptr)))
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
			// May not wrap since the chunk size is obviously lower than the size of the
			// kernelspace
			let user_cursor = ptr.as_ptr().wrapping_add(buf_cursor);
			let page_end = PAGE_SIZE - (user_cursor as usize % PAGE_SIZE);
			let len = min(page_end, CHUNK_SIZE);
			// Read the next chunk
			buf.reserve(len)?;
			unsafe {
				buf.set_len(buf_cursor + len);
				copy_from_user_raw(user_cursor, &mut buf[buf_cursor], len)?;
			}
			// Look for a nul byte
			let nul_off = buf[buf_cursor..(buf_cursor + len)]
				.iter()
				.position(|b| *b == b'\0');
			if let Some(i) = nul_off {
				buf.truncate(buf_cursor + i);
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
// Using `c_void` as the size of the pointer is different when using compat mode
pub struct SyscallArray {
	/// The array's pointer
	ptr: Option<NonNull<*const u8>>,
	/// If true, pointers are 4 bytes in size, else 8 bytes
	compat: bool,
}

impl FromSyscallArg for SyscallArray {
	fn from_syscall_arg(ptr: usize, compat: bool) -> Self {
		Self {
			ptr: NonNull::new(ptr::with_exposed_provenance_mut(ptr)),
			compat,
		}
	}
}

impl SyscallArray {
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

impl SyscallArrayIterator<'_> {
	fn next_impl(&mut self) -> EResult<Option<String>> {
		let Some(ptr) = self.arr.ptr else {
			return Err(errno!(EFAULT));
		};
		let str_ptr = if self.arr.compat {
			let str_ptr = unsafe { ptr.cast::<u32>().add(self.i) };
			let str_ptr = SyscallPtr(Some(str_ptr)).copy_from_user()?.unwrap();
			ptr::without_provenance(str_ptr as usize)
		} else {
			let str_ptr = unsafe { ptr.add(self.i) };
			SyscallPtr(Some(str_ptr)).copy_from_user()?.unwrap()
		};
		let res = SyscallString(NonNull::new(str_ptr as _)).copy_from_user()?;
		// Do not increment if reaching `NULL`
		if likely(res.is_some()) {
			self.i += 1;
		}
		Ok(res)
	}
}

impl Iterator for SyscallArrayIterator<'_> {
	type Item = EResult<String>;

	fn next(&mut self) -> Option<Self::Item> {
		self.next_impl().transpose()
	}
}

/// An entry of an IO vector used for scatter/gather IO.
#[repr(C)]
#[derive(Clone, Debug)]
pub struct IOVec {
	/// Starting address.
	pub iov_base: *mut c_void,
	/// Number of bytes to transfer.
	pub iov_len: usize,
}

/// An [`IOVec`] for compatibility mode.
#[repr(C)]
#[derive(Clone, Debug)]
struct IOVecCompat {
	/// Starting address.
	pub iov_base: u32,
	/// Number of bytes to transfer.
	pub iov_len: u32,
}

/// An [`IOVec`] as a system call argument.
pub struct SyscallIOVec {
	/// The pointer to the iovec.
	ptr: Option<NonNull<u8>>,
	/// Tells whether the userspace is in compatibility mode.
	compat: bool,
}

impl FromSyscallArg for SyscallIOVec {
	fn from_syscall_arg(ptr: usize, compat: bool) -> Self {
		Self {
			ptr: NonNull::new(ptr::with_exposed_provenance_mut(ptr)),
			compat,
		}
	}
}

impl fmt::Debug for SyscallIOVec {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self.ptr {
			Some(ptr) => write!(fmt, "{ptr:p}"),
			None => write!(fmt, "NULL"),
		}
	}
}

impl SyscallIOVec {
	/// Returns an iterator over the iovec.
	///
	/// `count` is the number of elements in the vector.
	pub fn iter(&self, count: usize) -> IOVecIter {
		IOVecIter {
			vec: self,
			cursor: 0,
			count,
		}
	}
}

/// Iterator over [`IOVec`]s.
pub struct IOVecIter<'a> {
	/// The iovec pointer.
	vec: &'a SyscallIOVec,
	/// Cursor
	cursor: usize,
	/// The number of elements.
	count: usize,
}

impl Iterator for IOVecIter<'_> {
	type Item = EResult<IOVec>;

	fn next(&mut self) -> Option<Self::Item> {
		let stride = if self.vec.compat {
			size_of::<IOVecCompat>()
		} else {
			size_of::<IOVec>()
		};
		// Bound check
		if unlikely(self.cursor >= self.count * stride) {
			return None;
		}
		let iov = unsafe {
			let ptr = self.vec.ptr?.byte_add(self.cursor);
			if self.vec.compat {
				let mut iov = MaybeUninit::<IOVecCompat>::uninit();
				copy_from_user_raw(
					ptr.as_ptr(),
					iov.as_mut_ptr() as *mut _,
					size_of::<IOVecCompat>(),
				)
				.map(|_| {
					let iovec = iov.assume_init();
					IOVec {
						iov_base: ptr::with_exposed_provenance_mut(iovec.iov_base as _),
						iov_len: iovec.iov_len as _,
					}
				})
			} else {
				let mut iov = MaybeUninit::<IOVec>::uninit();
				copy_from_user_raw(ptr.as_ptr(), iov.as_mut_ptr() as *mut _, size_of::<IOVec>())
					.map(|_| iov.assume_init())
			}
		};
		self.cursor += stride;
		Some(iov)
	}
}

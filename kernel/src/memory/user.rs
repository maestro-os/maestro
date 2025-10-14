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
	fmt,
	hint::{likely, unlikely},
	marker::PhantomData,
	mem::{MaybeUninit, size_of},
	ptr,
	ptr::NonNull,
};
use utils::{
	collections::{path::PathBuf, string::String, vec::Vec},
	errno,
	errno::EResult,
	limits::PAGE_SIZE,
};

unsafe extern "C" {
	/// Copy, with access check. On success, the function returns `true`.
	pub fn raw_copy(dst: *mut u8, src: *const u8, n: usize) -> bool;
	/// Function to be called back when a page fault occurs while using [`raw_copy`].
	pub fn copy_fault();
}

/// Low level function to copy data from userspace to kernelspace, with access check.
///
/// If the access check fails, the function returns [`EFAULT`].
unsafe fn user_copy(src: *const u8, dst: *mut u8, n: usize) -> EResult<()> {
	let res = vmem::smap_disable(|| raw_copy(dst, src, n));
	if likely(res) {
		Ok(())
	} else {
		Err(errno!(EFAULT))
	}
}

/// Wrapper for an userspace pointer.
#[derive(Clone, Copy)]
pub struct UserPtr<T: Sized + fmt::Debug>(pub Option<NonNull<T>>);

impl<T: Sized + fmt::Debug> FromSyscallArg for UserPtr<T> {
	fn from_syscall_arg(ptr: usize, _compat: bool) -> Self {
		Self(NonNull::new(ptr::with_exposed_provenance_mut(ptr)))
	}
}

impl<T: Sized + fmt::Debug> UserPtr<T> {
	/// Returns a mutable pointer to the data.
	pub fn as_ptr(&self) -> *mut T {
		self.0.map(NonNull::as_ptr).unwrap_or_default()
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
		if unlikely(!bound_check(self.as_ptr() as _, size_of::<T>())) {
			return Err(errno!(EFAULT));
		}
		unsafe {
			let mut val = MaybeUninit::<T>::uninit();
			user_copy(
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
		if unlikely(!bound_check(self.as_ptr() as _, size_of::<T>())) {
			return Err(errno!(EFAULT));
		}
		unsafe {
			user_copy(
				val as *const _ as *const _,
				ptr.as_ptr() as *mut _,
				size_of::<T>(),
			)?;
		}
		Ok(())
	}
}

impl<T: fmt::Debug> fmt::Debug for UserPtr<T> {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		let ptr = self.as_ptr();
		match self.copy_from_user() {
			Ok(Some(val)) => write!(fmt, "{ptr:p} = {val:?}"),
			Ok(None) => write!(fmt, "NULL"),
			Err(e) => write!(fmt, "{ptr:p} = (cannot read: {e})"),
		}
	}
}

/// Wrapper for an userspace slice of memory.
///
/// The size of the slice is required when trying to access it.
#[derive(Clone, Copy)]
pub struct UserSlice<'a, T: Sized + fmt::Debug> {
	/// Pointer at the start of the slice
	ptr: Option<NonNull<T>>,
	/// The length of the slice, in number of elements
	len: usize,

	phantom: PhantomData<&'a T>,
}

impl<T: Sized + fmt::Debug> UserSlice<'static, T> {
	/// Creates a new instance from a user-provided pointer `ptr`, with length `len`.
	///
	/// If `ptr` is out of bounds of the userspace, the function returns [`errno::EFAULT`].
	pub fn from_user(ptr: *mut T, len: usize) -> EResult<Self> {
		let ptr = NonNull::new(ptr);
		if let Some(ptr) = ptr {
			if unlikely(!bound_check(ptr.as_ptr() as _, len)) {
				return Err(errno!(EFAULT));
			}
		}
		Ok(Self {
			ptr,
			len,

			phantom: PhantomData,
		})
	}
}

impl<'a, T: Sized + fmt::Debug> UserSlice<'a, T> {
	/// Creates a new instance from a kernel immutable slice.
	///
	/// Contrary to [`Self::from_user`], this function does not check whether `ptr` is out of
	/// bounds.
	///
	/// # Safety
	///
	/// The resulting [`UserSlice`] must not be used to write to `slice`.
	pub unsafe fn from_slice(slice: &'a [T]) -> Self {
		Self {
			ptr: NonNull::new(slice.as_ptr() as _),
			len: slice.len(),

			phantom: PhantomData,
		}
	}

	/// Creates a new instance from a kernel mutable slice.
	///
	/// Contrary to [`Self::from_user`], this function does not check whether `ptr` is out of
	/// bounds.
	pub fn from_slice_mut(slice: &'a mut [T]) -> Self {
		Self {
			ptr: NonNull::new(slice.as_mut_ptr() as _),
			len: slice.len(),

			phantom: PhantomData,
		}
	}

	/// Returns a mutable pointer to the data.
	pub fn as_ptr(&self) -> *mut T {
		self.ptr.map(NonNull::as_ptr).unwrap_or_default()
	}

	/// Returns the length of the slice in number of elements.
	#[inline]
	pub fn len(&self) -> usize {
		self.len
	}

	/// Tells whether the slice is empty.
	#[inline]
	pub fn is_empty(&self) -> bool {
		self.len == 0
	}

	/// Same as [`Self::copy_from_user`], with a pointer `ptr` and length `len` instead of a slice.
	///
	/// # Safety
	///
	/// If the pointer/length pair does not point to a valid chunk of memory, the behaviour is
	/// undefined.
	pub unsafe fn copy_from_user_raw(
		&self,
		off: usize,
		dst: *mut T,
		len: usize,
	) -> EResult<usize> {
		let Some(ptr) = self.ptr else {
			return Ok(0);
		};
		let len = min(len, self.len.saturating_sub(off));
		user_copy(
			ptr.as_ptr().add(off) as *const _,
			dst as *mut _,
			size_of::<T>() * len,
		)?;
		Ok(len)
	}

	/// Copies the slice from userspace.
	///
	/// Arguments:
	/// - `off` is the offset relative to the beginning of the userspace slice
	/// - `buf` is the destination slice
	///
	/// The function returns the number of elements written.
	///
	/// If the pointer is null, the function does nothing and returns `0`.
	///
	/// If the slice is not accessible, the function returns an error.
	pub fn copy_from_user(&self, off: usize, buf: &mut [T]) -> EResult<usize> {
		unsafe { self.copy_from_user_raw(off, buf.as_mut_ptr(), buf.len()) }
	}

	/// Same as [`Self::copy_from_user`], except the function allocates and returns a [`Vec`]
	/// instead of copying to a provided buffer.
	///
	/// If the pointer is null, the function returns `None`.
	pub fn copy_from_user_vec(&self, off: usize) -> EResult<Option<Vec<T>>> {
		let Some(ptr) = self.ptr else {
			return Ok(None);
		};
		let len = self.len.saturating_sub(off);
		let mut buf = Vec::with_capacity(len)?;
		unsafe {
			buf.set_len(len);
			user_copy(
				ptr.as_ptr().add(off) as *const _,
				buf.as_mut_ptr() as *mut _,
				size_of::<T>() * len,
			)?;
		}
		Ok(Some(buf))
	}

	/// Same as [`Self::copy_to_user`], with a pointer `ptr` and length `len` instead of a slice.
	///
	/// # Safety
	///
	/// If the pointer/length pair does not point to a valid chunk of memory, the behaviour is
	/// undefined.
	pub unsafe fn copy_to_user_raw(
		&self,
		off: usize,
		src: *const T,
		len: usize,
	) -> EResult<usize> {
		let Some(ptr) = self.ptr else {
			return Ok(0);
		};
		let len = min(len, self.len.saturating_sub(off));
		user_copy(
			src as *const _,
			ptr.as_ptr().add(off) as *mut _,
			size_of::<T>() * len,
		)?;
		Ok(len)
	}

	/// Copies the slice to userspace.
	///
	/// Arguments:
	/// - `off` is the byte offset in the slice to which the data is to be copied
	/// - `buf` is the source slice to copy from
	///
	/// The function returns the number of elements written.
	///
	/// If the pointer is null, the function does nothing and returns `0`.
	///
	/// If the slice is not accessible, the function returns an error.
	pub fn copy_to_user(&self, off: usize, buf: &[T]) -> EResult<usize> {
		unsafe { self.copy_to_user_raw(off, buf.as_ptr(), buf.len()) }
	}
}

impl<T: fmt::Debug> fmt::Debug for UserSlice<'_, T> {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self.ptr {
			Some(ptr) => write!(fmt, "{ptr:p}"),
			None => write!(fmt, "NULL"),
		}
	}
}

/// Wrapper for a C-style, nul-terminated (`\0`) userspace string.
#[derive(Clone, Copy)]
pub struct UserString(pub Option<NonNull<u8>>);

impl FromSyscallArg for UserString {
	fn from_syscall_arg(ptr: usize, _compat: bool) -> Self {
		Self(NonNull::new(ptr::with_exposed_provenance_mut(ptr)))
	}
}

impl UserString {
	/// Returns an immutable pointer to the data.
	pub fn as_ptr(&self) -> *const u8 {
		self.0.map(NonNull::as_ptr).unwrap_or_default()
	}

	/// Copies a string from userspace.
	///
	/// If the pointer is `NULL`, the function returns `None`.
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
			if unlikely(!bound_check(user_cursor as _, len)) {
				return Err(errno!(EFAULT));
			}
			// Read the next chunk
			buf.reserve(len)?;
			unsafe {
				buf.set_len(buf_cursor + len);
				user_copy(user_cursor, &mut buf[buf_cursor], len)?;
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

	/// Copies a [`PathBuf`] from userspace.
	///
	/// If the pointer is `NULL`, the function returns `None`.
	///
	/// If the string is not accessible, the function returns an error.
	pub fn copy_path_opt_from_user(&self) -> EResult<Option<PathBuf>> {
		self.copy_from_user()?.map(PathBuf::try_from).transpose()
	}

	/// Copies a [`PathBuf`] from userspace.
	///
	/// If the string is not accessible, the function returns an error.
	pub fn copy_path_from_user(&self) -> EResult<PathBuf> {
		self.copy_path_opt_from_user()?
			.ok_or_else(|| errno!(EFAULT))
	}
}

impl fmt::Debug for UserString {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		let ptr = self.as_ptr();
		match self.copy_from_user() {
			Ok(Some(s)) => write!(fmt, "{ptr:p} = {s:?}"),
			Ok(None) => write!(fmt, "NULL"),
			Err(e) => write!(fmt, "{ptr:p} = (cannot read: {e})"),
		}
	}
}

/// Wrapper for a C-style, NULL-terminated userspace string array.
#[derive(Clone, Copy)]
pub struct UserArray {
	/// The array's pointer
	ptr: Option<NonNull<*const u8>>,
	/// If true, pointers are 4 bytes in size, else 8 bytes
	compat: bool,
}

impl FromSyscallArg for UserArray {
	fn from_syscall_arg(ptr: usize, compat: bool) -> Self {
		Self {
			ptr: NonNull::new(ptr::with_exposed_provenance_mut(ptr)),
			compat,
		}
	}
}

impl UserArray {
	/// Returns an iterator over the array's elements.
	pub fn iter(&self) -> UserArrayIterator {
		UserArrayIterator {
			arr: self,
			i: 0,
		}
	}
}

impl fmt::Debug for UserArray {
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

/// Iterators over elements of [`UserArray`].
pub struct UserArrayIterator<'a> {
	arr: &'a UserArray,
	i: usize,
}

impl UserArrayIterator<'_> {
	fn next_impl(&mut self) -> EResult<Option<String>> {
		let Some(ptr) = self.arr.ptr else {
			return Ok(None);
		};
		let str_ptr = if self.arr.compat {
			let str_ptr = unsafe { ptr.cast::<u32>().add(self.i) };
			let str_ptr = UserPtr(Some(str_ptr)).copy_from_user()?.unwrap();
			ptr::without_provenance(str_ptr as usize)
		} else {
			let str_ptr = unsafe { ptr.add(self.i) };
			UserPtr(Some(str_ptr)).copy_from_user()?.unwrap()
		};
		let res = UserString(NonNull::new(str_ptr as _)).copy_from_user()?;
		// Do not increment if reaching `NULL`
		if likely(res.is_some()) {
			self.i += 1;
		}
		Ok(res)
	}
}

impl Iterator for UserArrayIterator<'_> {
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
	pub iov_base: *mut u8,
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
pub struct UserIOVec {
	/// The pointer to the iovec.
	ptr: Option<NonNull<u8>>,
	/// Tells whether the userspace is in compatibility mode.
	compat: bool,
}

impl FromSyscallArg for UserIOVec {
	fn from_syscall_arg(ptr: usize, compat: bool) -> Self {
		Self {
			ptr: NonNull::new(ptr::with_exposed_provenance_mut(ptr)),
			compat,
		}
	}
}

impl fmt::Debug for UserIOVec {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self.ptr {
			Some(ptr) => write!(fmt, "{ptr:p}"),
			None => write!(fmt, "NULL"),
		}
	}
}

impl UserIOVec {
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
	vec: &'a UserIOVec,
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
				let ptr = UserPtr::<IOVecCompat>(Some(ptr.cast()));
				ptr.copy_from_user().transpose()?.map(|iov| IOVec {
					iov_base: ptr::with_exposed_provenance_mut(iov.iov_base as _),
					iov_len: iov.iov_len as _,
				})
			} else {
				let ptr = UserPtr::<IOVec>(Some(ptr.cast()));
				ptr.copy_from_user().transpose()?
			}
		};
		self.cursor += stride;
		Some(iov)
	}
}

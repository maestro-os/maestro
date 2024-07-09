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

use crate::{
	memory::vmem,
	process::{mem_space::MemSpace, Process},
	syscall::FromSyscallArg,
};
use core::{
	fmt,
	mem::{size_of, size_of_val},
	ptr,
	ptr::{null_mut, NonNull},
};
use utils::{
	collections::{string::String, vec::Vec},
	errno,
	errno::EResult,
};

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
	pub fn copy_from_user(&self, mem_space: &MemSpace) -> EResult<Option<T>> {
		let Some(ptr) = self.0 else {
			return Ok(None);
		};
		if !mem_space.can_access(ptr.as_ptr() as _, size_of::<T>(), true, false) {
			return Err(errno!(EFAULT));
		}
		// Safe because access is checked before
		let val = unsafe { vmem::smap_disable(|| ptr::read_volatile(ptr.as_ref())) };
		Ok(Some(val))
	}

	/// Copies the value to userspace.
	///
	/// If the pointer is null, the function does nothing.
	///
	/// If the value is not accessible, the function returns an error.
	///
	/// If the value is located on lazily allocated pages, the function
	/// allocates physical pages in order to allow writing.
	pub fn copy_to_user(&self, mem_space: &mut MemSpace, val: T) -> EResult<()> {
		let Some(mut ptr) = self.0 else {
			return Ok(());
		};
		if !mem_space.can_access(ptr.as_ptr() as _, size_of::<T>(), true, true) {
			return Err(errno!(EFAULT));
		}
		// Allocate memory to make sure it is writable
		mem_space.alloc(ptr.as_ptr() as _, size_of::<T>())?;
		// Safe because access is checked before
		unsafe {
			vmem::smap_disable(|| {
				ptr::write_volatile(ptr.as_mut(), val);
			});
		}
		Ok(())
	}
}

impl<T: fmt::Debug> fmt::Debug for SyscallPtr<T> {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();
		let mem_space_mutex = proc.get_mem_space().unwrap();
		let mem_space = mem_space_mutex.lock();
		let ptr = self.as_ptr();
		match self.copy_from_user(&mem_space) {
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
	pub fn copy_from_user(&self, mem_space: &MemSpace, len: usize) -> EResult<Option<Vec<T>>> {
		let Some(ptr) = self.0 else {
			return Ok(None);
		};
		let size = size_of::<T>() * len;
		if !mem_space.can_access(ptr.as_ptr() as _, size, true, false) {
			return Err(errno!(EFAULT));
		}
		let mut arr = Vec::with_capacity(len)?;
		// Safe because access is checked before
		unsafe {
			arr.set_len(len);
			vmem::smap_disable(|| {
				ptr::copy_nonoverlapping(ptr.as_ref(), arr.as_mut_ptr(), len);
			});
		}
		Ok(Some(arr))
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
	pub fn copy_to_user(&self, mem_space: &mut MemSpace, val: &[T]) -> EResult<()> {
		let Some(mut ptr) = self.0 else {
			return Ok(());
		};
		let size = size_of_val(val);
		if !mem_space.can_access(ptr.as_ptr() as _, size, true, true) {
			return Err(errno!(EFAULT));
		}
		// Allocate memory to make sure it is writable
		mem_space.alloc(ptr.as_ptr() as _, size)?;
		// Safe because access is checked before
		unsafe {
			vmem::smap_disable(|| {
				ptr::copy_nonoverlapping(val.as_ptr(), ptr.as_mut(), val.len());
			});
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
	pub fn copy_from_user(&self, mem_space: &MemSpace) -> EResult<Option<String>> {
		let Some(ptr) = self.0 else {
			return Ok(None);
		};
		// FIXME: data race
		let len = mem_space
			.can_access_string(ptr.as_ptr(), true, false)
			.ok_or_else(|| errno!(EFAULT))?;
		// Safe because access is checked before
		let mut arr = Vec::with_capacity(len)?;
		// Safe because access is checked before
		unsafe {
			arr.set_len(len);
			vmem::smap_disable(|| {
				ptr::copy_nonoverlapping(ptr.as_ref(), arr.as_mut_ptr(), len);
			});
		}
		Ok(Some(arr.into()))
	}
}

impl fmt::Debug for SyscallString {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();
		let mem_space_mutex = proc.get_mem_space().unwrap();
		let mem_space = mem_space_mutex.lock();
		let ptr = self.as_ptr();
		match self.copy_from_user(&mem_space) {
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
	pub fn iter<'a>(&'a self, mem_space: &'a MemSpace) -> SyscallArrayIterator {
		SyscallArrayIterator {
			mem_space,
			arr: self,
			i: 0,
		}
	}
}

impl fmt::Debug for SyscallArray {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();
		let mem_space_mutex = proc.get_mem_space().unwrap();
		let mem_space = mem_space_mutex.lock();
		let mut list = fmt.debug_list();
		let mut list_ref = &mut list;
		for elem in self.iter(&mem_space) {
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
	/// The memory space.
	mem_space: &'a MemSpace,
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
			.copy_from_user(self.mem_space)
			.transpose();
		// Do not increment if reaching `NULL`
		if res.is_some() {
			self.i += 1;
		}
		res
	}
}

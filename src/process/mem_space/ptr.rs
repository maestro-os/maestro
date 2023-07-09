//! When a pointer is passed to the kernel through a system call, the kernel is
//! required to check the process is allowed to access it to ensure safety.
//!
//! This module implements objets that wrap pointers in order to check they are
//! accessible.
//!
//! Those structure are especially useful in the cases where several processes
//! share the same memory space, making it possible to revoke the access to the
//! pointer while it is being used.

use super::MemSpace;
use crate::errno::Errno;
use crate::process::Process;
use crate::util::DisplayableStr;
use core::fmt;
use core::mem::size_of;
use core::slice;

/// Wrapper for a pointer to a simple data.
pub struct SyscallPtr<T: Sized> {
	/// The pointer.
	ptr: *mut T,
}

impl<T: Sized> From<usize> for SyscallPtr<T> {
	fn from(val: usize) -> Self {
		Self {
			ptr: val as _,
		}
	}
}

impl<T: Sized> SyscallPtr<T> {
	/// Tells whether the pointer is null.
	pub fn is_null(&self) -> bool {
		self.ptr.is_null()
	}

	/// Returns an immutable pointer to the the data.
	pub fn as_ptr(&self) -> *const T {
		self.ptr
	}

	/// Returns a mutable pointer to the the data.
	pub fn as_ptr_mut(&self) -> *mut T {
		self.ptr
	}

	/// Returns an immutable reference to the value of the pointer.
	///
	/// If the pointer is null, the function returns `None`.
	///
	/// If the value is not accessible, the function returns an error.
	pub fn get<'a>(&self, mem_space: &'a MemSpace) -> Result<Option<&'a T>, Errno> {
		if self.is_null() {
			return Ok(None);
		}

		if mem_space.can_access(self.ptr as _, size_of::<T>(), true, false) {
			Ok(Some(unsafe {
				// Safe because access is checked before
				&*self.ptr
			}))
		} else {
			Err(errno!(EFAULT))
		}
	}

	/// Returns a mutable reference to the value of the pointer.
	///
	/// If the pointer is null, the function returns `None`.
	///
	/// If the value is not accessible, the function returns an error.
	///
	/// If the value is located on lazily allocated pages, the function
	/// allocates physical pages in order to allow writing.
	pub fn get_mut<'a>(&self, mem_space: &'a mut MemSpace) -> Result<Option<&'a mut T>, Errno> {
		if self.is_null() {
			return Ok(None);
		}

		if mem_space.can_access(self.ptr as _, size_of::<T>(), true, true) {
			// Allocating physical pages if necessary
			mem_space.alloc(self.ptr, 1)?;

			Ok(Some(unsafe {
				// Safe because access is checked before
				&mut *self.ptr
			}))
		} else {
			Err(errno!(EFAULT))
		}
	}
}

impl<T: fmt::Debug> fmt::Debug for SyscallPtr<T> {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space_mutex = proc.get_mem_space().unwrap();
		let mem_space = mem_space_mutex.lock();

		match self.get(&mem_space) {
			Ok(Some(s)) => write!(fmt, "{:p} = {:?}", self.as_ptr(), s),

			Ok(None) => write!(fmt, "NULL"),

			Err(e) => write!(fmt, "{:p} = (cannot read: {e})", self.as_ptr()),
		}
	}
}

/// Wrapper for a slice.
///
/// Internally, the structure contains only a pointer.
///
/// The size of the slice is given when trying to access it.
pub struct SyscallSlice<T: Sized> {
	/// The pointer.
	ptr: *mut T,
}

impl<T: Sized> From<usize> for SyscallSlice<T> {
	fn from(val: usize) -> Self {
		Self {
			ptr: val as _,
		}
	}
}

impl<T: Sized> SyscallSlice<T> {
	/// Tells whether the pointer is null.
	pub fn is_null(&self) -> bool {
		self.ptr.is_null()
	}

	/// Returns an immutable pointer to the the data.
	pub fn as_ptr(&self) -> *const T {
		self.ptr
	}

	/// Returns a mutable pointer to the the data.
	pub fn as_ptr_mut(&self) -> *mut T {
		self.ptr
	}

	/// Returns an immutable reference to the slice.
	///
	/// `len` is the in number of elements in the slice.
	///
	/// If the slice is not accessible, the function returns an error.
	pub fn get<'a>(&self, mem_space: &'a MemSpace, len: usize) -> Result<Option<&'a [T]>, Errno> {
		if self.is_null() {
			return Ok(None);
		}

		let size = size_of::<T>() * len;
		if mem_space.can_access(self.ptr as _, size, true, false) {
			Ok(Some(unsafe {
				// Safe because access is checked before
				slice::from_raw_parts(self.ptr, len)
			}))
		} else {
			Err(errno!(EFAULT))
		}
	}

	/// Returns a mutable reference to the slice.
	///
	/// `len` is the in number of elements in the slice.
	///
	/// If the slice is not accessible, the function returns an error.
	///
	/// If the slice is located on lazily allocated pages, the function
	/// allocates physical pages in order to allow writing.
	pub fn get_mut<'a>(
		&self,
		mem_space: &'a mut MemSpace,
		len: usize,
	) -> Result<Option<&'a mut [T]>, Errno> {
		if self.is_null() {
			return Ok(None);
		}

		let size = size_of::<T>() * len;
		if mem_space.can_access(self.ptr as _, size, true, true) {
			// Allocating physical pages if necessary
			mem_space.alloc(self.ptr as *const u8, len)?;

			Ok(Some(unsafe {
				// Safe because access is checked before
				slice::from_raw_parts_mut(self.ptr, len)
			}))
		} else {
			Err(errno!(EFAULT))
		}
	}
}

impl<T: fmt::Debug> fmt::Debug for SyscallSlice<T> {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		// TODO Print value? (how to get the length of the slice?)
		let ptr = self.as_ptr();

		if !ptr.is_null() {
			write!(fmt, "{:p}", ptr)
		} else {
			write!(fmt, "NULL")
		}
	}
}

/// Wrapper for a string. Internally, the structure contains only a pointer.
pub struct SyscallString {
	/// The pointer.
	ptr: *mut u8,
}

impl From<usize> for SyscallString {
	fn from(val: usize) -> Self {
		Self {
			ptr: val as _,
		}
	}
}

impl SyscallString {
	/// Tells whether the pointer is null.
	pub fn is_null(&self) -> bool {
		self.ptr.is_null()
	}

	/// Returns an immutable pointer to the the data.
	pub fn as_ptr(&self) -> *const u8 {
		self.ptr
	}

	/// Returns a mutable pointer to the the data.
	pub fn as_ptr_mut(&self) -> *mut u8 {
		self.ptr
	}

	/// Returns an immutable reference to the string.
	///
	/// If the string is not accessible, the function returns an error.
	pub fn get<'a>(&self, mem_space: &'a MemSpace) -> Result<Option<&'a [u8]>, Errno> {
		if self.is_null() {
			return Ok(None);
		}

		let len = mem_space
			.can_access_string(self.ptr, true, false)
			.ok_or_else(|| errno!(EFAULT))?;
		Ok(Some(unsafe {
			// Safe because access is checked before
			slice::from_raw_parts(self.ptr, len)
		}))
	}

	/// Returns a mutable reference to the string.
	///
	/// If the string is located on lazily allocated pages, the function
	/// allocates physical pages in order to allow writing.
	///
	/// If the string is not accessible, the function returns an error.
	pub fn get_mut<'a>(&self, mem_space: &'a mut MemSpace) -> Result<Option<&'a mut [u8]>, Errno> {
		if self.is_null() {
			return Ok(None);
		}

		let len = mem_space
			.can_access_string(self.ptr, true, true)
			.ok_or_else(|| errno!(EFAULT))?;

		// Allocating physical pages if necessary
		mem_space.alloc(self.ptr as *const u8, len)?;

		Ok(Some(unsafe {
			// Safe because access is checked before
			slice::from_raw_parts_mut(self.ptr, len)
		}))
	}
}

impl fmt::Debug for SyscallString {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space_mutex = proc.get_mem_space().unwrap();
		let mem_space = mem_space_mutex.lock();

		match self.get(&mem_space) {
			Ok(Some(s)) => {
				// TODO Add backslashes to escape `"` and `\`

				let s = DisplayableStr(s);
				write!(fmt, "{:p} = \"{}\"", self.as_ptr(), s)
			}

			Ok(None) => write!(fmt, "NULL"),

			Err(e) => write!(fmt, "{:p} = (cannot read: {e})", self.as_ptr()),
		}
	}
}

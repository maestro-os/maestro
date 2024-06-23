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

//! A system call is "function" that allows to communicate between userspace and kernelspace.
//!
//! Documentation for each system call can be retrieved from the man. Type the
//! command: `man 2 <syscall>`

#![allow(unused_imports)]

mod _exit;
mod _llseek;
mod _newselect;
mod access;
mod arch_prctl;
mod bind;
mod r#break;
mod brk;
mod chdir;
mod chmod;
mod chown;
mod chown32;
mod chroot;
mod clock_gettime;
mod clock_gettime64;
mod clone;
mod close;
mod connect;
mod creat;
mod delete_module;
mod dup;
mod dup2;
mod execve;
mod exit_group;
mod faccessat;
mod faccessat2;
mod fadvise64_64;
mod fchdir;
mod fchmod;
mod fchmodat;
mod fcntl;
mod fcntl64;
mod finit_module;
mod fork;
mod fstat64;
mod fstatfs;
mod fstatfs64;
mod fsync;
mod getcwd;
mod getdents;
mod getdents64;
mod getegid;
mod getegid32;
mod geteuid;
mod geteuid32;
mod getgid;
mod getgid32;
mod getpgid;
mod getpid;
mod getppid;
mod getrandom;
mod getrusage;
mod getsockname;
mod getsockopt;
mod gettid;
mod getuid;
mod getuid32;
mod init_module;
pub mod ioctl;
mod kill;
mod lchown;
mod link;
mod linkat;
mod madvise;
mod mkdir;
mod mknod;
mod mmap;
mod mmap2;
mod mount;
mod mprotect;
mod msync;
mod munmap;
mod nanosleep;
mod open;
mod openat;
mod pipe;
mod pipe2;
mod poll;
mod preadv;
mod preadv2;
mod prlimit64;
mod pselect6;
mod pwritev;
mod pwritev2;
mod read;
mod readlink;
mod readv;
mod reboot;
mod rename;
mod renameat2;
mod rmdir;
mod rt_sigaction;
mod rt_sigprocmask;
mod sched_yield;
mod select;
mod sendto;
mod set_thread_area;
mod set_tid_address;
mod setgid;
mod setgid32;
mod sethostname;
mod setpgid;
mod setsockopt;
mod setuid;
mod setuid32;
mod shutdown;
mod signal;
mod sigreturn;
mod socket;
mod socketpair;
mod splice;
mod statfs;
mod statfs64;
mod statx;
mod symlink;
mod symlinkat;
mod syncfs;
mod time;
mod timer_create;
mod timer_delete;
mod timer_settime;
mod tkill;
mod truncate;
mod umask;
mod umount;
mod uname;
mod unlink;
mod unlinkat;
mod util;
mod utimensat;
mod vfork;
mod wait;
mod wait4;
mod waitpid;
mod write;
mod writev;

//use wait::wait;
use crate::process::{mem_space::MemSpace, regs::Regs, signal::Signal, Process};
use _exit::_exit;
use _llseek::_llseek;
use _newselect::_newselect;
use access::access;
use arch_prctl::arch_prctl;
use bind::bind;
use brk::brk;
use chdir::chdir;
use chmod::chmod;
use chown::chown;
use chown32::chown32;
use chroot::chroot;
use clock_gettime::clock_gettime;
use clock_gettime64::clock_gettime64;
use clone::clone;
use close::close;
use connect::connect;
use core::{
	fmt,
	mem::size_of,
	ptr::{null, null_mut, NonNull},
	slice,
};
use creat::creat;
use delete_module::delete_module;
use dup::dup;
use dup2::dup2;
use execve::execve;
use exit_group::exit_group;
use faccessat::faccessat;
use faccessat2::faccessat2;
use fadvise64_64::fadvise64_64;
use fchdir::fchdir;
use fchmod::fchmod;
use fchmodat::fchmodat;
use fcntl::fcntl;
use fcntl64::fcntl64;
use finit_module::finit_module;
use fork::fork;
use fstat64::fstat64;
use fstatfs::fstatfs;
use fstatfs64::fstatfs64;
use fsync::fsync;
use getcwd::getcwd;
use getdents::getdents;
use getdents64::getdents64;
use getegid::getegid;
use getegid32::getegid32;
use geteuid::geteuid;
use geteuid32::geteuid32;
use getgid::getgid;
use getgid32::getgid32;
use getpgid::getpgid;
use getpid::getpid;
use getppid::getppid;
use getrandom::getrandom;
use getrusage::getrusage;
use getsockname::getsockname;
use getsockopt::getsockopt;
use gettid::gettid;
use getuid::getuid;
use getuid32::getuid32;
use init_module::init_module;
use ioctl::ioctl;
use kill::kill;
use lchown::lchown;
use link::link;
use linkat::linkat;
use madvise::madvise;
use mkdir::mkdir;
use mknod::mknod;
use mmap::mmap;
use mmap2::mmap2;
use mount::mount;
use mprotect::mprotect;
use msync::msync;
use munmap::munmap;
use nanosleep::nanosleep;
use open::open;
use openat::openat;
use pipe::pipe;
use pipe2::pipe2;
use poll::poll;
use preadv::preadv;
use preadv2::preadv2;
use prlimit64::prlimit64;
use pselect6::pselect6;
use pwritev::pwritev;
use pwritev2::pwritev2;
use r#break::r#break;
use read::read;
use readlink::readlink;
use readv::readv;
use reboot::reboot;
use rename::rename;
use renameat2::renameat2;
use rmdir::rmdir;
use rt_sigaction::rt_sigaction;
use rt_sigprocmask::rt_sigprocmask;
use sched_yield::sched_yield;
use select::select;
use sendto::sendto;
use set_thread_area::set_thread_area;
use set_tid_address::set_tid_address;
use setgid::setgid;
use setgid32::setgid32;
use sethostname::sethostname;
use setpgid::setpgid;
use setsockopt::setsockopt;
use setuid::setuid;
use setuid32::setuid32;
use shutdown::shutdown;
use signal::signal;
use sigreturn::sigreturn;
use socket::socket;
use socketpair::socketpair;
use splice::splice;
use statfs::statfs;
use statfs64::statfs64;
use statx::statx;
use symlink::symlink;
use symlinkat::symlinkat;
use syncfs::syncfs;
use time::time;
use timer_create::timer_create;
use timer_delete::timer_delete;
use timer_settime::timer_settime;
use tkill::tkill;
use truncate::truncate;
use umask::umask;
use umount::umount;
use uname::uname;
use unlink::unlink;
use unlinkat::unlinkat;
use utils::{errno, errno::EResult, lock::IntMutex, ptr::arc::Arc, DisplayableStr};
use utimensat::utimensat;
use vfork::vfork;
use wait4::wait4;
use waitpid::waitpid;
use write::write;
use writev::writev;

/// The ID of the `sigreturn` system call, for use by the signal trampoline.
pub const SIGRETURN_ID: usize = 0x077;

/// A system call handler.
pub trait SyscallHandler<'p, const NAME: &'static str, Args> {
	/// Returns the name of the handler.
	#[inline]
	fn name(&self) -> &'static str {
		NAME
	}

	/// Calls the system call.
	///
	/// Arguments:
	/// - `process` is the process calling the system call.
	/// - `regs` is the register state of the process at the moment of the system call.
	///
	/// The function returns the result of the system call.
	fn call(self, process: &'p Arc<IntMutex<Process>>, regs: &'p Regs) -> EResult<usize>;
}

/// Implementation of [`SyscallHandler`] for functions with arguments.
macro_rules! impl_syscall_handler {
    ($($ty:ident),*) => {
        impl<'p, F, const N: &'static str, $($ty,)*> SyscallHandler<'p, N, ($($ty,)*)> for F
        where F: FnOnce($($ty,)*) -> EResult<usize>,
			$($ty: FromSyscall<'p> + 'p,)*
        {
			#[allow(non_snake_case, unused_variables, unused_mut)]
            fn call(self, process: &'p Arc<IntMutex<Process>>, regs: &'p Regs) -> EResult<usize> {
                let mut args_cursor = 0;
                $(
                    let $ty = $ty::from_syscall(process, regs, &mut args_cursor);
                )*
                self($($ty,)*)
            }
        }
    };
}

impl_syscall_handler!();
impl_syscall_handler!(T1);
impl_syscall_handler!(T1, T2);
impl_syscall_handler!(T1, T2, T3);
impl_syscall_handler!(T1, T2, T3, T4);
impl_syscall_handler!(T1, T2, T3, T4, T5);
impl_syscall_handler!(T1, T2, T3, T4, T5, T6);
impl_syscall_handler!(T1, T2, T3, T4, T5, T6, T7);
impl_syscall_handler!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_syscall_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_syscall_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_syscall_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_syscall_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_syscall_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_syscall_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
impl_syscall_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
impl_syscall_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);

/// Extracts a value from a system call, either from the process that made the call, or from an
/// argument.
///
/// The [`Debug`] trait is used for the `strace` feature.
pub trait FromSyscall<'p>: fmt::Debug {
	/// Constructs the value from the given process or syscall argument value.
	fn from_syscall(
		process: &'p Arc<IntMutex<Process>>,
		regs: &'p Regs,
		args_cursor: &mut u8,
	) -> Self
	where
		Self: 'p;
}

impl<'p> FromSyscall<'p> for &'p Arc<IntMutex<Process>> {
	#[inline]
	fn from_syscall(
		process: &'p Arc<IntMutex<Process>>,
		_regs: &'p Regs,
		_args_cursor: &mut u8,
	) -> Self
	where
		Self: 'p,
	{
		process
	}
}

impl<'p> FromSyscall<'p> for &'p IntMutex<Process> {
	#[inline]
	fn from_syscall(
		process: &'p Arc<IntMutex<Process>>,
		_regs: &'p Regs,
		_args_cursor: &mut u8,
	) -> Self
	where
		Self: 'p,
	{
		process
	}
}

impl<'p> FromSyscall<'p> for &'p Regs {
	#[inline]
	fn from_syscall(
		_process: &'p Arc<IntMutex<Process>>,
		regs: &'p Regs,
		_args_cursor: &mut u8,
	) -> Self
	where
		Self: 'p,
	{
		regs
	}
}

/// Implement [`FromSyscall`] for a primitive type.
macro_rules! impl_from_syscall_primitive {
	($type:ident) => {
		impl<'p> FromSyscall<'p> for $type {
			#[inline]
			fn from_syscall(
				_process: &'p Arc<IntMutex<Process>>,
				regs: &'p Regs,
				args_cursor: &mut u8,
			) -> Self {
				let val = regs.get_syscall_arg(*args_cursor);
				*args_cursor += 1;
				val as _
			}
		}
	};
}

impl_from_syscall_primitive!(i8);
impl_from_syscall_primitive!(u8);
impl_from_syscall_primitive!(i16);
impl_from_syscall_primitive!(u16);
impl_from_syscall_primitive!(i32);
impl_from_syscall_primitive!(u32);
impl_from_syscall_primitive!(i64);
impl_from_syscall_primitive!(u64);
impl_from_syscall_primitive!(isize);
impl_from_syscall_primitive!(usize);

impl<T> FromSyscall<'_> for *const T {
	#[inline]
	fn from_syscall(_process: &Arc<IntMutex<Process>>, regs: &Regs, args_cursor: &mut u8) -> Self {
		let val = regs.get_syscall_arg(*args_cursor);
		*args_cursor += 1;
		val as _
	}
}

impl<T> FromSyscall<'_> for *mut T {
	#[inline]
	fn from_syscall(_process: &Arc<IntMutex<Process>>, regs: &Regs, args_cursor: &mut u8) -> Self {
		let val = regs.get_syscall_arg(*args_cursor);
		*args_cursor += 1;
		val as _
	}
}

/// Wrapper for a pointer.
pub struct SyscallPtr<T: Sized + fmt::Debug>(Option<NonNull<T>>);

impl<T: Sized + fmt::Debug> From<usize> for SyscallPtr<T> {
	fn from(value: usize) -> Self {
		Self(NonNull::new(value as _))
	}
}

impl<T: Sized + fmt::Debug> FromSyscall<'_> for SyscallPtr<T> {
	fn from_syscall(_process: &Arc<IntMutex<Process>>, regs: &Regs, args_cursor: &mut u8) -> Self {
		let val = regs.get_syscall_arg(*args_cursor);
		*args_cursor += 1;
		Self(NonNull::new(val as _))
	}
}

impl<T: Sized + fmt::Debug> SyscallPtr<T> {
	/// Tells whether the pointer is null.
	pub fn is_null(&self) -> bool {
		self.0.is_none()
	}

	/// Returns an immutable pointer to the data.
	pub fn as_ptr(&self) -> *const T {
		self.0.as_ref().map(|p| p.as_ptr() as _).unwrap_or(null())
	}

	/// Returns a mutable pointer to the data.
	pub fn as_ptr_mut(&self) -> *mut T {
		self.0
			.as_ref()
			.map(|p| p.as_ptr() as _)
			.unwrap_or(null_mut())
	}

	/// Returns an immutable reference to the value of the pointer.
	///
	/// If the pointer is null, the function returns `None`.
	///
	/// If the value is not accessible, the function returns an error.
	pub fn get<'a>(&self, mem_space: &'a MemSpace) -> EResult<Option<&'a T>> {
		let Some(ptr) = self.0 else {
			return Ok(None);
		};
		if !mem_space.can_access(ptr.as_ptr() as _, size_of::<T>(), true, false) {
			return Err(errno!(EFAULT));
		}
		// Safe because access is checked before
		Ok(Some(unsafe { ptr.as_ref() }))
	}

	/// Returns a mutable reference to the value of the pointer.
	///
	/// If the pointer is null, the function returns `None`.
	///
	/// If the value is not accessible, the function returns an error.
	///
	/// If the value is located on lazily allocated pages, the function
	/// allocates physical pages in order to allow writing.
	pub fn get_mut<'a>(&self, mem_space: &'a mut MemSpace) -> EResult<Option<&'a mut T>> {
		let Some(mut ptr) = self.0 else {
			return Ok(None);
		};
		if !mem_space.can_access(ptr.as_ptr() as _, size_of::<T>(), true, true) {
			return Err(errno!(EFAULT));
		}
		// Allocate memory to make sure it is writable
		mem_space.alloc(ptr.as_ptr() as _, size_of::<T>())?;
		// Safe because access is checked before
		Ok(Some(unsafe { ptr.as_mut() }))
	}
}

impl<T: fmt::Debug> fmt::Debug for SyscallPtr<T> {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();
		let mem_space_mutex = proc.get_mem_space().unwrap();
		let mem_space = mem_space_mutex.lock();
		let ptr = self.as_ptr();
		match self.get(&mem_space) {
			Ok(Some(val)) => write!(fmt, "{ptr:p} = {val:?}"),
			Ok(None) => write!(fmt, "NULL"),
			Err(e) => write!(fmt, "{ptr:p} = (cannot read: {e})"),
		}
	}
}

/// Wrapper for a slice.
///
/// The size of the slice is required when trying to access it.
pub struct SyscallSlice<T: Sized + fmt::Debug>(Option<NonNull<T>>);

impl<T: Sized + fmt::Debug> From<usize> for SyscallSlice<T> {
	fn from(value: usize) -> Self {
		Self(NonNull::new(value as _))
	}
}

impl<T: Sized + fmt::Debug> FromSyscall<'_> for SyscallSlice<T> {
	fn from_syscall(_process: &Arc<IntMutex<Process>>, regs: &Regs, args_cursor: &mut u8) -> Self {
		let val = regs.get_syscall_arg(*args_cursor);
		*args_cursor += 1;
		Self(NonNull::new(val as _))
	}
}

impl<T: Sized + fmt::Debug> SyscallSlice<T> {
	/// Tells whether the pointer is null.
	pub fn is_null(&self) -> bool {
		self.0.is_none()
	}

	/// Returns an immutable pointer to the data.
	pub fn as_ptr(&self) -> *const T {
		self.0.as_ref().map(|p| p.as_ptr() as _).unwrap_or(null())
	}

	/// Returns a mutable pointer to the data.
	pub fn as_ptr_mut(&self) -> *mut T {
		self.0
			.as_ref()
			.map(|p| p.as_ptr() as _)
			.unwrap_or(null_mut())
	}

	/// Returns an immutable reference to the slice.
	///
	/// `len` is the in number of elements in the slice.
	///
	/// If the slice is not accessible, the function returns an error.
	pub fn get<'a>(&self, mem_space: &'a MemSpace, len: usize) -> EResult<Option<&'a [T]>> {
		let Some(ptr) = self.0 else {
			return Ok(None);
		};
		let size = size_of::<T>() * len;
		if !mem_space.can_access(ptr.as_ptr() as _, size, true, false) {
			return Err(errno!(EFAULT));
		}
		Ok(Some(unsafe {
			// Safe because access is checked before
			slice::from_raw_parts(ptr.as_ptr(), len)
		}))
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
	) -> EResult<Option<&'a mut [T]>> {
		let Some(ptr) = self.0 else {
			return Ok(None);
		};
		let size = size_of::<T>() * len;
		if !mem_space.can_access(ptr.as_ptr() as _, size, true, true) {
			return Err(errno!(EFAULT));
		}
		// Allocate memory to make sure it is writable
		mem_space.alloc(ptr.as_ptr() as _, size)?;
		Ok(Some(unsafe {
			// Safe because access is checked before
			slice::from_raw_parts_mut(ptr.as_ptr(), len)
		}))
	}
}

impl<T: fmt::Debug> fmt::Debug for SyscallSlice<T> {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		// TODO Print value? (how to get the length of the slice?)
		let ptr = self.as_ptr();
		if !ptr.is_null() {
			write!(fmt, "{ptr:p}")
		} else {
			write!(fmt, "NULL")
		}
	}
}

/// Wrapper for a C-style, nul-terminated (`\0`) string.
pub struct SyscallString(Option<NonNull<u8>>);

impl From<usize> for SyscallString {
	fn from(value: usize) -> Self {
		Self(NonNull::new(value as _))
	}
}

impl FromSyscall<'_> for SyscallString {
	fn from_syscall(_process: &Arc<IntMutex<Process>>, regs: &Regs, args_cursor: &mut u8) -> Self {
		let val = regs.get_syscall_arg(*args_cursor);
		*args_cursor += 1;
		Self(NonNull::new(val as _))
	}
}

impl SyscallString {
	/// Tells whether the pointer is null.
	pub fn is_null(&self) -> bool {
		self.0.is_none()
	}

	/// Returns an immutable pointer to the data.
	pub fn as_ptr(&self) -> *const u8 {
		self.0.as_ref().map(|p| p.as_ptr() as _).unwrap_or(null())
	}

	/// Returns an immutable reference to the string.
	///
	/// If the string is not accessible, the function returns an error.
	pub fn get<'a>(&self, mem_space: &'a MemSpace) -> EResult<Option<&'a [u8]>> {
		let Some(ptr) = self.0 else {
			return Ok(None);
		};
		let len = mem_space
			.can_access_string(ptr.as_ptr(), true, false)
			.ok_or_else(|| errno!(EFAULT))?;
		Ok(Some(unsafe {
			// Safe because access is checked before
			slice::from_raw_parts(ptr.as_ptr(), len)
		}))
	}
}

impl fmt::Debug for SyscallString {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();
		let mem_space_mutex = proc.get_mem_space().unwrap();
		let mem_space = mem_space_mutex.lock();
		let ptr = self.as_ptr();
		match self.get(&mem_space) {
			Ok(Some(s)) => {
				// TODO Add backslashes to escape `"` and `\`
				let s = DisplayableStr(s);
				write!(fmt, "{ptr:p} = \"{s}\"")
			}
			Ok(None) => write!(fmt, "NULL"),
			Err(e) => write!(fmt, "{ptr:p} = (cannot read: {e})"),
		}
	}
}

/// Wrapper for a C-style, NULL-terminated string array.
pub struct SyscallArray(Option<NonNull<*const u8>>);

impl From<usize> for SyscallArray {
	fn from(value: usize) -> Self {
		Self(NonNull::new(value as _))
	}
}

impl FromSyscall<'_> for SyscallArray {
	fn from_syscall(_process: &Arc<IntMutex<Process>>, regs: &Regs, args_cursor: &mut u8) -> Self {
		let val = regs.get_syscall_arg(*args_cursor);
		*args_cursor += 1;
		Self(NonNull::new(val as _))
	}
}

impl SyscallArray {
	/// Tells whether the pointer is null.
	pub fn is_null(&self) -> bool {
		self.0.is_none()
	}

	/// Returns an immutable pointer to the data.
	pub fn as_ptr(&self) -> *const u8 {
		self.0.as_ref().map(|p| p.as_ptr() as _).unwrap_or(null())
	}

	/// Returns an iterator over the array's elements.
	pub fn iter<'a>(
		&'a self,
		mem_space: &'a MemSpace,
	) -> impl Iterator<Item = EResult<&'a [u8]>> + 'a {
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
				Ok(s) => list_ref.entry(&DisplayableStr(s)),
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
	type Item = EResult<&'a [u8]>;

	fn next(&mut self) -> Option<Self::Item> {
		let Some(arr) = self.arr.0 else {
			return Some(Err(errno!(EFAULT)));
		};
		// If reaching the end of the array, stop
		let str_ptr = unsafe { arr.add(self.i).read_volatile() };
		if str_ptr.is_null() {
			return None;
		}
		// Get string
		let string: SyscallString = (str_ptr as usize).into();
		let string = string
			.get(self.mem_space)
			.and_then(|s| s.ok_or_else(|| errno!(EFAULT)));
		self.i += 1;
		Some(string)
	}
}

/// Syscall declaration.
macro_rules! syscall {
	($name:ident, $process:expr, $regs:expr) => {{
		const NAME: &str = stringify!($name);
		SyscallHandler::<NAME, _>::call($name, $process, $regs)
	}};
}

/// Executes the system call associated with the given `id` and returns its result.
///
/// If the syscall doesn't exist, the function returns `None`.
#[inline]
fn do_syscall(process: &Arc<IntMutex<Process>>, regs: &Regs, id: usize) -> Option<EResult<usize>> {
	match id {
		0x001 => Some(syscall!(_exit, process, regs)),
		0x002 => Some(syscall!(fork, process, regs)),
		0x003 => Some(syscall!(read, process, regs)),
		0x004 => Some(syscall!(write, process, regs)),
		0x005 => Some(syscall!(open, process, regs)),
		0x006 => Some(syscall!(close, process, regs)),
		0x007 => Some(syscall!(waitpid, process, regs)),
		0x008 => Some(syscall!(creat, process, regs)),
		0x009 => Some(syscall!(link, process, regs)),
		0x00a => Some(syscall!(unlink, process, regs)),
		0x00b => Some(syscall!(execve, process, regs)),
		0x00c => Some(syscall!(chdir, process, regs)),
		0x00d => Some(syscall!(time, process, regs)),
		0x00e => Some(syscall!(mknod, process, regs)),
		0x00f => Some(syscall!(chmod, process, regs)),
		0x010 => Some(syscall!(lchown, process, regs)),
		0x011 => Some(syscall!(r#break, process, regs)),
		// TODO 0x012 => Some(syscall!(oldstat, process, regs)),
		// TODO 0x013 => Some(syscall!(lseek, process, regs)),
		0x014 => Some(syscall!(getpid, process, regs)),
		0x015 => Some(syscall!(mount, process, regs)),
		0x016 => Some(syscall!(umount, process, regs)),
		0x017 => Some(syscall!(setuid, process, regs)),
		0x018 => Some(syscall!(getuid, process, regs)),
		// TODO 0x019 => Some(syscall!(stime, process, regs)),
		// TODO 0x01a => Some(syscall!(ptrace, process, regs)),
		// TODO 0x01b => Some(syscall!(alarm, process, regs)),
		// TODO 0x01c => Some(syscall!(oldfstat, process, regs)),
		// TODO 0x01d => Some(syscall!(pause, process, regs)),
		// TODO 0x01e => Some(syscall!(utime, process, regs)),
		// TODO 0x01f => Some(syscall!(stty, process, regs)),
		// TODO 0x020 => Some(syscall!(gtty, process, regs)),
		0x021 => Some(syscall!(access, process, regs)),
		// TODO 0x022 => Some(syscall!(nice, process, regs)),
		// TODO 0x023 => Some(syscall!(ftime, process, regs)),
		// TODO 0x024 => Some(syscall!(sync, process, regs)),
		0x025 => Some(syscall!(kill, process, regs)),
		0x026 => Some(syscall!(rename, process, regs)),
		0x027 => Some(syscall!(mkdir, process, regs)),
		0x028 => Some(syscall!(rmdir, process, regs)),
		0x029 => Some(syscall!(dup, process, regs)),
		0x02a => Some(syscall!(pipe, process, regs)),
		// TODO 0x02b => Some(syscall!(times, process, regs)),
		// TODO 0x02c => Some(syscall!(prof, process, regs)),
		0x02d => Some(syscall!(brk, process, regs)),
		0x02e => Some(syscall!(setgid, process, regs)),
		0x02f => Some(syscall!(getgid, process, regs)),
		0x030 => Some(syscall!(signal, process, regs)),
		0x031 => Some(syscall!(geteuid, process, regs)),
		0x032 => Some(syscall!(getegid, process, regs)),
		// TODO 0x033 => Some(syscall!(acct, process, regs)),
		// TODO 0x034 => Some(syscall!(umount2, process, regs)),
		// TODO 0x035 => Some(syscall!(lock, process, regs)),
		0x036 => Some(syscall!(ioctl, process, regs)),
		0x037 => Some(syscall!(fcntl, process, regs)),
		// TODO 0x038 => Some(syscall!(mpx, process, regs)),
		0x039 => Some(syscall!(setpgid, process, regs)),
		// TODO 0x03a => Some(syscall!(ulimit, process, regs)),
		// TODO 0x03b => Some(syscall!(oldolduname, process, regs)),
		0x03c => Some(syscall!(umask, process, regs)),
		0x03d => Some(syscall!(chroot, process, regs)),
		// TODO 0x03e => Some(syscall!(ustat, process, regs)),
		0x03f => Some(syscall!(dup2, process, regs)),
		0x040 => Some(syscall!(getppid, process, regs)),
		// TODO 0x041 => Some(syscall!(getpgrp, process, regs)),
		// TODO 0x042 => Some(syscall!(setsid, process, regs)),
		// TODO 0x043 => Some(syscall!(sigaction, process, regs)),
		// TODO 0x044 => Some(syscall!(sgetmask, process, regs)),
		// TODO 0x045 => Some(syscall!(ssetmask, process, regs)),
		// TODO 0x046 => Some(syscall!(setreuid, process, regs)),
		// TODO 0x047 => Some(syscall!(setregid, process, regs)),
		// TODO 0x048 => Some(syscall!(sigsuspend, process, regs)),
		// TODO 0x049 => Some(syscall!(sigpending, process, regs)),
		0x04a => Some(syscall!(sethostname, process, regs)),
		// TODO 0x04b => Some(syscall!(setrlimit, process, regs)),
		// TODO 0x04c => Some(syscall!(getrlimit, process, regs)),
		0x04d => Some(syscall!(getrusage, process, regs)),
		// TODO 0x04e => Some(syscall!(gettimeofday, process, regs)),
		// TODO 0x04f => Some(syscall!(settimeofday, process, regs)),
		// TODO 0x050 => Some(syscall!(getgroups, process, regs)),
		// TODO 0x051 => Some(syscall!(setgroups, process, regs)),
		0x052 => Some(syscall!(select, process, regs)),
		0x053 => Some(syscall!(symlink, process, regs)),
		// TODO 0x054 => Some(syscall!(oldlstat, process, regs)),
		0x055 => Some(syscall!(readlink, process, regs)),
		// TODO 0x056 => Some(syscall!(uselib, process, regs)),
		// TODO 0x057 => Some(syscall!(swapon, process, regs)),
		0x058 => Some(syscall!(reboot, process, regs)),
		// TODO 0x059 => Some(syscall!(readdir, process, regs)),
		0x05a => Some(syscall!(mmap, process, regs)),
		0x05b => Some(syscall!(munmap, process, regs)),
		0x05c => Some(syscall!(truncate, process, regs)),
		// TODO 0x05d => Some(syscall!(ftruncate, process, regs)),
		0x05e => Some(syscall!(fchmod, process, regs)),
		// TODO 0x05f => Some(syscall!(fchown, process, regs)),
		// TODO 0x060 => Some(syscall!(getpriority, process, regs)),
		// TODO 0x061 => Some(syscall!(setpriority, process, regs)),
		// TODO 0x062 => Some(syscall!(profil, process, regs)),
		0x063 => Some(syscall!(statfs, process, regs)),
		0x064 => Some(syscall!(fstatfs, process, regs)),
		// TODO 0x065 => Some(syscall!(ioperm, process, regs)),
		// TODO 0x066 => Some(syscall!(socketcall, process, regs)),
		// TODO 0x067 => Some(syscall!(syslog, process, regs)),
		// TODO 0x068 => Some(syscall!(setitimer, process, regs)),
		// TODO 0x069 => Some(syscall!(getitimer, process, regs)),
		// TODO 0x06a => Some(syscall!(stat, process, regs)),
		// TODO 0x06b => Some(syscall!(lstat, process, regs)),
		// TODO 0x06c => Some(syscall!(fstat, process, regs)),
		// TODO 0x06d => Some(syscall!(olduname, process, regs)),
		// TODO 0x06e => Some(syscall!(iopl, process, regs)),
		// TODO 0x06f => Some(syscall!(vhangup, process, regs)),
		// TODO 0x070 => Some(syscall!(idle, process, regs)),
		// TODO 0x071 => Some(syscall!(vm86old, process, regs)),
		0x072 => Some(syscall!(wait4, process, regs)),
		// TODO 0x073 => Some(syscall!(swapoff, process, regs)),
		// TODO 0x074 => Some(syscall!(sysinfo, process, regs)),
		// TODO 0x075 => Some(syscall!(ipc, process, regs)),
		0x076 => Some(syscall!(fsync, process, regs)),
		SIGRETURN_ID => Some(syscall!(sigreturn, process, regs)),
		0x078 => Some(syscall!(clone, process, regs)),
		// TODO 0x079 => Some(syscall!(setdomainname, process, regs)),
		0x07a => Some(syscall!(uname, process, regs)),
		// TODO 0x07c => Some(syscall!(adjtimex, process, regs)),
		0x07d => Some(syscall!(mprotect, process, regs)),
		// TODO 0x07e => Some(syscall!(sigprocmask, process, regs)),
		// TODO 0x07f => Some(syscall!(create_module, process, regs)),
		0x080 => Some(syscall!(init_module, process, regs)),
		0x081 => Some(syscall!(delete_module, process, regs)),
		// TODO 0x083 => Some(syscall!(quotactl, process, regs)),
		0x084 => Some(syscall!(getpgid, process, regs)),
		0x085 => Some(syscall!(fchdir, process, regs)),
		// TODO 0x086 => Some(syscall!(bdflush, process, regs)),
		// TODO 0x087 => Some(syscall!(sysfs, process, regs)),
		// TODO 0x088 => Some(syscall!(personality, process, regs)),
		// TODO 0x089 => Some(syscall!(afs_syscall, process, regs)),
		// TODO 0x08a => Some(syscall!(setfsuid, process, regs)),
		// TODO 0x08b => Some(syscall!(setfsgid, process, regs)),
		0x08c => Some(syscall!(_llseek, process, regs)),
		0x08d => Some(syscall!(getdents, process, regs)),
		0x08e => Some(syscall!(_newselect, process, regs)),
		// TODO 0x08f => Some(syscall!(flock, process, regs)),
		0x090 => Some(syscall!(msync, process, regs)),
		0x091 => Some(syscall!(readv, process, regs)),
		0x092 => Some(syscall!(writev, process, regs)),
		// TODO 0x093 => Some(syscall!(getsid, process, regs)),
		// TODO 0x094 => Some(syscall!(fdatasync, process, regs)),
		// TODO 0x095 => Some(syscall!(_sysctl, process, regs)),
		// TODO 0x096 => Some(syscall!(mlock, process, regs)),
		// TODO 0x097 => Some(syscall!(munlock, process, regs)),
		// TODO 0x098 => Some(syscall!(mlockall, process, regs)),
		// TODO 0x099 => Some(syscall!(munlockall, process, regs)),
		// TODO 0x09a => Some(syscall!(sched_setparam, process, regs)),
		// TODO 0x09b => Some(syscall!(sched_getparam, process, regs)),
		// TODO 0x09c => Some(syscall!(sched_setscheduler, process, regs)),
		// TODO 0x09d => Some(syscall!(sched_getscheduler, process, regs)),
		0x09e => Some(syscall!(sched_yield, process, regs)),
		// TODO 0x09f => Some(syscall!(sched_get_priority_max, process, regs)),
		// TODO 0x0a0 => Some(syscall!(sched_get_priority_min, process, regs)),
		// TODO 0x0a1 => Some(syscall!(sched_rr_get_interval, process, regs)),
		0x0a2 => Some(syscall!(nanosleep, process, regs)),
		// TODO 0x0a3 => Some(syscall!(mremap, process, regs)),
		// TODO 0x0a4 => Some(syscall!(setresuid, process, regs)),
		// TODO 0x0a5 => Some(syscall!(getresuid, process, regs)),
		// TODO 0x0a6 => Some(syscall!(vm86, process, regs)),
		// TODO 0x0a7 => Some(syscall!(query_module, process, regs)),
		0x0a8 => Some(syscall!(poll, process, regs)),
		// TODO 0x0a9 => Some(syscall!(nfsservctl, process, regs)),
		// TODO 0x0aa => Some(syscall!(setresgid, process, regs)),
		// TODO 0x0ab => Some(syscall!(getresgid, process, regs)),
		// TODO 0x0ac => Some(syscall!(prctl, process, regs)),
		// TODO 0x0ad => Some(syscall!(rt_sigreturn, process, regs)),
		0x0ae => Some(syscall!(rt_sigaction, process, regs)),
		0x0af => Some(syscall!(rt_sigprocmask, process, regs)),
		// TODO 0x0b0 => Some(syscall!(rt_sigpending, process, regs)),
		// TODO 0x0b1 => Some(syscall!(rt_sigtimedwait, process, regs)),
		// TODO 0x0b2 => Some(syscall!(rt_sigqueueinfo, process, regs)),
		// TODO 0x0b3 => Some(syscall!(rt_sigsuspend, process, regs)),
		// TODO 0x0b4 => Some(syscall!(pread64, process, regs)),
		// TODO 0x0b5 => Some(syscall!(pwrite64, process, regs)),
		0x0b6 => Some(syscall!(chown, process, regs)),
		0x0b7 => Some(syscall!(getcwd, process, regs)),
		// TODO 0x0b8 => Some(syscall!(capget, process, regs)),
		// TODO 0x0b9 => Some(syscall!(capset, process, regs)),
		// TODO 0x0ba => Some(syscall!(sigaltstack, process, regs)),
		// TODO 0x0bb => Some(syscall!(sendfile, process, regs)),
		// TODO 0x0bc => Some(syscall!(getpmsg, process, regs)),
		// TODO 0x0bd => Some(syscall!(putpmsg, process, regs)),
		0x0be => Some(syscall!(vfork, process, regs)),
		// TODO 0x0bf => Some(syscall!(ugetrlimit, process, regs)),
		0x0c0 => Some(syscall!(mmap2, process, regs)),
		// TODO 0x0c1 => Some(syscall!(truncate64, process, regs)),
		// TODO 0x0c2 => Some(syscall!(ftruncate64, process, regs)),
		// TODO 0x0c3 => Some(syscall!(stat64, process, regs)),
		// TODO 0x0c4 => Some(syscall!(lstat64, process, regs)),
		0x0c5 => Some(syscall!(fstat64, process, regs)),
		// TODO 0x0c6 => Some(syscall!(lchown32, process, regs)),
		0x0c7 => Some(syscall!(getuid32, process, regs)),
		0x0c8 => Some(syscall!(getgid32, process, regs)),
		0x0c9 => Some(syscall!(geteuid32, process, regs)),
		0x0ca => Some(syscall!(getegid32, process, regs)),
		// TODO 0x0cb => Some(syscall!(setreuid32, process, regs)),
		// TODO 0x0cc => Some(syscall!(setregid32, process, regs)),
		// TODO 0x0cd => Some(syscall!(getgroups32, process, regs)),
		// TODO 0x0ce => Some(syscall!(setgroups32, process, regs)),
		// TODO 0x0cf => Some(syscall!(fchown32, process, regs)),
		// TODO 0x0d0 => Some(syscall!(setresuid32, process, regs)),
		// TODO 0x0d1 => Some(syscall!(getresuid32, process, regs)),
		// TODO 0x0d2 => Some(syscall!(setresgid32, process, regs)),
		// TODO 0x0d3 => Some(syscall!(getresgid32, process, regs)),
		0x0d4 => Some(syscall!(chown32, process, regs)),
		0x0d5 => Some(syscall!(setuid32, process, regs)),
		0x0d6 => Some(syscall!(setgid32, process, regs)),
		// TODO 0x0d7 => Some(syscall!(setfsuid32, process, regs)),
		// TODO 0x0d8 => Some(syscall!(setfsgid32, process, regs)),
		// TODO 0x0d9 => Some(syscall!(pivot_root, process, regs)),
		// TODO 0x0da => Some(syscall!(mincore, process, regs)),
		0x0db => Some(syscall!(madvise, process, regs)),
		0x0dc => Some(syscall!(getdents64, process, regs)),
		0x0dd => Some(syscall!(fcntl64, process, regs)),
		0x0e0 => Some(syscall!(gettid, process, regs)),
		// TODO 0x0e1 => Some(syscall!(readahead, process, regs)),
		// TODO 0x0e2 => Some(syscall!(setxattr, process, regs)),
		// TODO 0x0e3 => Some(syscall!(lsetxattr, process, regs)),
		// TODO 0x0e4 => Some(syscall!(fsetxattr, process, regs)),
		// TODO 0x0e5 => Some(syscall!(getxattr, process, regs)),
		// TODO 0x0e6 => Some(syscall!(lgetxattr, process, regs)),
		// TODO 0x0e7 => Some(syscall!(fgetxattr, process, regs)),
		// TODO 0x0e8 => Some(syscall!(listxattr, process, regs)),
		// TODO 0x0e9 => Some(syscall!(llistxattr, process, regs)),
		// TODO 0x0ea => Some(syscall!(flistxattr, process, regs)),
		// TODO 0x0eb => Some(syscall!(removexattr, process, regs)),
		// TODO 0x0ec => Some(syscall!(lremovexattr, process, regs)),
		// TODO 0x0ed => Some(syscall!(fremovexattr, process, regs)),
		0x0ee => Some(syscall!(tkill, process, regs)),
		// TODO 0x0ef => Some(syscall!(sendfile64, process, regs)),
		// TODO 0x0f0 => Some(syscall!(futex, process, regs)),
		// TODO 0x0f1 => Some(syscall!(sched_setaffinity, process, regs)),
		// TODO 0x0f2 => Some(syscall!(sched_getaffinity, process, regs)),
		0x0f3 => Some(syscall!(set_thread_area, process, regs)),
		// TODO 0x0f4 => Some(syscall!(get_thread_area, process, regs)),
		// TODO 0x0f5 => Some(syscall!(io_setup, process, regs)),
		// TODO 0x0f6 => Some(syscall!(io_destroy, process, regs)),
		// TODO 0x0f7 => Some(syscall!(io_getevents, process, regs)),
		// TODO 0x0f8 => Some(syscall!(io_submit, process, regs)),
		// TODO 0x0f9 => Some(syscall!(io_cancel, process, regs)),
		// TODO 0x0fa => Some(syscall!(fadvise64, process, regs)),
		0x0fc => Some(syscall!(exit_group, process, regs)),
		// TODO 0x0fd => Some(syscall!(lookup_dcookie, process, regs)),
		// TODO 0x0fe => Some(syscall!(epoll_create, process, regs)),
		// TODO 0x0ff => Some(syscall!(epoll_ctl, process, regs)),
		// TODO 0x100 => Some(syscall!(epoll_wait, process, regs)),
		// TODO 0x101 => Some(syscall!(remap_file_pages, process, regs)),
		0x102 => Some(syscall!(set_tid_address, process, regs)),
		0x103 => Some(syscall!(timer_create, process, regs)),
		0x104 => Some(syscall!(timer_settime, process, regs)),
		// TODO 0x105 => Some(syscall!(timer_gettime, process, regs)),
		// TODO 0x106 => Some(syscall!(timer_getoverrun, process, regs)),
		0x107 => Some(syscall!(timer_delete, process, regs)),
		// TODO 0x108 => Some(syscall!(clock_settime, process, regs)),
		0x109 => Some(syscall!(clock_gettime, process, regs)),
		// TODO 0x10a => Some(syscall!(clock_getres, process, regs)),
		// TODO 0x10b => Some(syscall!(clock_nanosleep, process, regs)),
		0x10c => Some(syscall!(statfs64, process, regs)),
		0x10d => Some(syscall!(fstatfs64, process, regs)),
		// TODO 0x10e => Some(syscall!(tgkill, process, regs)),
		// TODO 0x10f => Some(syscall!(utimes, process, regs)),
		0x110 => Some(syscall!(fadvise64_64, process, regs)),
		// TODO 0x111 => Some(syscall!(vserver, process, regs)),
		// TODO 0x112 => Some(syscall!(mbind, process, regs)),
		// TODO 0x113 => Some(syscall!(get_mempolicy, process, regs)),
		// TODO 0x114 => Some(syscall!(set_mempolicy, process, regs)),
		// TODO 0x115 => Some(syscall!(mq_open, process, regs)),
		// TODO 0x116 => Some(syscall!(mq_unlink, process, regs)),
		// TODO 0x117 => Some(syscall!(mq_timedsend, process, regs)),
		// TODO 0x118 => Some(syscall!(mq_timedreceive, process, regs)),
		// TODO 0x119 => Some(syscall!(mq_notify, process, regs)),
		// TODO 0x11a => Some(syscall!(mq_getsetattr, process, regs)),
		// TODO 0x11b => Some(syscall!(kexec_load, process, regs)),
		// TODO 0x11c => Some(syscall!(waitid, process, regs)),
		// TODO 0x11e => Some(syscall!(add_key, process, regs)),
		// TODO 0x11f => Some(syscall!(request_key, process, regs)),
		// TODO 0x120 => Some(syscall!(keyctl, process, regs)),
		// TODO 0x121 => Some(syscall!(ioprio_set, process, regs)),
		// TODO 0x122 => Some(syscall!(ioprio_get, process, regs)),
		// TODO 0x123 => Some(syscall!(inotify_init, process, regs)),
		// TODO 0x124 => Some(syscall!(inotify_add_watch, process, regs)),
		// TODO 0x125 => Some(syscall!(inotify_rm_watch, process, regs)),
		// TODO 0x126 => Some(syscall!(migrate_pages, process, regs)),
		0x127 => Some(syscall!(openat, process, regs)),
		// TODO 0x128 => Some(syscall!(mkdirat, process, regs)),
		// TODO 0x129 => Some(syscall!(mknodat, process, regs)),
		// TODO 0x12a => Some(syscall!(fchownat, process, regs)),
		// TODO 0x12b => Some(syscall!(futimesat, process, regs)),
		// TODO 0x12c => Some(syscall!(fstatat64, process, regs)),
		0x12d => Some(syscall!(unlinkat, process, regs)),
		// TODO 0x12e => Some(syscall!(renameat, process, regs)),
		0x12f => Some(syscall!(linkat, process, regs)),
		0x130 => Some(syscall!(symlinkat, process, regs)),
		// TODO 0x131 => Some(syscall!(readlinkat, process, regs)),
		0x132 => Some(syscall!(fchmodat, process, regs)),
		0x133 => Some(syscall!(faccessat, process, regs)),
		0x134 => Some(syscall!(pselect6, process, regs)),
		// TODO 0x135 => Some(syscall!(ppoll, process, regs)),
		// TODO 0x136 => Some(syscall!(unshare, process, regs)),
		// TODO 0x137 => Some(syscall!(set_robust_list, process, regs)),
		// TODO 0x138 => Some(syscall!(get_robust_list, process, regs)),
		0x139 => Some(syscall!(splice, process, regs)),
		// TODO 0x13a => Some(syscall!(sync_file_range, process, regs)),
		// TODO 0x13b => Some(syscall!(tee, process, regs)),
		// TODO 0x13c => Some(syscall!(vmsplice, process, regs)),
		// TODO 0x13d => Some(syscall!(move_pages, process, regs)),
		// TODO 0x13e => Some(syscall!(getcpu, process, regs)),
		// TODO 0x13f => Some(syscall!(epoll_pwait, process, regs)),
		0x140 => Some(syscall!(utimensat, process, regs)),
		// TODO 0x141 => Some(syscall!(signalfd, process, regs)),
		// TODO 0x142 => Some(syscall!(timerfd_create, process, regs)),
		// TODO 0x143 => Some(syscall!(eventfd, process, regs)),
		// TODO 0x144 => Some(syscall!(fallocate, process, regs)),
		// TODO 0x145 => Some(syscall!(timerfd_settime, process, regs)),
		// TODO 0x146 => Some(syscall!(timerfd_gettime, process, regs)),
		// TODO 0x147 => Some(syscall!(signalfd4, process, regs)),
		// TODO 0x148 => Some(syscall!(eventfd2, process, regs)),
		// TODO 0x149 => Some(syscall!(epoll_create1, process, regs)),
		// TODO 0x14a => Some(syscall!(dup3, process, regs)),
		0x14b => Some(syscall!(pipe2, process, regs)),
		// TODO 0x14c => Some(syscall!(inotify_init1, process, regs)),
		0x14d => Some(syscall!(preadv, process, regs)),
		0x14e => Some(syscall!(pwritev, process, regs)),
		// TODO 0x14f => Some(syscall!(rt_tgsigqueueinfo, process, regs)),
		// TODO 0x150 => Some(syscall!(perf_event_open, process, regs)),
		// TODO 0x151 => Some(syscall!(recvmmsg, process, regs)),
		// TODO 0x152 => Some(syscall!(fanotify_init, process, regs)),
		// TODO 0x153 => Some(syscall!(fanotify_mark, process, regs)),
		0x154 => Some(syscall!(prlimit64, process, regs)),
		// TODO 0x155 => Some(syscall!(name_to_handle_at, process, regs)),
		// TODO 0x156 => Some(syscall!(open_by_handle_at, process, regs)),
		// TODO 0x157 => Some(syscall!(clock_adjtime, process, regs)),
		0x158 => Some(syscall!(syncfs, process, regs)),
		// TODO 0x159 => Some(syscall!(sendmmsg, process, regs)),
		// TODO 0x15a => Some(syscall!(setns, process, regs)),
		// TODO 0x15b => Some(syscall!(process_vm_readv, process, regs)),
		// TODO 0x15c => Some(syscall!(process_vm_writev, process, regs)),
		// TODO 0x15d => Some(syscall!(kcmp, process, regs)),
		0x15e => Some(syscall!(finit_module, process, regs)),
		// TODO 0x15f => Some(syscall!(sched_setattr, process, regs)),
		// TODO 0x160 => Some(syscall!(sched_getattr, process, regs)),
		0x161 => Some(syscall!(renameat2, process, regs)),
		// TODO 0x162 => Some(syscall!(seccomp, process, regs)),
		0x163 => Some(syscall!(getrandom, process, regs)),
		// TODO 0x164 => Some(syscall!(memfd_create, process, regs)),
		// TODO 0x165 => Some(syscall!(bpf, process, regs)),
		// TODO 0x166 => Some(syscall!(execveat, process, regs)),
		0x167 => Some(syscall!(socket, process, regs)),
		0x168 => Some(syscall!(socketpair, process, regs)),
		0x169 => Some(syscall!(bind, process, regs)),
		0x16a => Some(syscall!(connect, process, regs)),
		// TODO 0x16b => Some(syscall!(listen, process, regs)),
		// TODO 0x16c => Some(syscall!(accept4, process, regs)),
		0x16d => Some(syscall!(getsockopt, process, regs)),
		0x16e => Some(syscall!(setsockopt, process, regs)),
		0x16f => Some(syscall!(getsockname, process, regs)),
		// TODO 0x170 => Some(syscall!(getpeername, process, regs)),
		0x171 => Some(syscall!(sendto, process, regs)),
		// TODO 0x172 => Some(syscall!(sendmsg, process, regs)),
		// TODO 0x173 => Some(syscall!(recvfrom, process, regs)),
		// TODO 0x174 => Some(syscall!(recvmsg, process, regs)),
		0x175 => Some(syscall!(shutdown, process, regs)),
		// TODO 0x176 => Some(syscall!(userfaultfd, process, regs)),
		// TODO 0x177 => Some(syscall!(membarrier, process, regs)),
		// TODO 0x178 => Some(syscall!(mlock2, process, regs)),
		// TODO 0x179 => Some(syscall!(copy_file_range, process, regs)),
		0x17a => Some(syscall!(preadv2, process, regs)),
		0x17b => Some(syscall!(pwritev2, process, regs)),
		// TODO 0x17c => Some(syscall!(pkey_mprotect, process, regs)),
		// TODO 0x17d => Some(syscall!(pkey_alloc, process, regs)),
		// TODO 0x17e => Some(syscall!(pkey_free, process, regs)),
		0x17f => Some(syscall!(statx, process, regs)),
		0x180 => Some(syscall!(arch_prctl, process, regs)),
		// TODO 0x181 => Some(syscall!(io_pgetevents, process, regs)),
		// TODO 0x182 => Some(syscall!(rseq, process, regs)),
		// TODO 0x189 => Some(syscall!(semget, process, regs)),
		// TODO 0x18a => Some(syscall!(semctl, process, regs)),
		// TODO 0x18b => Some(syscall!(shmget, process, regs)),
		// TODO 0x18c => Some(syscall!(shmctl, process, regs)),
		// TODO 0x18d => Some(syscall!(shmat, process, regs)),
		// TODO 0x18e => Some(syscall!(shmdt, process, regs)),
		// TODO 0x18f => Some(syscall!(msgget, process, regs)),
		// TODO 0x190 => Some(syscall!(msgsnd, process, regs)),
		// TODO 0x191 => Some(syscall!(msgrcv, process, regs)),
		// TODO 0x192 => Some(syscall!(msgctl, process, regs)),
		0x193 => Some(syscall!(clock_gettime64, process, regs)),
		// TODO 0x194 => Some(syscall!(clock_settime64, process, regs)),
		// TODO 0x195 => Some(syscall!(clock_adjtime64, process, regs)),
		// TODO 0x196 => Some(syscall!(clock_getres_time64, process, regs)),
		// TODO 0x197 => Some(syscall!(clock_nanosleep_time64, process, regs)),
		// TODO 0x198 => Some(syscall!(timer_gettime64, process, regs)),
		// TODO 0x199 => Some(syscall!(timer_settime64, process, regs)),
		// TODO 0x19a => Some(syscall!(timerfd_gettime64, process, regs)),
		// TODO 0x19b => Some(syscall!(timerfd_settime64, process, regs)),
		// TODO 0x19c => Some(syscall!(utimensat_time64, process, regs)),
		// TODO 0x19d => Some(syscall!(pselect6_time64, process, regs)),
		// TODO 0x19e => Some(syscall!(ppoll_time64, process, regs)),
		// TODO 0x1a0 => Some(syscall!(io_pgetevents_time64, process, regs)),
		// TODO 0x1a1 => Some(syscall!(recvmmsg_time64, process, regs)),
		// TODO 0x1a2 => Some(syscall!(mq_timedsend_time64, process, regs)),
		// TODO 0x1a3 => Some(syscall!(mq_timedreceive_time64, process, regs)),
		// TODO 0x1a4 => Some(syscall!(semtimedop_time64, process, regs)),
		// TODO 0x1a5 => Some(syscall!(rt_sigtimedwait_time64, process, regs)),
		// TODO 0x1a6 => Some(syscall!(futex_time64, process, regs)),
		// TODO 0x1a7 => Some(syscall!(sched_rr_get_interval_time64, process, regs)),
		// TODO 0x1a8 => Some(syscall!(pidfd_send_signal, process, regs)),
		// TODO 0x1a9 => Some(syscall!(io_uring_setup, process, regs)),
		// TODO 0x1aa => Some(syscall!(io_uring_enter, process, regs)),
		// TODO 0x1ab => Some(syscall!(io_uring_register, process, regs)),
		// TODO 0x1ac => Some(syscall!(open_tree, process, regs)),
		// TODO 0x1ad => Some(syscall!(move_mount, process, regs)),
		// TODO 0x1ae => Some(syscall!(fsopen, process, regs)),
		// TODO 0x1af => Some(syscall!(fsconfig, process, regs)),
		// TODO 0x1b0 => Some(syscall!(fsmount, process, regs)),
		// TODO 0x1b1 => Some(syscall!(fspick, process, regs)),
		// TODO 0x1b2 => Some(syscall!(pidfd_open, process, regs)),
		// TODO 0x1b3 => Some(syscall!(clone3, process, regs)),
		// TODO 0x1b4 => Some(syscall!(close_range, process, regs)),
		// TODO 0x1b5 => Some(syscall!(openat2, process, regs)),
		// TODO 0x1b6 => Some(syscall!(pidfd_getfd, process, regs)),
		0x1b7 => Some(syscall!(faccessat2, process, regs)),
		// TODO 0x1b8 => Some(syscall!(process_madvise, process, regs)),
		// TODO 0x1b9 => Some(syscall!(epoll_pwait2, process, regs)),
		// TODO 0x1ba => Some(syscall!(mount_setattr, process, regs)),
		// TODO 0x1bb => Some(syscall!(quotactl_fd, process, regs)),
		// TODO 0x1bc => Some(syscall!(landlock_create_ruleset, process, regs)),
		// TODO 0x1bd => Some(syscall!(landlock_add_rule, process, regs)),
		// TODO 0x1be => Some(syscall!(landlock_restrict_self, process, regs)),
		// TODO 0x1bf => Some(syscall!(memfd_secret, process, regs)),
		// TODO 0x1c0 => Some(syscall!(process_mrelease, process, regs)),
		// TODO 0x1c1 => Some(syscall!(futex_waitv, process, regs)),
		// TODO 0x1c2 => Some(syscall!(set_mempolicy_home_node, process, regs)),
		_ => None,
	}
}

/// Called whenever a system call is triggered.
#[no_mangle]
pub extern "C" fn syscall_handler(regs: &mut Regs) {
	let id = regs.get_syscall_id();
	let proc_mutex = Process::current_assert();
	let Some(res) = do_syscall(&proc_mutex, regs, id) else {
		// The system call doesn't exist. Kill the process with SIGSYS
		{
			let mut proc = proc_mutex.lock();
			if cfg!(feature = "strace") {
				crate::println!(
					"[strace PID: {pid}] invalid syscall (ID: 0x{id:x})",
					pid = proc.get_pid()
				);
			}
			// SIGSYS cannot be caught, thus the process will be terminated
			proc.kill_now(&Signal::SIGSYS);
		}
		drop(proc_mutex);
		crate::enter_loop();
	};
	regs.set_syscall_return(res);
}

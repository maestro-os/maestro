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
mod chroot;
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
mod fstatfs;
mod fstatfs64;
mod getcwd;
mod getdents;
mod getpgid;
mod getpid;
mod getppid;
mod getrandom;
mod getresgid;
mod getresuid;
mod getrusage;
mod getsockname;
mod getsockopt;
mod gettid;
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
mod mount;
mod mprotect;
mod munmap;
mod open;
mod openat;
mod pipe;
mod pipe2;
pub mod poll;
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
mod sethostname;
mod setpgid;
mod setsockopt;
mod shutdown;
mod signal;
mod sigreturn;
mod socket;
mod socketpair;
mod stat;
mod statfs;
mod statfs64;
mod symlink;
mod symlinkat;
mod sync;
mod time;
mod tkill;
mod truncate;
mod umask;
mod umount;
mod uname;
mod unlink;
mod unlinkat;
mod user;
mod util;
mod utimensat;
mod vfork;
mod wait4;
mod waitpid;
mod write;
mod writev;

use crate::{
	arch::x86::{gdt, idt::IntFrame},
	file,
	file::{fd::FileDescriptorTable, perm::AccessProfile, vfs::ResolutionSettings},
	process,
	process::{Process, mem_space::MemSpace, signal::Signal},
	sync::mutex::{IntMutex, Mutex},
	syscall::{
		getdents::getdents64,
		mmap::mmap2,
		sync::{fdatasync, fsync, msync, sync, syncfs},
		time::{
			clock_gettime, clock_gettime64, nanosleep32, nanosleep64, time64, timer_create,
			timer_delete, timer_settime,
		},
		user::{
			getegid, geteuid, getgid, getuid, setgid, setregid, setresgid, setresuid, setreuid,
			setuid,
		},
	},
};
use _exit::_exit;
use _llseek::{_llseek, lseek};
use _newselect::_newselect;
use access::access;
use arch_prctl::arch_prctl;
use bind::bind;
use r#break::r#break;
use brk::brk;
use chdir::chdir;
use chmod::chmod;
use chown::chown;
use chroot::chroot;
use clone::{clone, compat_clone};
use close::close;
use connect::connect;
use core::{arch::global_asm, fmt, ops::Deref, ptr};
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
use fstatfs::fstatfs;
use fstatfs64::fstatfs64;
use getcwd::getcwd;
use getdents::getdents;
use getpgid::getpgid;
use getpid::getpid;
use getppid::getppid;
use getrandom::getrandom;
use getresgid::getresgid;
use getresuid::getresuid;
use getrusage::getrusage;
use getsockname::getsockname;
use getsockopt::getsockopt;
use gettid::gettid;
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
use mount::mount;
use mprotect::mprotect;
use munmap::munmap;
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
use read::read;
use readlink::readlink;
use readv::readv;
use reboot::reboot;
use rename::rename;
use renameat2::renameat2;
use rmdir::rmdir;
use rt_sigaction::{compat_rt_sigaction, rt_sigaction};
use rt_sigprocmask::rt_sigprocmask;
use sched_yield::sched_yield;
use select::select;
use sendto::sendto;
use set_thread_area::set_thread_area;
use set_tid_address::set_tid_address;
use sethostname::sethostname;
use setpgid::setpgid;
use setsockopt::setsockopt;
use shutdown::shutdown;
use signal::signal;
use sigreturn::{rt_sigreturn, sigreturn};
use socket::socket;
use socketpair::socketpair;
use stat::{fstat, fstat64, lstat, lstat64, stat, stat64, statx};
use statfs::statfs;
use statfs64::statfs64;
use symlink::symlink;
use symlinkat::symlinkat;
use time::time32;
use tkill::tkill;
use truncate::truncate;
use umask::umask;
use umount::{umount, umount2};
use uname::uname;
use unlink::unlink;
use unlinkat::unlinkat;
use utils::{errno::EResult, ptr::arc::Arc};
use utimensat::utimensat;
use vfork::vfork;
use wait4::wait4;
use waitpid::waitpid;
use write::write;
use writev::writev;

/// The ID of the `sigreturn` system call, for use by the signal trampoline.
pub const SIGRETURN_ID: usize = 0x077;

/// A system call handler.
pub trait SyscallHandler<Args> {
	/// Calls the system call.
	///
	/// Arguments:
	/// - `name` is the name of the system call.
	/// - `frame` is the interrupt handler's stack frame.
	///
	/// The function returns the result of the system call.
	fn call(self, name: &str, frame: &mut IntFrame) -> EResult<usize>;
}

/// Implementation of [`SyscallHandler`] for functions with arguments.
macro_rules! impl_syscall_handler {
    ($($ty:ident),*) => {
		// Implementation **without** trailing reference to frame
        impl<F, $($ty,)*> SyscallHandler<($($ty,)*)> for F
        where F: FnOnce($($ty,)*) -> EResult<usize>,
			$($ty: FromSyscall,)*
        {
			#[allow(non_snake_case, unused_variables)]
            fn call(self, name: &str, frame: &mut IntFrame) -> EResult<usize> {
				#[cfg(feature = "strace")]
				let pid = {
					let pid = Process::current().get_pid();
					print!("[strace {pid}] {name}");
					pid
				};
                $(
                    let $ty = $ty::from_syscall(frame);
                )*
                let res = self($($ty,)*);
				#[cfg(feature = "strace")]
				println!("[strace {pid}] -> {res:?}");
				res
            }
        }

		// Implementation **with** trailing reference to frame
		#[allow(unused_parens)]
        impl<F, $($ty,)*> SyscallHandler<($($ty,)* &mut IntFrame)> for F
        where F: FnOnce($($ty,)* &mut IntFrame) -> EResult<usize>,
			$($ty: FromSyscall,)*
        {
			#[allow(non_snake_case, unused_variables)]
            fn call(self, name: &str, frame: &mut IntFrame) -> EResult<usize> {
				#[cfg(feature = "strace")]
				let pid = {
					let pid = Process::current().get_pid();
					print!("[strace {pid}] {name}");
					pid
				};
                $(
                    let $ty = $ty::from_syscall(frame);
                )*
                let res = self($($ty,)* frame);
				#[cfg(feature = "strace")]
				println!("[strace {pid}] -> {res:?}");
				res
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

/// Extracts a value from the process that made a system call.
pub trait FromSyscall {
	/// Constructs the value from the given process or syscall argument value.
	fn from_syscall(frame: &IntFrame) -> Self;
}

impl FromSyscall for Arc<Process> {
	#[inline]
	fn from_syscall(_frame: &IntFrame) -> Self {
		Process::current()
	}
}

impl FromSyscall for Arc<MemSpace> {
	#[inline]
	fn from_syscall(_frame: &IntFrame) -> Self {
		Process::current().mem_space.as_ref().unwrap().clone()
	}
}

impl FromSyscall for Arc<Mutex<FileDescriptorTable>> {
	#[inline]
	fn from_syscall(_frame: &IntFrame) -> Self {
		Process::current().file_descriptors.deref().clone().unwrap()
	}
}

impl FromSyscall for AccessProfile {
	fn from_syscall(_frame: &IntFrame) -> Self {
		Process::current().fs.lock().access_profile
	}
}

impl FromSyscall for ResolutionSettings {
	fn from_syscall(_frame: &IntFrame) -> Self {
		ResolutionSettings::for_process(&Process::current(), true)
	}
}

/// The umask of the process performing the system call.
pub struct Umask(file::Mode);

impl FromSyscall for Umask {
	fn from_syscall(_frame: &IntFrame) -> Self {
		Self(Process::current().fs.lock().umask())
	}
}

/// System call arguments.
#[derive(Debug)]
pub struct Args<T: fmt::Debug>(pub T);

impl<T: FromSyscallArg> FromSyscall for Args<T> {
	fn from_syscall(frame: &IntFrame) -> Self {
		let arg = T::from_syscall_arg(frame.get_syscall_arg(0), frame.is_compat());
		#[cfg(feature = "strace")]
		println!("({arg:?})");
		Self(arg)
	}
}

macro_rules! impl_from_syscall_args {
    ($($ty:ident),*) => {
		impl<$($ty: FromSyscallArg,)*> FromSyscall for Args<($($ty,)*)> {
			#[inline]
			#[allow(non_snake_case, unused_variables, unused_mut, unused_assignments)]
			fn from_syscall(
				frame: &IntFrame,
			) -> Self {
				let mut cursor = 0;
                $(
                    let $ty = $ty::from_syscall_arg(frame.get_syscall_arg(cursor), frame.is_compat());
					cursor += 1;
                )*
				let args = ($($ty,)*);
				#[cfg(feature = "strace")]
				println!("{args:?}");
				Args(args)
			}
		}
	};
}

impl_from_syscall_args!(T1);
impl_from_syscall_args!(T1, T2);
impl_from_syscall_args!(T1, T2, T3);
impl_from_syscall_args!(T1, T2, T3, T4);
impl_from_syscall_args!(T1, T2, T3, T4, T5);
impl_from_syscall_args!(T1, T2, T3, T4, T5, T6);

/// A value that can be constructed from a system call argument.
///
/// The [`fmt::Debug`] trait is required for the `strace` feature.
pub trait FromSyscallArg: fmt::Debug + Sized {
	/// Constructs a value from the given pointer passed as a system call argument.
	///
	/// Arguments:
	/// - `ptr`: is the pointer
	/// - `compat`: if true, pointers are 4 bytes in size, else 8 bytes
	fn from_syscall_arg(ptr: usize, compat: bool) -> Self;

	/// Constructs a value from the given pointer.
	///
	/// `compat` is set to `false`.
	fn from_ptr(ptr: usize) -> Self {
		Self::from_syscall_arg(ptr, false)
	}
}

macro_rules! impl_from_syscall_arg_primitive {
	($type:ident) => {
		impl FromSyscallArg for $type {
			fn from_syscall_arg(val: usize, _compat: bool) -> Self {
				val as _
			}
		}
	};
}

impl_from_syscall_arg_primitive!(i8);
impl_from_syscall_arg_primitive!(u8);
impl_from_syscall_arg_primitive!(i16);
impl_from_syscall_arg_primitive!(u16);
impl_from_syscall_arg_primitive!(i32);
impl_from_syscall_arg_primitive!(u32);
impl_from_syscall_arg_primitive!(i64);
impl_from_syscall_arg_primitive!(u64);
impl_from_syscall_arg_primitive!(isize);
impl_from_syscall_arg_primitive!(usize);

impl<T> FromSyscallArg for *const T {
	fn from_syscall_arg(val: usize, _compat: bool) -> Self {
		ptr::with_exposed_provenance(val)
	}
}

impl<T> FromSyscallArg for *mut T {
	fn from_syscall_arg(val: usize, _compat: bool) -> Self {
		ptr::with_exposed_provenance_mut(val)
	}
}

/// Syscall declaration.
macro_rules! syscall {
	($name:ident, $frame:expr) => {
		Some(SyscallHandler::call($name, stringify!($name), $frame))
	};
}

/// Executes the system call associated with the given `id` and returns its result.
///
/// If the syscall doesn't exist, the function returns `None`.
#[inline]
fn do_syscall32(id: usize, frame: &mut IntFrame) -> Option<EResult<usize>> {
	match id {
		0x001 => syscall!(_exit, frame),
		0x002 => syscall!(fork, frame),
		0x003 => syscall!(read, frame),
		0x004 => syscall!(write, frame),
		0x005 => syscall!(open, frame),
		0x006 => syscall!(close, frame),
		0x007 => syscall!(waitpid, frame),
		0x008 => syscall!(creat, frame),
		0x009 => syscall!(link, frame),
		0x00a => syscall!(unlink, frame),
		0x00b => syscall!(execve, frame),
		0x00c => syscall!(chdir, frame),
		0x00d => syscall!(time32, frame),
		0x00e => syscall!(mknod, frame),
		0x00f => syscall!(chmod, frame),
		0x010 => syscall!(lchown, frame),
		0x011 => syscall!(r#break, frame),
		// TODO 0x012 => syscall!(oldstat, frame),
		// TODO 0x013 => syscall!(lseek, frame),
		0x014 => syscall!(getpid, frame),
		0x015 => syscall!(mount, frame),
		0x016 => syscall!(umount, frame),
		0x017 => syscall!(setuid, frame),
		0x018 => syscall!(getuid, frame),
		// TODO 0x019 => syscall!(stime, frame),
		// TODO 0x01a => syscall!(ptrace, frame),
		// TODO 0x01b => syscall!(alarm, frame),
		// TODO 0x01c => syscall!(oldfstat, frame),
		// TODO 0x01d => syscall!(pause, frame),
		// TODO 0x01e => syscall!(utime, frame),
		// TODO 0x01f => syscall!(stty, frame),
		// TODO 0x020 => syscall!(gtty, frame),
		0x021 => syscall!(access, frame),
		// TODO 0x022 => syscall!(nice, frame),
		// TODO 0x023 => syscall!(ftime, frame),
		0x024 => syscall!(sync, frame),
		0x025 => syscall!(kill, frame),
		0x026 => syscall!(rename, frame),
		0x027 => syscall!(mkdir, frame),
		0x028 => syscall!(rmdir, frame),
		0x029 => syscall!(dup, frame),
		0x02a => syscall!(pipe, frame),
		// TODO 0x02b => syscall!(times, frame),
		// TODO 0x02c => syscall!(prof, frame),
		0x02d => syscall!(brk, frame),
		0x02e => syscall!(setgid, frame),
		0x02f => syscall!(getgid, frame),
		0x030 => syscall!(signal, frame),
		0x031 => syscall!(geteuid, frame),
		0x032 => syscall!(getegid, frame),
		// TODO 0x033 => syscall!(acct, frame),
		0x034 => syscall!(umount2, frame),
		// TODO 0x035 => syscall!(lock, frame),
		0x036 => syscall!(ioctl, frame),
		0x037 => syscall!(fcntl, frame),
		// TODO 0x038 => syscall!(mpx, frame),
		0x039 => syscall!(setpgid, frame),
		// TODO 0x03a => syscall!(ulimit, frame),
		// TODO 0x03b => syscall!(oldolduname, frame),
		0x03c => syscall!(umask, frame),
		0x03d => syscall!(chroot, frame),
		// TODO 0x03e => syscall!(ustat, frame),
		0x03f => syscall!(dup2, frame),
		0x040 => syscall!(getppid, frame),
		// TODO 0x041 => syscall!(getpgrp, frame),
		// TODO 0x042 => syscall!(setsid, frame),
		// TODO 0x043 => syscall!(sigaction, frame),
		// TODO 0x044 => syscall!(sgetmask, frame),
		// TODO 0x045 => syscall!(ssetmask, frame),
		0x046 => syscall!(setreuid, frame),
		0x047 => syscall!(setregid, frame),
		// TODO 0x048 => syscall!(sigsuspend, frame),
		// TODO 0x049 => syscall!(sigpending, frame),
		0x04a => syscall!(sethostname, frame),
		// TODO 0x04b => syscall!(setrlimit, frame),
		// TODO 0x04c => syscall!(getrlimit, frame),
		0x04d => syscall!(getrusage, frame),
		// TODO 0x04e => syscall!(gettimeofday, frame),
		// TODO 0x04f => syscall!(settimeofday, frame),
		// TODO 0x050 => syscall!(getgroups, frame),
		// TODO 0x051 => syscall!(setgroups, frame),
		0x052 => syscall!(select, frame),
		0x053 => syscall!(symlink, frame),
		// TODO 0x054 => syscall!(oldlstat, frame),
		0x055 => syscall!(readlink, frame),
		// TODO 0x056 => syscall!(uselib, frame),
		// TODO 0x057 => syscall!(swapon, frame),
		0x058 => syscall!(reboot, frame),
		// TODO 0x059 => syscall!(readdir, frame),
		0x05a => syscall!(mmap, frame),
		0x05b => syscall!(munmap, frame),
		0x05c => syscall!(truncate, frame),
		// TODO 0x05d => syscall!(ftruncate, frame),
		0x05e => syscall!(fchmod, frame),
		// TODO 0x05f => syscall!(fchown, frame),
		// TODO 0x060 => syscall!(getpriority, frame),
		// TODO 0x061 => syscall!(setpriority, frame),
		// TODO 0x062 => syscall!(profil, frame),
		0x063 => syscall!(statfs, frame),
		0x064 => syscall!(fstatfs, frame),
		// TODO 0x065 => syscall!(ioperm, frame),
		// TODO 0x066 => syscall!(socketcall, frame),
		// TODO 0x067 => syscall!(syslog, frame),
		// TODO 0x068 => syscall!(setitimer, frame),
		// TODO 0x069 => syscall!(getitimer, frame),
		0x06a => syscall!(stat, frame),
		0x06b => syscall!(lstat, frame),
		0x06c => syscall!(fstat, frame),
		// TODO 0x06d => syscall!(olduname, frame),
		// TODO 0x06e => syscall!(iopl, frame),
		// TODO 0x06f => syscall!(vhangup, frame),
		// TODO 0x070 => syscall!(idle, frame),
		// TODO 0x071 => syscall!(vm86old, frame),
		0x072 => syscall!(wait4, frame),
		// TODO 0x073 => syscall!(swapoff, frame),
		// TODO 0x074 => syscall!(sysinfo, frame),
		// TODO 0x075 => syscall!(ipc, frame),
		0x076 => syscall!(fsync, frame),
		SIGRETURN_ID => syscall!(sigreturn, frame),
		0x078 => syscall!(compat_clone, frame),
		// TODO 0x079 => syscall!(setdomainname, frame),
		0x07a => syscall!(uname, frame),
		// TODO 0x07c => syscall!(adjtimex, frame),
		0x07d => syscall!(mprotect, frame),
		// TODO 0x07e => syscall!(sigprocmask, frame),
		// TODO 0x07f => syscall!(create_module, frame),
		0x080 => syscall!(init_module, frame),
		0x081 => syscall!(delete_module, frame),
		// TODO 0x083 => syscall!(quotactl, frame),
		0x084 => syscall!(getpgid, frame),
		0x085 => syscall!(fchdir, frame),
		// TODO 0x086 => syscall!(bdflush, frame),
		// TODO 0x087 => syscall!(sysfs, frame),
		// TODO 0x088 => syscall!(personality, frame),
		// TODO 0x089 => syscall!(afs_syscall, frame),
		// TODO 0x08a => syscall!(setfsuid, frame),
		// TODO 0x08b => syscall!(setfsgid, frame),
		0x08c => syscall!(_llseek, frame),
		0x08d => syscall!(getdents, frame),
		0x08e => syscall!(_newselect, frame),
		// TODO 0x08f => syscall!(flock, frame),
		0x090 => syscall!(msync, frame),
		0x091 => syscall!(readv, frame),
		0x092 => syscall!(writev, frame),
		// TODO 0x093 => syscall!(getsid, frame),
		0x094 => syscall!(fdatasync, frame),
		// TODO 0x095 => syscall!(_sysctl, frame),
		// TODO 0x096 => syscall!(mlock, frame),
		// TODO 0x097 => syscall!(munlock, frame),
		// TODO 0x098 => syscall!(mlockall, frame),
		// TODO 0x099 => syscall!(munlockall, frame),
		// TODO 0x09a => syscall!(sched_setparam, frame),
		// TODO 0x09b => syscall!(sched_getparam, frame),
		// TODO 0x09c => syscall!(sched_setscheduler, frame),
		// TODO 0x09d => syscall!(sched_getscheduler, frame),
		0x09e => syscall!(sched_yield, frame),
		// TODO 0x09f => syscall!(sched_get_priority_max, frame),
		// TODO 0x0a0 => syscall!(sched_get_priority_min, frame),
		// TODO 0x0a1 => syscall!(sched_rr_get_interval, frame),
		0x0a2 => syscall!(nanosleep32, frame),
		// TODO 0x0a3 => syscall!(mremap, frame),
		0x0a4 => syscall!(setresuid, frame),
		0x0a5 => syscall!(getresuid, frame),
		// TODO 0x0a6 => syscall!(vm86, frame),
		// TODO 0x0a7 => syscall!(query_module, frame),
		0x0a8 => syscall!(poll, frame),
		// TODO 0x0a9 => syscall!(nfsservctl, frame),
		0x0aa => syscall!(setresgid, frame),
		0x0ab => syscall!(getresgid, frame),
		// TODO 0x0ac => syscall!(prctl, frame),
		0x0ad => syscall!(rt_sigreturn, frame),
		0x0ae => syscall!(compat_rt_sigaction, frame),
		0x0af => syscall!(rt_sigprocmask, frame),
		// TODO 0x0b0 => syscall!(rt_sigpending, frame),
		// TODO 0x0b1 => syscall!(rt_sigtimedwait, frame),
		// TODO 0x0b2 => syscall!(rt_sigqueueinfo, frame),
		// TODO 0x0b3 => syscall!(rt_sigsuspend, frame),
		// TODO 0x0b4 => syscall!(pread64, frame),
		// TODO 0x0b5 => syscall!(pwrite64, frame),
		0x0b6 => syscall!(chown, frame),
		0x0b7 => syscall!(getcwd, frame),
		// TODO 0x0b8 => syscall!(capget, frame),
		// TODO 0x0b9 => syscall!(capset, frame),
		// TODO 0x0ba => syscall!(sigaltstack, frame),
		// TODO 0x0bb => syscall!(sendfile, frame),
		// TODO 0x0bc => syscall!(getpmsg, frame),
		// TODO 0x0bd => syscall!(putpmsg, frame),
		0x0be => syscall!(vfork, frame),
		// TODO 0x0bf => syscall!(ugetrlimit, frame),
		0x0c0 => syscall!(mmap2, frame),
		// TODO 0x0c1 => syscall!(truncate64, frame),
		// TODO 0x0c2 => syscall!(ftruncate64, frame),
		0x0c3 => syscall!(stat64, frame),
		0x0c4 => syscall!(lstat64, frame),
		0x0c5 => syscall!(fstat64, frame),
		// TODO 0x0c6 => syscall!(lchown32, frame),
		0x0c7 => syscall!(getuid, frame),   // getuid32
		0x0c8 => syscall!(getgid, frame),   // getgid32
		0x0c9 => syscall!(geteuid, frame),  // geteuid32
		0x0ca => syscall!(getegid, frame),  // getegid32
		0x0cb => syscall!(setreuid, frame), // setreuid32
		0x0cc => syscall!(setregid, frame), // setregid32
		// TODO 0x0cd => syscall!(getgroups32, frame),
		// TODO 0x0ce => syscall!(setgroups32, frame),
		// TODO 0x0cf => syscall!(fchown32, frame),
		0x0d0 => syscall!(setresuid, frame), // setresuid32
		0x0d1 => syscall!(getresuid, frame), // getresuid32
		0x0d2 => syscall!(setresgid, frame), // setresgid32
		0x0d3 => syscall!(getresgid, frame), // getresgid32
		0x0d4 => syscall!(chown, frame),     // chown32
		0x0d5 => syscall!(setuid, frame),    // setuid32
		0x0d6 => syscall!(setgid, frame),    // setgid32
		// TODO 0x0d7 => syscall!(setfsuid32, frame),
		// TODO 0x0d8 => syscall!(setfsgid32, frame),
		// TODO 0x0d9 => syscall!(pivot_root, frame),
		// TODO 0x0da => syscall!(mincore, frame),
		0x0db => syscall!(madvise, frame),
		0x0dc => syscall!(getdents64, frame),
		0x0dd => syscall!(fcntl64, frame),
		0x0e0 => syscall!(gettid, frame),
		// TODO 0x0e1 => syscall!(readahead, frame),
		// TODO 0x0e2 => syscall!(setxattr, frame),
		// TODO 0x0e3 => syscall!(lsetxattr, frame),
		// TODO 0x0e4 => syscall!(fsetxattr, frame),
		// TODO 0x0e5 => syscall!(getxattr, frame),
		// TODO 0x0e6 => syscall!(lgetxattr, frame),
		// TODO 0x0e7 => syscall!(fgetxattr, frame),
		// TODO 0x0e8 => syscall!(listxattr, frame),
		// TODO 0x0e9 => syscall!(llistxattr, frame),
		// TODO 0x0ea => syscall!(flistxattr, frame),
		// TODO 0x0eb => syscall!(removexattr, frame),
		// TODO 0x0ec => syscall!(lremovexattr, frame),
		// TODO 0x0ed => syscall!(fremovexattr, frame),
		0x0ee => syscall!(tkill, frame),
		// TODO 0x0ef => syscall!(sendfile64, frame),
		// TODO 0x0f0 => syscall!(futex, frame),
		// TODO 0x0f1 => syscall!(sched_setaffinity, frame),
		// TODO 0x0f2 => syscall!(sched_getaffinity, frame),
		0x0f3 => syscall!(set_thread_area, frame),
		// TODO 0x0f4 => syscall!(get_thread_area, frame),
		// TODO 0x0f5 => syscall!(io_setup, frame),
		// TODO 0x0f6 => syscall!(io_destroy, frame),
		// TODO 0x0f7 => syscall!(io_getevents, frame),
		// TODO 0x0f8 => syscall!(io_submit, frame),
		// TODO 0x0f9 => syscall!(io_cancel, frame),
		// TODO 0x0fa => syscall!(fadvise64, frame),
		0x0fc => syscall!(exit_group, frame),
		// TODO 0x0fd => syscall!(lookup_dcookie, frame),
		// TODO 0x0fe => syscall!(epoll_create, frame),
		// TODO 0x0ff => syscall!(epoll_ctl, frame),
		// TODO 0x100 => syscall!(epoll_wait, frame),
		// TODO 0x101 => syscall!(remap_file_pages, frame),
		0x102 => syscall!(set_tid_address, frame),
		0x103 => syscall!(timer_create, frame),
		0x104 => syscall!(timer_settime, frame),
		// TODO 0x105 => syscall!(timer_gettime, frame),
		// TODO 0x106 => syscall!(timer_getoverrun, frame),
		0x107 => syscall!(timer_delete, frame),
		// TODO 0x108 => syscall!(clock_settime, frame),
		0x109 => syscall!(clock_gettime, frame),
		// TODO 0x10a => syscall!(clock_getres, frame),
		// TODO 0x10b => syscall!(clock_nanosleep, frame),
		0x10c => syscall!(statfs64, frame),
		0x10d => syscall!(fstatfs64, frame),
		// TODO 0x10e => syscall!(tgkill, frame),
		// TODO 0x10f => syscall!(utimes, frame),
		0x110 => syscall!(fadvise64_64, frame),
		// TODO 0x111 => syscall!(vserver, frame),
		// TODO 0x112 => syscall!(mbind, frame),
		// TODO 0x113 => syscall!(get_mempolicy, frame),
		// TODO 0x114 => syscall!(set_mempolicy, frame),
		// TODO 0x115 => syscall!(mq_open, frame),
		// TODO 0x116 => syscall!(mq_unlink, frame),
		// TODO 0x117 => syscall!(mq_timedsend, frame),
		// TODO 0x118 => syscall!(mq_timedreceive, frame),
		// TODO 0x119 => syscall!(mq_notify, frame),
		// TODO 0x11a => syscall!(mq_getsetattr, frame),
		// TODO 0x11b => syscall!(kexec_load, frame),
		// TODO 0x11c => syscall!(waitid, frame),
		// TODO 0x11e => syscall!(add_key, frame),
		// TODO 0x11f => syscall!(request_key, frame),
		// TODO 0x120 => syscall!(keyctl, frame),
		// TODO 0x121 => syscall!(ioprio_set, frame),
		// TODO 0x122 => syscall!(ioprio_get, frame),
		// TODO 0x123 => syscall!(inotify_init, frame),
		// TODO 0x124 => syscall!(inotify_add_watch, frame),
		// TODO 0x125 => syscall!(inotify_rm_watch, frame),
		// TODO 0x126 => syscall!(migrate_pages, frame),
		0x127 => syscall!(openat, frame),
		// TODO 0x128 => syscall!(mkdirat, frame),
		// TODO 0x129 => syscall!(mknodat, frame),
		// TODO 0x12a => syscall!(fchownat, frame),
		// TODO 0x12b => syscall!(futimesat, frame),
		// TODO 0x12c => syscall!(fstatat64, frame),
		0x12d => syscall!(unlinkat, frame),
		// TODO 0x12e => syscall!(renameat, frame),
		0x12f => syscall!(linkat, frame),
		0x130 => syscall!(symlinkat, frame),
		// TODO 0x131 => syscall!(readlinkat, frame),
		0x132 => syscall!(fchmodat, frame),
		0x133 => syscall!(faccessat, frame),
		0x134 => syscall!(pselect6, frame),
		// TODO 0x135 => syscall!(ppoll, frame),
		// TODO 0x136 => syscall!(unshare, frame),
		// TODO 0x137 => syscall!(set_robust_list, frame),
		// TODO 0x138 => syscall!(get_robust_list, frame),
		// TODO 0x139 => syscall!(splice, frame),
		// TODO 0x13a => syscall!(sync_file_range, frame),
		// TODO 0x13b => syscall!(tee, frame),
		// TODO 0x13c => syscall!(vmsplice, frame),
		// TODO 0x13d => syscall!(move_pages, frame),
		// TODO 0x13e => syscall!(getcpu, frame),
		// TODO 0x13f => syscall!(epoll_pwait, frame),
		0x140 => syscall!(utimensat, frame),
		// TODO 0x141 => syscall!(signalfd, frame),
		// TODO 0x142 => syscall!(timerfd_create, frame),
		// TODO 0x143 => syscall!(eventfd, frame),
		// TODO 0x144 => syscall!(fallocate, frame),
		// TODO 0x145 => syscall!(timerfd_settime, frame),
		// TODO 0x146 => syscall!(timerfd_gettime, frame),
		// TODO 0x147 => syscall!(signalfd4, frame),
		// TODO 0x148 => syscall!(eventfd2, frame),
		// TODO 0x149 => syscall!(epoll_create1, frame),
		// TODO 0x14a => syscall!(dup3, frame),
		0x14b => syscall!(pipe2, frame),
		// TODO 0x14c => syscall!(inotify_init1, frame),
		0x14d => syscall!(preadv, frame),
		0x14e => syscall!(pwritev, frame),
		// TODO 0x14f => syscall!(rt_tgsigqueueinfo, frame),
		// TODO 0x150 => syscall!(perf_event_open, frame),
		// TODO 0x151 => syscall!(recvmmsg, frame),
		// TODO 0x152 => syscall!(fanotify_init, frame),
		// TODO 0x153 => syscall!(fanotify_mark, frame),
		0x154 => syscall!(prlimit64, frame),
		// TODO 0x155 => syscall!(name_to_handle_at, frame),
		// TODO 0x156 => syscall!(open_by_handle_at, frame),
		// TODO 0x157 => syscall!(clock_adjtime, frame),
		0x158 => syscall!(syncfs, frame),
		// TODO 0x159 => syscall!(sendmmsg, frame),
		// TODO 0x15a => syscall!(setns, frame),
		// TODO 0x15b => syscall!(process_vm_readv, frame),
		// TODO 0x15c => syscall!(process_vm_writev, frame),
		// TODO 0x15d => syscall!(kcmp, frame),
		0x15e => syscall!(finit_module, frame),
		// TODO 0x15f => syscall!(sched_setattr, frame),
		// TODO 0x160 => syscall!(sched_getattr, frame),
		0x161 => syscall!(renameat2, frame),
		// TODO 0x162 => syscall!(seccomp, frame),
		0x163 => syscall!(getrandom, frame),
		// TODO 0x164 => syscall!(memfd_create, frame),
		// TODO 0x165 => syscall!(bpf, frame),
		// TODO 0x166 => syscall!(execveat, frame),
		0x167 => syscall!(socket, frame),
		0x168 => syscall!(socketpair, frame),
		0x169 => syscall!(bind, frame),
		0x16a => syscall!(connect, frame),
		// TODO 0x16b => syscall!(listen, frame),
		// TODO 0x16c => syscall!(accept4, frame),
		0x16d => syscall!(getsockopt, frame),
		0x16e => syscall!(setsockopt, frame),
		0x16f => syscall!(getsockname, frame),
		// TODO 0x170 => syscall!(getpeername, frame),
		0x171 => syscall!(sendto, frame),
		// TODO 0x172 => syscall!(sendmsg, frame),
		// TODO 0x173 => syscall!(recvfrom, frame),
		// TODO 0x174 => syscall!(recvmsg, frame),
		0x175 => syscall!(shutdown, frame),
		// TODO 0x176 => syscall!(userfaultfd, frame),
		// TODO 0x177 => syscall!(membarrier, frame),
		// TODO 0x178 => syscall!(mlock2, frame),
		// TODO 0x179 => syscall!(copy_file_range, frame),
		0x17a => syscall!(preadv2, frame),
		0x17b => syscall!(pwritev2, frame),
		// TODO 0x17c => syscall!(pkey_mprotect, frame),
		// TODO 0x17d => syscall!(pkey_alloc, frame),
		// TODO 0x17e => syscall!(pkey_free, frame),
		0x17f => syscall!(statx, frame),
		0x180 => syscall!(arch_prctl, frame),
		// TODO 0x181 => syscall!(io_pgetevents, frame),
		// TODO 0x182 => syscall!(rseq, frame),
		// TODO 0x189 => syscall!(semget, frame),
		// TODO 0x18a => syscall!(semctl, frame),
		// TODO 0x18b => syscall!(shmget, frame),
		// TODO 0x18c => syscall!(shmctl, frame),
		// TODO 0x18d => syscall!(shmat, frame),
		// TODO 0x18e => syscall!(shmdt, frame),
		// TODO 0x18f => syscall!(msgget, frame),
		// TODO 0x190 => syscall!(msgsnd, frame),
		// TODO 0x191 => syscall!(msgrcv, frame),
		// TODO 0x192 => syscall!(msgctl, frame),
		0x193 => syscall!(clock_gettime64, frame),
		// TODO 0x194 => syscall!(clock_settime64, frame),
		// TODO 0x195 => syscall!(clock_adjtime64, frame),
		// TODO 0x196 => syscall!(clock_getres_time64, frame),
		// TODO 0x197 => syscall!(clock_nanosleep_time64, frame),
		// TODO 0x198 => syscall!(timer_gettime64, frame),
		// TODO 0x199 => syscall!(timer_settime64, frame),
		// TODO 0x19a => syscall!(timerfd_gettime64, frame),
		// TODO 0x19b => syscall!(timerfd_settime64, frame),
		// TODO 0x19c => syscall!(utimensat_time64, frame),
		// TODO 0x19d => syscall!(pselect6_time64, frame),
		// TODO 0x19e => syscall!(ppoll_time64, frame),
		// TODO 0x1a0 => syscall!(io_pgetevents_time64, frame),
		// TODO 0x1a1 => syscall!(recvmmsg_time64, frame),
		// TODO 0x1a2 => syscall!(mq_timedsend_time64, frame),
		// TODO 0x1a3 => syscall!(mq_timedreceive_time64, frame),
		// TODO 0x1a4 => syscall!(semtimedop_time64, frame),
		// TODO 0x1a5 => syscall!(rt_sigtimedwait_time64, frame),
		// TODO 0x1a6 => syscall!(futex_time64, frame),
		// TODO 0x1a7 => syscall!(sched_rr_get_interval_time64, frame),
		// TODO 0x1a8 => syscall!(pidfd_send_signal, frame),
		// TODO 0x1a9 => syscall!(io_uring_setup, frame),
		// TODO 0x1aa => syscall!(io_uring_enter, frame),
		// TODO 0x1ab => syscall!(io_uring_register, frame),
		// TODO 0x1ac => syscall!(open_tree, frame),
		// TODO 0x1ad => syscall!(move_mount, frame),
		// TODO 0x1ae => syscall!(fsopen, frame),
		// TODO 0x1af => syscall!(fsconfig, frame),
		// TODO 0x1b0 => syscall!(fsmount, frame),
		// TODO 0x1b1 => syscall!(fspick, frame),
		// TODO 0x1b2 => syscall!(pidfd_open, frame),
		// TODO 0x1b3 => syscall!(clone3, frame),
		// TODO 0x1b4 => syscall!(close_range, frame),
		// TODO 0x1b5 => syscall!(openat2, frame),
		// TODO 0x1b6 => syscall!(pidfd_getfd, frame),
		0x1b7 => syscall!(faccessat2, frame),
		// TODO 0x1b8 => syscall!(process_madvise, frame),
		// TODO 0x1b9 => syscall!(epoll_pwait2, frame),
		// TODO 0x1ba => syscall!(mount_setattr, frame),
		// TODO 0x1bb => syscall!(quotactl_fd, frame),
		// TODO 0x1bc => syscall!(landlock_create_ruleset, frame),
		// TODO 0x1bd => syscall!(landlock_add_rule, frame),
		// TODO 0x1be => syscall!(landlock_restrict_self, frame),
		// TODO 0x1bf => syscall!(memfd_secret, frame),
		// TODO 0x1c0 => syscall!(process_mrelease, frame),
		// TODO 0x1c1 => syscall!(futex_waitv, frame),
		// TODO 0x1c2 => syscall!(set_mempolicy_home_node, frame),
		_ => None,
	}
}

#[cfg(target_arch = "x86_64")]
#[inline]
fn do_syscall64(id: usize, frame: &mut IntFrame) -> Option<EResult<usize>> {
	match id {
		0x000 => syscall!(read, frame),
		0x001 => syscall!(write, frame),
		0x002 => syscall!(open, frame),
		0x003 => syscall!(close, frame),
		0x004 => syscall!(stat64, frame),
		0x005 => syscall!(fstat64, frame),
		0x006 => syscall!(lstat64, frame),
		0x007 => syscall!(poll, frame),
		0x008 => syscall!(lseek, frame),
		0x009 => syscall!(mmap, frame),
		0x00a => syscall!(mprotect, frame),
		0x00b => syscall!(munmap, frame),
		0x00c => syscall!(brk, frame),
		0x00d => syscall!(rt_sigaction, frame),
		0x00e => syscall!(rt_sigprocmask, frame),
		0x00f => syscall!(rt_sigreturn, frame),
		0x010 => syscall!(ioctl, frame),
		// TODO 0x011 => syscall!(pread64, frame),
		// TODO 0x012 => syscall!(pwrite64, frame),
		0x013 => syscall!(readv, frame),
		0x014 => syscall!(writev, frame),
		0x015 => syscall!(access, frame),
		0x016 => syscall!(pipe, frame),
		0x017 => syscall!(select, frame),
		0x018 => syscall!(sched_yield, frame),
		// TODO 0x019 => syscall!(mremap, frame),
		0x01a => syscall!(msync, frame),
		// TODO 0x01b => syscall!(mincore, frame),
		0x01c => syscall!(madvise, frame),
		// TODO 0x01d => syscall!(shmget, frame),
		// TODO 0x01e => syscall!(shmat, frame),
		// TODO 0x01f => syscall!(shmctl, frame),
		0x020 => syscall!(dup, frame),
		0x021 => syscall!(dup2, frame),
		// TODO 0x022 => syscall!(pause, frame),
		0x023 => syscall!(nanosleep64, frame),
		// TODO 0x024 => syscall!(getitimer, frame),
		// TODO 0x025 => syscall!(alarm, frame),
		// TODO 0x026 => syscall!(setitimer, frame),
		0x027 => syscall!(getpid, frame),
		// TODO 0x028 => syscall!(sendfile, frame),
		0x029 => syscall!(socket, frame),
		0x02a => syscall!(connect, frame),
		// TODO 0x02b => syscall!(accept, frame),
		0x02c => syscall!(sendto, frame),
		// TODO 0x02d => syscall!(recvfrom, frame),
		// TODO 0x02e => syscall!(sendmsg, frame),
		// TODO 0x02f => syscall!(recvmsg, frame),
		0x030 => syscall!(shutdown, frame),
		0x031 => syscall!(bind, frame),
		// TODO 0x032 => syscall!(listen, frame),
		0x033 => syscall!(getsockname, frame),
		// TODO 0x034 => syscall!(getpeername, frame),
		0x035 => syscall!(socketpair, frame),
		0x036 => syscall!(setsockopt, frame),
		0x037 => syscall!(getsockopt, frame),
		0x038 => syscall!(clone, frame),
		0x039 => syscall!(fork, frame),
		0x03a => syscall!(vfork, frame),
		0x03b => syscall!(execve, frame),
		// TODO 0x03c => syscall!(exit, frame),
		0x03d => syscall!(wait4, frame),
		0x03e => syscall!(kill, frame),
		0x03f => syscall!(uname, frame),
		// TODO 0x040 => syscall!(semget, frame),
		// TODO 0x041 => syscall!(semop, frame),
		// TODO 0x042 => syscall!(semctl, frame),
		// TODO 0x043 => syscall!(shmdt, frame),
		// TODO 0x044 => syscall!(msgget, frame),
		// TODO 0x045 => syscall!(msgsnd, frame),
		// TODO 0x046 => syscall!(msgrcv, frame),
		// TODO 0x047 => syscall!(msgctl, frame),
		0x048 => syscall!(fcntl, frame),
		// TODO 0x049 => syscall!(flock, frame),
		0x04a => syscall!(fsync, frame),
		0x04b => syscall!(fdatasync, frame),
		0x04c => syscall!(truncate, frame),
		// TODO 0x04d => syscall!(ftruncate, frame),
		0x04e => syscall!(getdents, frame),
		0x04f => syscall!(getcwd, frame),
		0x050 => syscall!(chdir, frame),
		0x051 => syscall!(fchdir, frame),
		0x052 => syscall!(rename, frame),
		0x053 => syscall!(mkdir, frame),
		0x054 => syscall!(rmdir, frame),
		0x055 => syscall!(creat, frame),
		0x056 => syscall!(link, frame),
		0x057 => syscall!(unlink, frame),
		0x058 => syscall!(symlink, frame),
		0x059 => syscall!(readlink, frame),
		0x05a => syscall!(chmod, frame),
		0x05b => syscall!(fchmod, frame),
		0x05c => syscall!(chown, frame),
		// TODO 0x05d => syscall!(fchown, frame),
		0x05e => syscall!(lchown, frame),
		0x05f => syscall!(umask, frame),
		// TODO 0x060 => syscall!(gettimeofday, frame),
		// TODO 0x061 => syscall!(getrlimit, frame),
		0x062 => syscall!(getrusage, frame),
		// TODO 0x063 => syscall!(sysinfo, frame),
		// TODO 0x064 => syscall!(times, frame),
		// TODO 0x065 => syscall!(ptrace, frame),
		0x066 => syscall!(getuid, frame),
		// TODO 0x067 => syscall!(syslog, frame),
		0x068 => syscall!(getgid, frame),
		0x069 => syscall!(setuid, frame),
		0x06a => syscall!(setgid, frame),
		0x06b => syscall!(geteuid, frame),
		0x06c => syscall!(getegid, frame),
		0x06d => syscall!(setpgid, frame),
		0x06e => syscall!(getppid, frame),
		// TODO 0x06f => syscall!(getpgrp, frame),
		// TODO 0x070 => syscall!(setsid, frame),
		0x071 => syscall!(setreuid, frame),
		0x072 => syscall!(setregid, frame),
		// TODO 0x073 => syscall!(getgroups, frame),
		// TODO 0x074 => syscall!(setgroups, frame),
		0x075 => syscall!(setresuid, frame),
		0x076 => syscall!(getresuid, frame),
		0x077 => syscall!(setresgid, frame),
		0x078 => syscall!(getresgid, frame),
		0x079 => syscall!(getpgid, frame),
		// TODO 0x07a => syscall!(setfsuid, frame),
		// TODO 0x07b => syscall!(setfsgid, frame),
		// TODO 0x07c => syscall!(getsid, frame),
		// TODO 0x07d => syscall!(capget, frame),
		// TODO 0x07e => syscall!(capset, frame),
		// TODO 0x07f => syscall!(rt_sigpending, frame),
		// TODO 0x080 => syscall!(rt_sigtimedwait, frame),
		// TODO 0x081 => syscall!(rt_sigqueueinfo, frame),
		// TODO 0x082 => syscall!(rt_sigsuspend, frame),
		// TODO 0x083 => syscall!(sigaltstack, frame),
		// TODO 0x084 => syscall!(utime, frame),
		0x085 => syscall!(mknod, frame),
		// TODO 0x086 => syscall!(useli, frame),
		// TODO 0x087 => syscall!(personality, frame),
		// TODO 0x088 => syscall!(ustat, frame),
		0x089 => syscall!(statfs, frame),
		0x08a => syscall!(fstatfs, frame),
		// TODO 0x08b => syscall!(sysfs, frame),
		// TODO 0x08c => syscall!(getpriority, frame),
		// TODO 0x08d => syscall!(setpriority, frame),
		// TODO 0x08e => syscall!(sched_setparam, frame),
		// TODO 0x08f => syscall!(sched_getparam, frame),
		// TODO 0x090 => syscall!(sched_setscheduler, frame),
		// TODO 0x091 => syscall!(sched_getscheduler, frame),
		// TODO 0x092 => syscall!(sched_get_priority_max, frame),
		// TODO 0x093 => syscall!(sched_get_priority_min, frame),
		// TODO 0x094 => syscall!(sched_rr_get_interval, frame),
		// TODO 0x095 => syscall!(mlock, frame),
		// TODO 0x096 => syscall!(munlock, frame),
		// TODO 0x097 => syscall!(mlockall, frame),
		// TODO 0x098 => syscall!(munlockall, frame),
		// TODO 0x099 => syscall!(vhangup, frame),
		// TODO 0x09a => syscall!(modify_ldt, frame),
		// TODO 0x09b => syscall!(pivot_root, frame),
		// TODO 0x09c => syscall!(_sysctl, frame),
		// TODO 0x09d => syscall!(prctl, frame),
		0x09e => syscall!(arch_prctl, frame),
		// TODO 0x09f => syscall!(adjtimex, frame),
		// TODO 0x0a0 => syscall!(setrlimit, frame),
		0x0a1 => syscall!(chroot, frame),
		0x0a2 => syscall!(sync, frame),
		// TODO 0x0a3 => syscall!(acct, frame),
		// TODO 0x0a4 => syscall!(settimeofday, frame),
		0x0a5 => syscall!(mount, frame),
		0x0a6 => syscall!(umount2, frame),
		// TODO 0x0a7 => syscall!(swapon, frame),
		// TODO 0x0a8 => syscall!(swapoff, frame),
		0x0a9 => syscall!(reboot, frame),
		0x0aa => syscall!(sethostname, frame),
		// TODO 0x0ab => syscall!(setdomainname, frame),
		// TODO 0x0ac => syscall!(iopl, frame),
		// TODO 0x0ad => syscall!(ioperm, frame),
		// TODO 0x0ae => syscall!(create_modul, frame),
		0x0af => syscall!(init_module, frame),
		0x0b0 => syscall!(delete_module, frame),
		// TODO 0x0b1 => syscall!(get_kernel_sym, frame),
		// TODO 0x0b2 => syscall!(query_modul, frame),
		// TODO 0x0b3 => syscall!(quotactl, frame),
		// TODO 0x0b4 => syscall!(nfsservct, frame),
		// TODO 0x0b5 => syscall!(getpms, frame),
		// TODO 0x0b6 => syscall!(putpms, frame),
		// TODO 0x0b7 => syscall!(afs_syscal, frame),
		// TODO 0x0b8 => syscall!(tuxcal, frame),
		// TODO 0x0b9 => syscall!(securit, frame),
		0x0ba => syscall!(gettid, frame),
		// TODO 0x0bb => syscall!(readahead, frame),
		// TODO 0x0bc => syscall!(setxattr, frame),
		// TODO 0x0bd => syscall!(lsetxattr, frame),
		// TODO 0x0be => syscall!(fsetxattr, frame),
		// TODO 0x0bf => syscall!(getxattr, frame),
		// TODO 0x0c0 => syscall!(lgetxattr, frame),
		// TODO 0x0c1 => syscall!(fgetxattr, frame),
		// TODO 0x0c2 => syscall!(listxattr, frame),
		// TODO 0x0c3 => syscall!(llistxattr, frame),
		// TODO 0x0c4 => syscall!(flistxattr, frame),
		// TODO 0x0c5 => syscall!(removexattr, frame),
		// TODO 0x0c6 => syscall!(lremovexattr, frame),
		// TODO 0x0c7 => syscall!(fremovexattr, frame),
		0x0c8 => syscall!(tkill, frame),
		0x0c9 => syscall!(time64, frame),
		// TODO 0x0ca => syscall!(futex, frame),
		// TODO 0x0cb => syscall!(sched_setaffinity, frame),
		// TODO 0x0cc => syscall!(sched_getaffinity, frame),
		// TODO 0x0cd => syscall!(set_thread_are, frame),
		// TODO 0x0ce => syscall!(io_setup, frame),
		// TODO 0x0cf => syscall!(io_destroy, frame),
		// TODO 0x0d0 => syscall!(io_getevents, frame),
		// TODO 0x0d1 => syscall!(io_submit, frame),
		// TODO 0x0d2 => syscall!(io_cancel, frame),
		// TODO 0x0d3 => syscall!(get_thread_are, frame),
		// TODO 0x0d4 => syscall!(lookup_dcooki, frame),
		// TODO 0x0d5 => syscall!(epoll_create, frame),
		// TODO 0x0d6 => syscall!(epoll_ctl_ol, frame),
		// TODO 0x0d7 => syscall!(epoll_wait_ol, frame),
		// TODO 0x0d8 => syscall!(remap_file_pages, frame),
		0x0d9 => syscall!(getdents64, frame),
		0x0da => syscall!(set_tid_address, frame),
		// TODO 0x0db => syscall!(restart_syscall, frame),
		// TODO 0x0dc => syscall!(semtimedop, frame),
		// TODO 0x0dd => syscall!(fadvise64, frame),
		0x0de => syscall!(timer_create, frame),
		0x0df => syscall!(timer_settime, frame),
		// TODO 0x0e0 => syscall!(timer_gettime, frame),
		// TODO 0x0e1 => syscall!(timer_getoverrun, frame),
		0x0e2 => syscall!(timer_delete, frame),
		// TODO 0x0e3 => syscall!(clock_settime, frame),
		0x0e4 => syscall!(clock_gettime, frame),
		// TODO 0x0e5 => syscall!(clock_getres, frame),
		// TODO 0x0e6 => syscall!(clock_nanosleep, frame),
		0x0e7 => syscall!(exit_group, frame),
		// TODO 0x0e8 => syscall!(epoll_wait, frame),
		// TODO 0x0e9 => syscall!(epoll_ctl, frame),
		// TODO 0x0ea => syscall!(tgkill, frame),
		// TODO 0x0eb => syscall!(utimes, frame),
		// TODO 0x0ec => syscall!(vserve, frame),
		// TODO 0x0ed => syscall!(mbind, frame),
		// TODO 0x0ee => syscall!(set_mempolicy, frame),
		// TODO 0x0ef => syscall!(get_mempolicy, frame),
		// TODO 0x0f0 => syscall!(mq_open, frame),
		// TODO 0x0f1 => syscall!(mq_unlink, frame),
		// TODO 0x0f2 => syscall!(mq_timedsend, frame),
		// TODO 0x0f3 => syscall!(mq_timedreceive, frame),
		// TODO 0x0f4 => syscall!(mq_notify, frame),
		// TODO 0x0f5 => syscall!(mq_getsetattr, frame),
		// TODO 0x0f6 => syscall!(kexec_load, frame),
		// TODO 0x0f7 => syscall!(waitid, frame),
		// TODO 0x0f8 => syscall!(add_key, frame),
		// TODO 0x0f9 => syscall!(request_key, frame),
		// TODO 0x0fa => syscall!(keyctl, frame),
		// TODO 0x0fb => syscall!(ioprio_set, frame),
		// TODO 0x0fc => syscall!(ioprio_get, frame),
		// TODO 0x0fd => syscall!(inotify_init, frame),
		// TODO 0x0fe => syscall!(inotify_add_watch, frame),
		// TODO 0x0ff => syscall!(inotify_rm_watch, frame),
		// TODO 0x100 => syscall!(migrate_pages, frame),
		0x101 => syscall!(openat, frame),
		// TODO 0x102 => syscall!(mkdirat, frame),
		// TODO 0x103 => syscall!(mknodat, frame),
		// TODO 0x104 => syscall!(fchownat, frame),
		// TODO 0x105 => syscall!(futimesat, frame),
		// TODO 0x106 => syscall!(newfstatat, frame),
		0x107 => syscall!(unlinkat, frame),
		// TODO 0x108 => syscall!(renameat, frame),
		0x109 => syscall!(linkat, frame),
		0x10a => syscall!(symlinkat, frame),
		// TODO 0x10b => syscall!(readlinkat, frame),
		0x10c => syscall!(fchmodat, frame),
		0x10d => syscall!(faccessat, frame),
		0x10e => syscall!(pselect6, frame),
		// TODO 0x10f => syscall!(ppoll, frame),
		// TODO 0x110 => syscall!(unshare, frame),
		// TODO 0x111 => syscall!(set_robust_list, frame),
		// TODO 0x112 => syscall!(get_robust_list, frame),
		// TODO 0x113 => syscall!(splice, frame),
		// TODO 0x114 => syscall!(tee, frame),
		// TODO 0x115 => syscall!(sync_file_range, frame),
		// TODO 0x116 => syscall!(vmsplice, frame),
		// TODO 0x117 => syscall!(move_pages, frame),
		0x118 => syscall!(utimensat, frame),
		// TODO 0x119 => syscall!(epoll_pwait, frame),
		// TODO 0x11a => syscall!(signalfd, frame),
		// TODO 0x11b => syscall!(timerfd_create, frame),
		// TODO 0x11c => syscall!(eventfd, frame),
		// TODO 0x11d => syscall!(fallocate, frame),
		// TODO 0x11e => syscall!(timerfd_settime, frame),
		// TODO 0x11f => syscall!(timerfd_gettime, frame),
		// TODO 0x120 => syscall!(accept4, frame),
		// TODO 0x121 => syscall!(signalfd4, frame),
		// TODO 0x122 => syscall!(eventfd2, frame),
		// TODO 0x123 => syscall!(epoll_create1, frame),
		// TODO 0x124 => syscall!(dup3, frame),
		0x125 => syscall!(pipe2, frame),
		// TODO 0x126 => syscall!(inotify_init1, frame),
		0x127 => syscall!(preadv, frame),
		0x128 => syscall!(pwritev, frame),
		// TODO 0x129 => syscall!(rt_tgsigqueueinfo, frame),
		// TODO 0x12a => syscall!(perf_event_open, frame),
		// TODO 0x12b => syscall!(recvmmsg, frame),
		// TODO 0x12c => syscall!(fanotify_init, frame),
		// TODO 0x12d => syscall!(fanotify_mark, frame),
		0x12e => syscall!(prlimit64, frame),
		// TODO 0x12f => syscall!(name_to_handle_at, frame),
		// TODO 0x130 => syscall!(open_by_handle_at, frame),
		// TODO 0x131 => syscall!(clock_adjtime, frame),
		0x132 => syscall!(syncfs, frame),
		// TODO 0x133 => syscall!(sendmmsg, frame),
		// TODO 0x134 => syscall!(setns, frame),
		// TODO 0x135 => syscall!(getcpu, frame),
		// TODO 0x136 => syscall!(process_vm_readv, frame),
		// TODO 0x137 => syscall!(process_vm_writev, frame),
		// TODO 0x138 => syscall!(kcmp, frame),
		0x139 => syscall!(finit_module, frame),
		// TODO 0x13a => syscall!(sched_setattr, frame),
		// TODO 0x13b => syscall!(sched_getattr, frame),
		0x13c => syscall!(renameat2, frame),
		// TODO 0x13d => syscall!(seccomp, frame),
		0x13e => syscall!(getrandom, frame),
		// TODO 0x13f => syscall!(memfd_create, frame),
		// TODO 0x140 => syscall!(kexec_file_load, frame),
		// TODO 0x141 => syscall!(bpf, frame),
		// TODO 0x142 => syscall!(execveat, frame),
		// TODO 0x143 => syscall!(userfaultfd, frame),
		// TODO 0x144 => syscall!(membarrier, frame),
		// TODO 0x145 => syscall!(mlock2, frame),
		// TODO 0x146 => syscall!(copy_file_range, frame),
		0x147 => syscall!(preadv2, frame),
		0x148 => syscall!(pwritev2, frame),
		// TODO 0x149 => syscall!(pkey_mprotect, frame),
		// TODO 0x14a => syscall!(pkey_alloc, frame),
		// TODO 0x14b => syscall!(pkey_free, frame),
		0x14c => syscall!(statx, frame),
		// TODO 0x14d => syscall!(io_pgetevents, frame),
		// TODO 0x14e => syscall!(rseq, frame),
		// TODO 0x1a8 => syscall!(pidfd_send_signal, frame),
		// TODO 0x1a9 => syscall!(io_uring_setup, frame),
		// TODO 0x1aa => syscall!(io_uring_enter, frame),
		// TODO 0x1ab => syscall!(io_uring_register, frame),
		// TODO 0x1ac => syscall!(open_tree, frame),
		// TODO 0x1ad => syscall!(move_mount, frame),
		// TODO 0x1ae => syscall!(fsopen, frame),
		// TODO 0x1af => syscall!(fsconfig, frame),
		// TODO 0x1b0 => syscall!(fsmount, frame),
		// TODO 0x1b1 => syscall!(fspick, frame),
		// TODO 0x1b2 => syscall!(pidfd_open, frame),
		// TODO 0x1b3 => syscall!(clone3, frame),
		// TODO 0x1b4 => syscall!(close_range, frame),
		// TODO 0x1b5 => syscall!(openat2, frame),
		// TODO 0x1b6 => syscall!(pidfd_getfd, frame),
		0x1b7 => syscall!(faccessat2, frame),
		// TODO 0x1b8 => syscall!(process_madvise, frame),
		// TODO 0x1b9 => syscall!(epoll_pwait2, frame),
		// TODO 0x1ba => syscall!(mount_setattr, frame),
		// TODO 0x1bb => syscall!(quotactl_fd, frame),
		// TODO 0x1bc => syscall!(landlock_create_ruleset, frame),
		// TODO 0x1bd => syscall!(landlock_add_rule, frame),
		// TODO 0x1be => syscall!(landlock_restrict_self, frame),
		// TODO 0x1bf => syscall!(memfd_secret, frame),
		// TODO 0x1c0 => syscall!(process_mrelease, frame),
		// TODO 0x1c1 => syscall!(futex_waitv, frame),
		// TODO 0x1c2 => syscall!(set_mempolicy_home_node, frame),
		// TODO 0x1c3 => syscall!(cachestat, frame),
		// TODO 0x1c4 => syscall!(fchmodat2, frame),
		// TODO 0x1c5 => syscall!(map_shadow_stack, frame),
		// TODO 0x1c6 => syscall!(futex_wake, frame),
		// TODO 0x1c7 => syscall!(futex_wait, frame),
		// TODO 0x1c8 => syscall!(futex_requeue, frame),
		_ => None,
	}
}

/// Called whenever a system call is triggered.
#[unsafe(no_mangle)]
pub extern "C" fn syscall_handler(frame: &mut IntFrame) {
	let id = frame.get_syscall_id();
	#[cfg(target_arch = "x86")]
	let res = do_syscall32(id, frame);
	#[cfg(target_arch = "x86_64")]
	let res = if frame.is_compat() {
		do_syscall32(id, frame)
	} else {
		do_syscall64(id, frame)
	};
	match res {
		// Success: Set the return value
		Some(res) => frame.set_syscall_return(res),
		// The system call does not exist: Kill the process with SIGSYS
		None => {
			let proc = Process::current();
			#[cfg(feature = "strace")]
			crate::println!(
				"[strace PID: {pid}] invalid syscall (ID: 0x{id:x})",
				pid = proc.get_pid()
			);
			// SIGSYS cannot be caught, thus the process will be terminated
			proc.kill(Signal::SIGSYS);
		}
	}
	// If the process has been killed, handle it
	process::yield_current(3, frame);
}

unsafe extern "C" {
	/// The syscall interrupt handler.
	pub fn syscall_int();
	/// Trampoline for the `syscall` instruction.
	pub fn syscall();
}

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
mod geteuid;
mod getgid;
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
mod getuid;
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
mod setgid;
mod sethostname;
mod setpgid;
mod setregid;
mod setresgid;
mod setresuid;
mod setreuid;
mod setsockopt;
mod setuid;
mod shutdown;
mod signal;
mod sigreturn;
mod socket;
mod socketpair;
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
use crate::{
	file,
	file::{fd::FileDescriptorTable, perm::AccessProfile, vfs::ResolutionSettings},
	process,
	process::{mem_space::MemSpace, regs::Regs, signal::Signal, Process},
};
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
use chroot::chroot;
use clock_gettime::clock_gettime;
use clock_gettime64::clock_gettime64;
use clone::clone;
use close::close;
use connect::connect;
use core::fmt;
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
use geteuid::geteuid;
use getgid::getgid;
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
use getuid::getuid;
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
use sethostname::sethostname;
use setpgid::setpgid;
use setregid::setregid;
use setresgid::setresgid;
use setresuid::setresuid;
use setreuid::setreuid;
use setsockopt::setsockopt;
use setuid::setuid;
use shutdown::shutdown;
use signal::signal;
use sigreturn::sigreturn;
use socket::socket;
use socketpair::socketpair;
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
use utils::{
	errno::EResult,
	lock::{IntMutex, Mutex},
	ptr::arc::Arc,
};
use utimensat::utimensat;
use vfork::vfork;
use wait4::wait4;
use waitpid::waitpid;
use write::write;
use writev::writev;

/// The ID of the `sigreturn` system call, for use by the signal trampoline.
pub const SIGRETURN_ID: usize = 0x077;

/// A system call handler.
pub trait SyscallHandler<'p, Args> {
	/// Calls the system call.
	///
	/// Arguments:
	/// - `name` is the name of the system call.
	/// - `regs` is the register state of the process at the moment of the system call.
	///
	/// The function returns the result of the system call.
	fn call(self, name: &str, regs: &'p Regs) -> EResult<usize>;
}

/// Implementation of [`SyscallHandler`] for functions with arguments.
macro_rules! impl_syscall_handler {
    ($($ty:ident),*) => {
        impl<'p, F, $($ty,)*> SyscallHandler<'p, ($($ty,)*)> for F
        where F: FnOnce($($ty,)*) -> EResult<usize>,
			$($ty: FromSyscall<'p>,)*
        {
			#[allow(non_snake_case, unused_variables)]
            fn call(self, name: &str, regs: &'p Regs) -> EResult<usize> {
				#[cfg(feature = "strace")]
				let pid = {
					let pid = Process::current().lock().get_pid();
					print!("[strace {pid}] {name}");
					pid
				};
                $(
                    let $ty = $ty::from_syscall(regs);
                )*
                let res = self($($ty,)*);
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
pub trait FromSyscall<'p> {
	/// Constructs the value from the given process or syscall argument value.
	fn from_syscall(regs: &'p Regs) -> Self;
}

impl<'p> FromSyscall<'p> for Arc<IntMutex<Process>> {
	#[inline]
	fn from_syscall(_regs: &'p Regs) -> Self {
		Process::current()
	}
}

impl FromSyscall<'_> for Arc<IntMutex<MemSpace>> {
	#[inline]
	fn from_syscall(_regs: &Regs) -> Self {
		Process::current().lock().get_mem_space().unwrap().clone()
	}
}

impl FromSyscall<'_> for Arc<Mutex<FileDescriptorTable>> {
	#[inline]
	fn from_syscall(_regs: &Regs) -> Self {
		Process::current().lock().file_descriptors.clone().unwrap()
	}
}

impl FromSyscall<'_> for AccessProfile {
	fn from_syscall(_regs: &Regs) -> Self {
		Process::current().lock().access_profile
	}
}

impl FromSyscall<'_> for ResolutionSettings {
	fn from_syscall(_regs: &Regs) -> Self {
		ResolutionSettings::for_process(&Process::current().lock(), true)
	}
}

/// The umask of the process performing the system call.
pub struct Umask(file::Mode);

impl FromSyscall<'_> for Umask {
	fn from_syscall(_regs: &Regs) -> Self {
		Self(Process::current().lock().umask)
	}
}

impl<'p> FromSyscall<'p> for &'p Regs {
	#[inline]
	fn from_syscall(regs: &'p Regs) -> Self {
		regs
	}
}

/// System call arguments.
#[derive(Debug)]
pub struct Args<T: fmt::Debug>(pub T);

impl<T: FromSyscallArg> FromSyscall<'_> for Args<T> {
	fn from_syscall(regs: &Regs) -> Self {
		let arg = T::from_syscall_arg(regs.get_syscall_arg(0));
		#[cfg(feature = "strace")]
		println!("({arg:?})");
		Self(arg)
	}
}

macro_rules! impl_from_syscall_args {
    ($($ty:ident),*) => {
		impl<$($ty: FromSyscallArg,)*> FromSyscall<'_> for Args<($($ty,)*)> {
			#[inline]
			#[allow(non_snake_case, unused_variables, unused_mut, unused_assignments)]
			fn from_syscall(
				regs: &Regs,
			) -> Self {
				let mut cursor = 0;
                $(
                    let $ty = $ty::from_syscall_arg(regs.get_syscall_arg(cursor));
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
pub trait FromSyscallArg: fmt::Debug {
	/// Constructs a value from the given system call argument value.
	fn from_syscall_arg(val: usize) -> Self;
}

macro_rules! impl_from_syscall_arg_primitive {
	($type:ident) => {
		impl FromSyscallArg for $type {
			fn from_syscall_arg(val: usize) -> Self {
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
	fn from_syscall_arg(val: usize) -> Self {
		val as _
	}
}

impl<T> FromSyscallArg for *mut T {
	fn from_syscall_arg(val: usize) -> Self {
		val as _
	}
}

/// Syscall declaration.
macro_rules! syscall {
	($name:ident, $regs:expr) => {{
		const NAME: &str = stringify!($name);
		SyscallHandler::call($name, NAME, $regs)
	}};
}

/// Executes the system call associated with the given `id` and returns its result.
///
/// If the syscall doesn't exist, the function returns `None`.
#[inline]
fn do_syscall(id: usize, regs: &Regs) -> Option<EResult<usize>> {
	match id {
		0x001 => Some(syscall!(_exit, regs)),
		0x002 => Some(syscall!(fork, regs)),
		0x003 => Some(syscall!(read, regs)),
		0x004 => Some(syscall!(write, regs)),
		0x005 => Some(syscall!(open, regs)),
		0x006 => Some(syscall!(close, regs)),
		0x007 => Some(syscall!(waitpid, regs)),
		0x008 => Some(syscall!(creat, regs)),
		0x009 => Some(syscall!(link, regs)),
		0x00a => Some(syscall!(unlink, regs)),
		0x00b => Some(syscall!(execve, regs)),
		0x00c => Some(syscall!(chdir, regs)),
		0x00d => Some(syscall!(time, regs)),
		0x00e => Some(syscall!(mknod, regs)),
		0x00f => Some(syscall!(chmod, regs)),
		0x010 => Some(syscall!(lchown, regs)),
		0x011 => Some(syscall!(r#break, regs)),
		// TODO 0x012 => Some(syscall!(oldstat, regs)),
		// TODO 0x013 => Some(syscall!(lseek, regs)),
		0x014 => Some(syscall!(getpid, regs)),
		0x015 => Some(syscall!(mount, regs)),
		0x016 => Some(syscall!(umount, regs)),
		0x017 => Some(syscall!(setuid, regs)),
		0x018 => Some(syscall!(getuid, regs)),
		// TODO 0x019 => Some(syscall!(stime, regs)),
		// TODO 0x01a => Some(syscall!(ptrace, regs)),
		// TODO 0x01b => Some(syscall!(alarm, regs)),
		// TODO 0x01c => Some(syscall!(oldfstat, regs)),
		// TODO 0x01d => Some(syscall!(pause, regs)),
		// TODO 0x01e => Some(syscall!(utime, regs)),
		// TODO 0x01f => Some(syscall!(stty, regs)),
		// TODO 0x020 => Some(syscall!(gtty, regs)),
		0x021 => Some(syscall!(access, regs)),
		// TODO 0x022 => Some(syscall!(nice, regs)),
		// TODO 0x023 => Some(syscall!(ftime, regs)),
		// TODO 0x024 => Some(syscall!(sync, regs)),
		0x025 => Some(syscall!(kill, regs)),
		0x026 => Some(syscall!(rename, regs)),
		0x027 => Some(syscall!(mkdir, regs)),
		0x028 => Some(syscall!(rmdir, regs)),
		0x029 => Some(syscall!(dup, regs)),
		0x02a => Some(syscall!(pipe, regs)),
		// TODO 0x02b => Some(syscall!(times, regs)),
		// TODO 0x02c => Some(syscall!(prof, regs)),
		0x02d => Some(syscall!(brk, regs)),
		0x02e => Some(syscall!(setgid, regs)),
		0x02f => Some(syscall!(getgid, regs)),
		0x030 => Some(syscall!(signal, regs)),
		0x031 => Some(syscall!(geteuid, regs)),
		0x032 => Some(syscall!(getegid, regs)),
		// TODO 0x033 => Some(syscall!(acct, regs)),
		// TODO 0x034 => Some(syscall!(umount2, regs)),
		// TODO 0x035 => Some(syscall!(lock, regs)),
		0x036 => Some(syscall!(ioctl, regs)),
		0x037 => Some(syscall!(fcntl, regs)),
		// TODO 0x038 => Some(syscall!(mpx, regs)),
		0x039 => Some(syscall!(setpgid, regs)),
		// TODO 0x03a => Some(syscall!(ulimit, regs)),
		// TODO 0x03b => Some(syscall!(oldolduname, regs)),
		0x03c => Some(syscall!(umask, regs)),
		0x03d => Some(syscall!(chroot, regs)),
		// TODO 0x03e => Some(syscall!(ustat, regs)),
		0x03f => Some(syscall!(dup2, regs)),
		0x040 => Some(syscall!(getppid, regs)),
		// TODO 0x041 => Some(syscall!(getpgrp, regs)),
		// TODO 0x042 => Some(syscall!(setsid, regs)),
		// TODO 0x043 => Some(syscall!(sigaction, regs)),
		// TODO 0x044 => Some(syscall!(sgetmask, regs)),
		// TODO 0x045 => Some(syscall!(ssetmask, regs)),
		0x046 => Some(syscall!(setreuid, regs)),
		0x047 => Some(syscall!(setregid, regs)),
		// TODO 0x048 => Some(syscall!(sigsuspend, regs)),
		// TODO 0x049 => Some(syscall!(sigpending, regs)),
		0x04a => Some(syscall!(sethostname, regs)),
		// TODO 0x04b => Some(syscall!(setrlimit, regs)),
		// TODO 0x04c => Some(syscall!(getrlimit, regs)),
		0x04d => Some(syscall!(getrusage, regs)),
		// TODO 0x04e => Some(syscall!(gettimeofday, regs)),
		// TODO 0x04f => Some(syscall!(settimeofday, regs)),
		// TODO 0x050 => Some(syscall!(getgroups, regs)),
		// TODO 0x051 => Some(syscall!(setgroups, regs)),
		0x052 => Some(syscall!(select, regs)),
		0x053 => Some(syscall!(symlink, regs)),
		// TODO 0x054 => Some(syscall!(oldlstat, regs)),
		0x055 => Some(syscall!(readlink, regs)),
		// TODO 0x056 => Some(syscall!(uselib, regs)),
		// TODO 0x057 => Some(syscall!(swapon, regs)),
		0x058 => Some(syscall!(reboot, regs)),
		// TODO 0x059 => Some(syscall!(readdir, regs)),
		0x05a => Some(syscall!(mmap, regs)),
		0x05b => Some(syscall!(munmap, regs)),
		0x05c => Some(syscall!(truncate, regs)),
		// TODO 0x05d => Some(syscall!(ftruncate, regs)),
		0x05e => Some(syscall!(fchmod, regs)),
		// TODO 0x05f => Some(syscall!(fchown, regs)),
		// TODO 0x060 => Some(syscall!(getpriority, regs)),
		// TODO 0x061 => Some(syscall!(setpriority, regs)),
		// TODO 0x062 => Some(syscall!(profil, regs)),
		0x063 => Some(syscall!(statfs, regs)),
		0x064 => Some(syscall!(fstatfs, regs)),
		// TODO 0x065 => Some(syscall!(ioperm, regs)),
		// TODO 0x066 => Some(syscall!(socketcall, regs)),
		// TODO 0x067 => Some(syscall!(syslog, regs)),
		// TODO 0x068 => Some(syscall!(setitimer, regs)),
		// TODO 0x069 => Some(syscall!(getitimer, regs)),
		// TODO 0x06a => Some(syscall!(stat, regs)),
		// TODO 0x06b => Some(syscall!(lstat, regs)),
		// TODO 0x06c => Some(syscall!(fstat, regs)),
		// TODO 0x06d => Some(syscall!(olduname, regs)),
		// TODO 0x06e => Some(syscall!(iopl, regs)),
		// TODO 0x06f => Some(syscall!(vhangup, regs)),
		// TODO 0x070 => Some(syscall!(idle, regs)),
		// TODO 0x071 => Some(syscall!(vm86old, regs)),
		0x072 => Some(syscall!(wait4, regs)),
		// TODO 0x073 => Some(syscall!(swapoff, regs)),
		// TODO 0x074 => Some(syscall!(sysinfo, regs)),
		// TODO 0x075 => Some(syscall!(ipc, regs)),
		0x076 => Some(syscall!(fsync, regs)),
		SIGRETURN_ID => Some(syscall!(sigreturn, regs)),
		0x078 => Some(syscall!(clone, regs)),
		// TODO 0x079 => Some(syscall!(setdomainname, regs)),
		0x07a => Some(syscall!(uname, regs)),
		// TODO 0x07c => Some(syscall!(adjtimex, regs)),
		0x07d => Some(syscall!(mprotect, regs)),
		// TODO 0x07e => Some(syscall!(sigprocmask, regs)),
		// TODO 0x07f => Some(syscall!(create_module, regs)),
		0x080 => Some(syscall!(init_module, regs)),
		0x081 => Some(syscall!(delete_module, regs)),
		// TODO 0x083 => Some(syscall!(quotactl, regs)),
		0x084 => Some(syscall!(getpgid, regs)),
		0x085 => Some(syscall!(fchdir, regs)),
		// TODO 0x086 => Some(syscall!(bdflush, regs)),
		// TODO 0x087 => Some(syscall!(sysfs, regs)),
		// TODO 0x088 => Some(syscall!(personality, regs)),
		// TODO 0x089 => Some(syscall!(afs_syscall, regs)),
		// TODO 0x08a => Some(syscall!(setfsuid, regs)),
		// TODO 0x08b => Some(syscall!(setfsgid, regs)),
		0x08c => Some(syscall!(_llseek, regs)),
		0x08d => Some(syscall!(getdents, regs)),
		0x08e => Some(syscall!(_newselect, regs)),
		// TODO 0x08f => Some(syscall!(flock, regs)),
		0x090 => Some(syscall!(msync, regs)),
		0x091 => Some(syscall!(readv, regs)),
		0x092 => Some(syscall!(writev, regs)),
		// TODO 0x093 => Some(syscall!(getsid, regs)),
		// TODO 0x094 => Some(syscall!(fdatasync, regs)),
		// TODO 0x095 => Some(syscall!(_sysctl, regs)),
		// TODO 0x096 => Some(syscall!(mlock, regs)),
		// TODO 0x097 => Some(syscall!(munlock, regs)),
		// TODO 0x098 => Some(syscall!(mlockall, regs)),
		// TODO 0x099 => Some(syscall!(munlockall, regs)),
		// TODO 0x09a => Some(syscall!(sched_setparam, regs)),
		// TODO 0x09b => Some(syscall!(sched_getparam, regs)),
		// TODO 0x09c => Some(syscall!(sched_setscheduler, regs)),
		// TODO 0x09d => Some(syscall!(sched_getscheduler, regs)),
		0x09e => Some(syscall!(sched_yield, regs)),
		// TODO 0x09f => Some(syscall!(sched_get_priority_max, regs)),
		// TODO 0x0a0 => Some(syscall!(sched_get_priority_min, regs)),
		// TODO 0x0a1 => Some(syscall!(sched_rr_get_interval, regs)),
		0x0a2 => Some(syscall!(nanosleep, regs)),
		// TODO 0x0a3 => Some(syscall!(mremap, regs)),
		0x0a4 => Some(syscall!(setresuid, regs)),
		0x0a5 => Some(syscall!(getresuid, regs)),
		// TODO 0x0a6 => Some(syscall!(vm86, regs)),
		// TODO 0x0a7 => Some(syscall!(query_module, regs)),
		0x0a8 => Some(syscall!(poll, regs)),
		// TODO 0x0a9 => Some(syscall!(nfsservctl, regs)),
		0x0aa => Some(syscall!(setresgid, regs)),
		0x0ab => Some(syscall!(getresgid, regs)),
		// TODO 0x0ac => Some(syscall!(prctl, regs)),
		// TODO 0x0ad => Some(syscall!(rt_sigreturn, regs)),
		0x0ae => Some(syscall!(rt_sigaction, regs)),
		0x0af => Some(syscall!(rt_sigprocmask, regs)),
		// TODO 0x0b0 => Some(syscall!(rt_sigpending, regs)),
		// TODO 0x0b1 => Some(syscall!(rt_sigtimedwait, regs)),
		// TODO 0x0b2 => Some(syscall!(rt_sigqueueinfo, regs)),
		// TODO 0x0b3 => Some(syscall!(rt_sigsuspend, regs)),
		// TODO 0x0b4 => Some(syscall!(pread64, regs)),
		// TODO 0x0b5 => Some(syscall!(pwrite64, regs)),
		0x0b6 => Some(syscall!(chown, regs)),
		0x0b7 => Some(syscall!(getcwd, regs)),
		// TODO 0x0b8 => Some(syscall!(capget, regs)),
		// TODO 0x0b9 => Some(syscall!(capset, regs)),
		// TODO 0x0ba => Some(syscall!(sigaltstack, regs)),
		// TODO 0x0bb => Some(syscall!(sendfile, regs)),
		// TODO 0x0bc => Some(syscall!(getpmsg, regs)),
		// TODO 0x0bd => Some(syscall!(putpmsg, regs)),
		0x0be => Some(syscall!(vfork, regs)),
		// TODO 0x0bf => Some(syscall!(ugetrlimit, regs)),
		0x0c0 => Some(syscall!(mmap2, regs)),
		// TODO 0x0c1 => Some(syscall!(truncate64, regs)),
		// TODO 0x0c2 => Some(syscall!(ftruncate64, regs)),
		// TODO 0x0c3 => Some(syscall!(stat64, regs)),
		// TODO 0x0c4 => Some(syscall!(lstat64, regs)),
		0x0c5 => Some(syscall!(fstat64, regs)),
		// TODO 0x0c6 => Some(syscall!(lchown32, regs)),
		0x0c7 => Some(syscall!(getuid, regs)),   // getuid32
		0x0c8 => Some(syscall!(getgid, regs)),   // getgid32
		0x0c9 => Some(syscall!(geteuid, regs)),  // geteuid32
		0x0ca => Some(syscall!(getegid, regs)),  // getegid32
		0x0cb => Some(syscall!(setreuid, regs)), // setreuid32
		0x0cc => Some(syscall!(setregid, regs)), // setregid32
		// TODO 0x0cd => Some(syscall!(getgroups32, regs)),
		// TODO 0x0ce => Some(syscall!(setgroups32, regs)),
		// TODO 0x0cf => Some(syscall!(fchown32, regs)),
		0x0d0 => Some(syscall!(setresuid, regs)), // setresuid32
		0x0d1 => Some(syscall!(getresuid, regs)), // getresuid32
		0x0d2 => Some(syscall!(setresgid, regs)), // setresgid32
		0x0d3 => Some(syscall!(getresgid, regs)), // getresgid32
		0x0d4 => Some(syscall!(chown, regs)),     // chown32
		0x0d5 => Some(syscall!(setuid, regs)),    // setuid32
		0x0d6 => Some(syscall!(setgid, regs)),    // setgid32
		// TODO 0x0d7 => Some(syscall!(setfsuid32, regs)),
		// TODO 0x0d8 => Some(syscall!(setfsgid32, regs)),
		// TODO 0x0d9 => Some(syscall!(pivot_root, regs)),
		// TODO 0x0da => Some(syscall!(mincore, regs)),
		0x0db => Some(syscall!(madvise, regs)),
		0x0dc => Some(syscall!(getdents64, regs)),
		0x0dd => Some(syscall!(fcntl64, regs)),
		0x0e0 => Some(syscall!(gettid, regs)),
		// TODO 0x0e1 => Some(syscall!(readahead, regs)),
		// TODO 0x0e2 => Some(syscall!(setxattr, regs)),
		// TODO 0x0e3 => Some(syscall!(lsetxattr, regs)),
		// TODO 0x0e4 => Some(syscall!(fsetxattr, regs)),
		// TODO 0x0e5 => Some(syscall!(getxattr, regs)),
		// TODO 0x0e6 => Some(syscall!(lgetxattr, regs)),
		// TODO 0x0e7 => Some(syscall!(fgetxattr, regs)),
		// TODO 0x0e8 => Some(syscall!(listxattr, regs)),
		// TODO 0x0e9 => Some(syscall!(llistxattr, regs)),
		// TODO 0x0ea => Some(syscall!(flistxattr, regs)),
		// TODO 0x0eb => Some(syscall!(removexattr, regs)),
		// TODO 0x0ec => Some(syscall!(lremovexattr, regs)),
		// TODO 0x0ed => Some(syscall!(fremovexattr, regs)),
		0x0ee => Some(syscall!(tkill, regs)),
		// TODO 0x0ef => Some(syscall!(sendfile64, regs)),
		// TODO 0x0f0 => Some(syscall!(futex, regs)),
		// TODO 0x0f1 => Some(syscall!(sched_setaffinity, regs)),
		// TODO 0x0f2 => Some(syscall!(sched_getaffinity, regs)),
		0x0f3 => Some(syscall!(set_thread_area, regs)),
		// TODO 0x0f4 => Some(syscall!(get_thread_area, regs)),
		// TODO 0x0f5 => Some(syscall!(io_setup, regs)),
		// TODO 0x0f6 => Some(syscall!(io_destroy, regs)),
		// TODO 0x0f7 => Some(syscall!(io_getevents, regs)),
		// TODO 0x0f8 => Some(syscall!(io_submit, regs)),
		// TODO 0x0f9 => Some(syscall!(io_cancel, regs)),
		// TODO 0x0fa => Some(syscall!(fadvise64, regs)),
		0x0fc => Some(syscall!(exit_group, regs)),
		// TODO 0x0fd => Some(syscall!(lookup_dcookie, regs)),
		// TODO 0x0fe => Some(syscall!(epoll_create, regs)),
		// TODO 0x0ff => Some(syscall!(epoll_ctl, regs)),
		// TODO 0x100 => Some(syscall!(epoll_wait, regs)),
		// TODO 0x101 => Some(syscall!(remap_file_pages, regs)),
		0x102 => Some(syscall!(set_tid_address, regs)),
		0x103 => Some(syscall!(timer_create, regs)),
		0x104 => Some(syscall!(timer_settime, regs)),
		// TODO 0x105 => Some(syscall!(timer_gettime, regs)),
		// TODO 0x106 => Some(syscall!(timer_getoverrun, regs)),
		0x107 => Some(syscall!(timer_delete, regs)),
		// TODO 0x108 => Some(syscall!(clock_settime, regs)),
		0x109 => Some(syscall!(clock_gettime, regs)),
		// TODO 0x10a => Some(syscall!(clock_getres, regs)),
		// TODO 0x10b => Some(syscall!(clock_nanosleep, regs)),
		0x10c => Some(syscall!(statfs64, regs)),
		0x10d => Some(syscall!(fstatfs64, regs)),
		// TODO 0x10e => Some(syscall!(tgkill, regs)),
		// TODO 0x10f => Some(syscall!(utimes, regs)),
		0x110 => Some(syscall!(fadvise64_64, regs)),
		// TODO 0x111 => Some(syscall!(vserver, regs)),
		// TODO 0x112 => Some(syscall!(mbind, regs)),
		// TODO 0x113 => Some(syscall!(get_mempolicy, regs)),
		// TODO 0x114 => Some(syscall!(set_mempolicy, regs)),
		// TODO 0x115 => Some(syscall!(mq_open, regs)),
		// TODO 0x116 => Some(syscall!(mq_unlink, regs)),
		// TODO 0x117 => Some(syscall!(mq_timedsend, regs)),
		// TODO 0x118 => Some(syscall!(mq_timedreceive, regs)),
		// TODO 0x119 => Some(syscall!(mq_notify, regs)),
		// TODO 0x11a => Some(syscall!(mq_getsetattr, regs)),
		// TODO 0x11b => Some(syscall!(kexec_load, regs)),
		// TODO 0x11c => Some(syscall!(waitid, regs)),
		// TODO 0x11e => Some(syscall!(add_key, regs)),
		// TODO 0x11f => Some(syscall!(request_key, regs)),
		// TODO 0x120 => Some(syscall!(keyctl, regs)),
		// TODO 0x121 => Some(syscall!(ioprio_set, regs)),
		// TODO 0x122 => Some(syscall!(ioprio_get, regs)),
		// TODO 0x123 => Some(syscall!(inotify_init, regs)),
		// TODO 0x124 => Some(syscall!(inotify_add_watch, regs)),
		// TODO 0x125 => Some(syscall!(inotify_rm_watch, regs)),
		// TODO 0x126 => Some(syscall!(migrate_pages, regs)),
		0x127 => Some(syscall!(openat, regs)),
		// TODO 0x128 => Some(syscall!(mkdirat, regs)),
		// TODO 0x129 => Some(syscall!(mknodat, regs)),
		// TODO 0x12a => Some(syscall!(fchownat, regs)),
		// TODO 0x12b => Some(syscall!(futimesat, regs)),
		// TODO 0x12c => Some(syscall!(fstatat64, regs)),
		0x12d => Some(syscall!(unlinkat, regs)),
		// TODO 0x12e => Some(syscall!(renameat, regs)),
		0x12f => Some(syscall!(linkat, regs)),
		0x130 => Some(syscall!(symlinkat, regs)),
		// TODO 0x131 => Some(syscall!(readlinkat, regs)),
		0x132 => Some(syscall!(fchmodat, regs)),
		0x133 => Some(syscall!(faccessat, regs)),
		0x134 => Some(syscall!(pselect6, regs)),
		// TODO 0x135 => Some(syscall!(ppoll, regs)),
		// TODO 0x136 => Some(syscall!(unshare, regs)),
		// TODO 0x137 => Some(syscall!(set_robust_list, regs)),
		// TODO 0x138 => Some(syscall!(get_robust_list, regs)),
		// TODO 0x139 => Some(syscall!(splice, regs)),
		// TODO 0x13a => Some(syscall!(sync_file_range, regs)),
		// TODO 0x13b => Some(syscall!(tee, regs)),
		// TODO 0x13c => Some(syscall!(vmsplice, regs)),
		// TODO 0x13d => Some(syscall!(move_pages, regs)),
		// TODO 0x13e => Some(syscall!(getcpu, regs)),
		// TODO 0x13f => Some(syscall!(epoll_pwait, regs)),
		0x140 => Some(syscall!(utimensat, regs)),
		// TODO 0x141 => Some(syscall!(signalfd, regs)),
		// TODO 0x142 => Some(syscall!(timerfd_create, regs)),
		// TODO 0x143 => Some(syscall!(eventfd, regs)),
		// TODO 0x144 => Some(syscall!(fallocate, regs)),
		// TODO 0x145 => Some(syscall!(timerfd_settime, regs)),
		// TODO 0x146 => Some(syscall!(timerfd_gettime, regs)),
		// TODO 0x147 => Some(syscall!(signalfd4, regs)),
		// TODO 0x148 => Some(syscall!(eventfd2, regs)),
		// TODO 0x149 => Some(syscall!(epoll_create1, regs)),
		// TODO 0x14a => Some(syscall!(dup3, regs)),
		0x14b => Some(syscall!(pipe2, regs)),
		// TODO 0x14c => Some(syscall!(inotify_init1, regs)),
		0x14d => Some(syscall!(preadv, regs)),
		0x14e => Some(syscall!(pwritev, regs)),
		// TODO 0x14f => Some(syscall!(rt_tgsigqueueinfo, regs)),
		// TODO 0x150 => Some(syscall!(perf_event_open, regs)),
		// TODO 0x151 => Some(syscall!(recvmmsg, regs)),
		// TODO 0x152 => Some(syscall!(fanotify_init, regs)),
		// TODO 0x153 => Some(syscall!(fanotify_mark, regs)),
		0x154 => Some(syscall!(prlimit64, regs)),
		// TODO 0x155 => Some(syscall!(name_to_handle_at, regs)),
		// TODO 0x156 => Some(syscall!(open_by_handle_at, regs)),
		// TODO 0x157 => Some(syscall!(clock_adjtime, regs)),
		0x158 => Some(syscall!(syncfs, regs)),
		// TODO 0x159 => Some(syscall!(sendmmsg, regs)),
		// TODO 0x15a => Some(syscall!(setns, regs)),
		// TODO 0x15b => Some(syscall!(process_vm_readv, regs)),
		// TODO 0x15c => Some(syscall!(process_vm_writev, regs)),
		// TODO 0x15d => Some(syscall!(kcmp, regs)),
		0x15e => Some(syscall!(finit_module, regs)),
		// TODO 0x15f => Some(syscall!(sched_setattr, regs)),
		// TODO 0x160 => Some(syscall!(sched_getattr, regs)),
		0x161 => Some(syscall!(renameat2, regs)),
		// TODO 0x162 => Some(syscall!(seccomp, regs)),
		0x163 => Some(syscall!(getrandom, regs)),
		// TODO 0x164 => Some(syscall!(memfd_create, regs)),
		// TODO 0x165 => Some(syscall!(bpf, regs)),
		// TODO 0x166 => Some(syscall!(execveat, regs)),
		0x167 => Some(syscall!(socket, regs)),
		0x168 => Some(syscall!(socketpair, regs)),
		0x169 => Some(syscall!(bind, regs)),
		0x16a => Some(syscall!(connect, regs)),
		// TODO 0x16b => Some(syscall!(listen, regs)),
		// TODO 0x16c => Some(syscall!(accept4, regs)),
		0x16d => Some(syscall!(getsockopt, regs)),
		0x16e => Some(syscall!(setsockopt, regs)),
		0x16f => Some(syscall!(getsockname, regs)),
		// TODO 0x170 => Some(syscall!(getpeername, regs)),
		0x171 => Some(syscall!(sendto, regs)),
		// TODO 0x172 => Some(syscall!(sendmsg, regs)),
		// TODO 0x173 => Some(syscall!(recvfrom, regs)),
		// TODO 0x174 => Some(syscall!(recvmsg, regs)),
		0x175 => Some(syscall!(shutdown, regs)),
		// TODO 0x176 => Some(syscall!(userfaultfd, regs)),
		// TODO 0x177 => Some(syscall!(membarrier, regs)),
		// TODO 0x178 => Some(syscall!(mlock2, regs)),
		// TODO 0x179 => Some(syscall!(copy_file_range, regs)),
		0x17a => Some(syscall!(preadv2, regs)),
		0x17b => Some(syscall!(pwritev2, regs)),
		// TODO 0x17c => Some(syscall!(pkey_mprotect, regs)),
		// TODO 0x17d => Some(syscall!(pkey_alloc, regs)),
		// TODO 0x17e => Some(syscall!(pkey_free, regs)),
		0x17f => Some(syscall!(statx, regs)),
		0x180 => Some(syscall!(arch_prctl, regs)),
		// TODO 0x181 => Some(syscall!(io_pgetevents, regs)),
		// TODO 0x182 => Some(syscall!(rseq, regs)),
		// TODO 0x189 => Some(syscall!(semget, regs)),
		// TODO 0x18a => Some(syscall!(semctl, regs)),
		// TODO 0x18b => Some(syscall!(shmget, regs)),
		// TODO 0x18c => Some(syscall!(shmctl, regs)),
		// TODO 0x18d => Some(syscall!(shmat, regs)),
		// TODO 0x18e => Some(syscall!(shmdt, regs)),
		// TODO 0x18f => Some(syscall!(msgget, regs)),
		// TODO 0x190 => Some(syscall!(msgsnd, regs)),
		// TODO 0x191 => Some(syscall!(msgrcv, regs)),
		// TODO 0x192 => Some(syscall!(msgctl, regs)),
		0x193 => Some(syscall!(clock_gettime64, regs)),
		// TODO 0x194 => Some(syscall!(clock_settime64, regs)),
		// TODO 0x195 => Some(syscall!(clock_adjtime64, regs)),
		// TODO 0x196 => Some(syscall!(clock_getres_time64, regs)),
		// TODO 0x197 => Some(syscall!(clock_nanosleep_time64, regs)),
		// TODO 0x198 => Some(syscall!(timer_gettime64, regs)),
		// TODO 0x199 => Some(syscall!(timer_settime64, regs)),
		// TODO 0x19a => Some(syscall!(timerfd_gettime64, regs)),
		// TODO 0x19b => Some(syscall!(timerfd_settime64, regs)),
		// TODO 0x19c => Some(syscall!(utimensat_time64, regs)),
		// TODO 0x19d => Some(syscall!(pselect6_time64, regs)),
		// TODO 0x19e => Some(syscall!(ppoll_time64, regs)),
		// TODO 0x1a0 => Some(syscall!(io_pgetevents_time64, regs)),
		// TODO 0x1a1 => Some(syscall!(recvmmsg_time64, regs)),
		// TODO 0x1a2 => Some(syscall!(mq_timedsend_time64, regs)),
		// TODO 0x1a3 => Some(syscall!(mq_timedreceive_time64, regs)),
		// TODO 0x1a4 => Some(syscall!(semtimedop_time64, regs)),
		// TODO 0x1a5 => Some(syscall!(rt_sigtimedwait_time64, regs)),
		// TODO 0x1a6 => Some(syscall!(futex_time64, regs)),
		// TODO 0x1a7 => Some(syscall!(sched_rr_get_interval_time64, regs)),
		// TODO 0x1a8 => Some(syscall!(pidfd_send_signal, regs)),
		// TODO 0x1a9 => Some(syscall!(io_uring_setup, regs)),
		// TODO 0x1aa => Some(syscall!(io_uring_enter, regs)),
		// TODO 0x1ab => Some(syscall!(io_uring_register, regs)),
		// TODO 0x1ac => Some(syscall!(open_tree, regs)),
		// TODO 0x1ad => Some(syscall!(move_mount, regs)),
		// TODO 0x1ae => Some(syscall!(fsopen, regs)),
		// TODO 0x1af => Some(syscall!(fsconfig, regs)),
		// TODO 0x1b0 => Some(syscall!(fsmount, regs)),
		// TODO 0x1b1 => Some(syscall!(fspick, regs)),
		// TODO 0x1b2 => Some(syscall!(pidfd_open, regs)),
		// TODO 0x1b3 => Some(syscall!(clone3, regs)),
		// TODO 0x1b4 => Some(syscall!(close_range, regs)),
		// TODO 0x1b5 => Some(syscall!(openat2, regs)),
		// TODO 0x1b6 => Some(syscall!(pidfd_getfd, regs)),
		0x1b7 => Some(syscall!(faccessat2, regs)),
		// TODO 0x1b8 => Some(syscall!(process_madvise, regs)),
		// TODO 0x1b9 => Some(syscall!(epoll_pwait2, regs)),
		// TODO 0x1ba => Some(syscall!(mount_setattr, regs)),
		// TODO 0x1bb => Some(syscall!(quotactl_fd, regs)),
		// TODO 0x1bc => Some(syscall!(landlock_create_ruleset, regs)),
		// TODO 0x1bd => Some(syscall!(landlock_add_rule, regs)),
		// TODO 0x1be => Some(syscall!(landlock_restrict_self, regs)),
		// TODO 0x1bf => Some(syscall!(memfd_secret, regs)),
		// TODO 0x1c0 => Some(syscall!(process_mrelease, regs)),
		// TODO 0x1c1 => Some(syscall!(futex_waitv, regs)),
		// TODO 0x1c2 => Some(syscall!(set_mempolicy_home_node, regs)),
		_ => None,
	}
}

/// Called whenever a system call is triggered.
#[no_mangle]
pub extern "C" fn syscall_handler(regs: &mut Regs) {
	let id = regs.get_syscall_id();
	let Some(res) = do_syscall(id, regs) else {
		// The system call doesn't exist. Kill the process with SIGSYS
		{
			let proc_mutex = Process::current();
			let mut proc = proc_mutex.lock();
			#[cfg(feature = "strace")]
			crate::println!(
				"[strace PID: {pid}] invalid syscall (ID: 0x{id:x})",
				pid = proc.get_pid()
			);
			// SIGSYS cannot be caught, thus the process will be terminated
			proc.kill_now(Signal::SIGSYS);
		}
		crate::enter_loop();
	};
	regs.set_syscall_return(res);
	// If the process has been killed, handle it
	process::yield_current(3, regs);
}

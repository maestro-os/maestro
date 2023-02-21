//! The `utimensat` system call allows to change the timestamps of a file.

use core::ffi::c_int;
use crate::errno::Errno;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::mem_space::ptr::SyscallString;
use crate::time::unit::TimeUnit;
use crate::time::unit::Timespec;
use macros::syscall;
use super::util;

#[syscall]
pub fn utimensat(
	dirfd: c_int,
	pathname: SyscallString,
	times: SyscallPtr<[Timespec; 2]>,
	flags: c_int
) -> Result<i32, Errno> {
	let (file_mutex, atime, mtime) = {
		let proc_mutex = Process::get_current().unwrap();
		let proc_guard = proc_mutex.lock();
		let proc = proc_guard.get();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();
		let path = pathname.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;

		let file_mutex = util::get_file_at(proc_guard, true, dirfd, path, flags)?;

		let times_val = times.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;

		let atime = times_val[0];
		let mtime = times_val[1];

		(file_mutex, atime, mtime)
	};

	let file_guard = file_mutex.lock();
	let file = file_guard.get_mut();

	// TODO clean
	file.set_atime(atime.to_nano() / 1000000000);
	file.set_mtime(mtime.to_nano() / 1000000000);

	// TODO sync only when required
	file.sync()?;

	Ok(0)
}

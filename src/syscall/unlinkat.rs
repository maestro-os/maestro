//! The `unlinkat` syscall allows to unlink a file.
//!
//! If no link remain to the file, the function also removes it.

use super::util;
use crate::errno::Errno;
use crate::file::vfs;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;

#[syscall]
pub fn unlinkat(dirfd: c_int, pathname: SyscallString, flags: c_int) -> Result<i32, Errno> {
	let (file_mutex, ap) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let ap = proc.access_profile;

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();
		let pathname = pathname
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;

		let file = util::get_file_at(proc, false, dirfd, pathname, flags)?;

		(file, ap)
	};

	let mut file = file_mutex.lock();
	vfs::remove_file(&mut file, &ap)?;

	Ok(0)
}

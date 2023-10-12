//! The `fchmodat` system call allows change the permissions on a file.

use super::util;
use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;

// TODO Check args type
#[syscall]
pub fn fchmodat(
	dirfd: c_int,
	pathname: SyscallString,
	mode: i32,
	_flags: c_int,
) -> Result<i32, Errno> {
	let (file_mutex, ap) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let ap = proc.access_profile;

		let mem_space = proc.get_mem_space().unwrap().clone();
		let mem_space_guard = mem_space.lock();

		let pathname = pathname
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let file_mutex = util::get_file_at(proc, true, dirfd, pathname, 0)?;

		(file_mutex, ap)
	};
	let mut file = file_mutex.lock();

	// Check permissions
	if !ap.can_set_file_permissions(&*file) {
		return Err(errno!(EPERM));
	}

	file.set_permissions(mode as _);
	// TODO lazy sync
	file.sync()?;

	Ok(0)
}

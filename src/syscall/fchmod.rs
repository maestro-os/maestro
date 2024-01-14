//! The `fchmod` system call allows change the permissions on a file.

use crate::errno::Errno;
use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;

// TODO Check args type
#[syscall]
pub fn fchmod(fd: c_int, mode: i32) -> Result<i32, Errno> {
	if fd < 0 {
		return Err(errno!(EBADF));
	}

	let (file_mutex, ap) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let fds_mutex = proc.file_descriptors.as_ref().unwrap();
		let fds = fds_mutex.lock();
		let fd = fds.get_fd(fd as _).ok_or_else(|| errno!(EBADF))?;

		let open_file_mutex = fd.get_open_file();
		let open_file = open_file_mutex.lock();
		let file_mutex = open_file.get_file().clone();

		(file_mutex, proc.access_profile)
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
